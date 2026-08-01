use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use totp_rs::{Algorithm as TotpAlgorithm, Secret, TOTP};
use uuid::Uuid;

use crate::{
    config::AuthConfig,
    contracts::auth::{AuthStatusPayload, Claims, MessagePayload, SetupStartPayload, TokenPayload},
    error::{AppError, Result},
    repositories::AppSettingsRepo,
};

const TOTP_SECRET_KEY: &str = "totp_secret";
const SESSION_SECRET_KEY: &str = "session_secret";
const TOTP_ISSUER: &str = "QuantAura";
const TOTP_ACCOUNT: &str = "local";
const TOTP_STEP_SECS: u64 = 30;
const TOTP_DIGITS: usize = 6;
const PENDING_SETUP_TTL: Duration = Duration::from_secs(600);
const RATE_LIMIT_MAX_FAILURES: u32 = 5;
const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);
const RATE_LIMIT_LOCKOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
struct PendingSetup {
    secret_b32: String,
    created_at: Instant,
}

#[derive(Debug)]
struct RateLimiter {
    failures: u32,
    window_start: Instant,
    locked_until: Option<Instant>,
}

impl RateLimiter {
    fn new() -> Self {
        Self {
            failures: 0,
            window_start: Instant::now(),
            locked_until: None,
        }
    }

    fn check(&mut self) -> Result<()> {
        let now = Instant::now();
        if let Some(until) = self.locked_until {
            if now < until {
                return Err(AppError::Unauthorized(
                    "Too many attempts, please try again later".into(),
                ));
            }
            self.locked_until = None;
            self.failures = 0;
            self.window_start = now;
        }
        Ok(())
    }

    fn record_failure(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.window_start) > RATE_LIMIT_WINDOW {
            self.window_start = now;
            self.failures = 0;
        }
        self.failures += 1;
        if self.failures >= RATE_LIMIT_MAX_FAILURES {
            self.locked_until = Some(now + RATE_LIMIT_LOCKOUT);
            self.failures = 0;
            self.window_start = now;
        }
    }

    fn reset(&mut self) {
        self.failures = 0;
        self.window_start = Instant::now();
        self.locked_until = None;
    }
}

#[derive(Debug)]
pub struct AuthService {
    auth_config: AuthConfig,
    settings: Arc<AppSettingsRepo>,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    env_totp_secret: Option<Vec<u8>>,
    pending_setup: Mutex<Option<PendingSetup>>,
    rate_limiter: Mutex<RateLimiter>,
    used_codes: Mutex<HashMap<String, u64>>,
}

impl AuthService {
    pub async fn new(auth_config: AuthConfig, settings: Arc<AppSettingsRepo>) -> Result<Self> {
        let session_secret = resolve_session_secret(&auth_config, &settings).await?;
        let env_totp_secret = parse_env_totp_secret(&auth_config)?;

        Ok(Self {
            auth_config,
            settings,
            encoding_key: EncodingKey::from_secret(&session_secret),
            decoding_key: DecodingKey::from_secret(&session_secret),
            env_totp_secret,
            pending_setup: Mutex::new(None),
            rate_limiter: Mutex::new(RateLimiter::new()),
            used_codes: Mutex::new(HashMap::new()),
        })
    }

    pub async fn status(&self) -> Result<AuthStatusPayload> {
        Ok(AuthStatusPayload {
            configured: self.resolve_totp_secret().await?.is_some(),
        })
    }

    /// Daily login: verify a TOTP code and issue a session token.
    pub async fn verify(&self, code: &str) -> Result<TokenPayload> {
        self.rate_limiter_lock()?.check()?;

        let code = normalize_code(code);
        let Some(secret) = self.resolve_totp_secret().await? else {
            return Err(AppError::Forbidden(
                "Authenticator is not configured yet".into(),
            ));
        };

        match self.check_code(&secret, &code) {
            Ok(()) => {
                self.rate_limiter_lock()?.reset();
                self.issue_token("Login successful")
            }
            Err(err) => {
                self.rate_limiter_lock()?.record_failure();
                Err(err)
            }
        }
    }

    /// First-run setup, step 1: generate a pending secret for the user to scan.
    pub async fn setup_start(&self) -> Result<SetupStartPayload> {
        self.ensure_setup_allowed().await?;
        if self.resolve_totp_secret().await?.is_some() {
            return Err(AppError::Conflict(
                "Authenticator is already configured".into(),
            ));
        }

        let payload = generate_setup_payload()?;
        *lock(&self.pending_setup)? = Some(PendingSetup {
            secret_b32: payload.secret.clone(),
            created_at: Instant::now(),
        });
        Ok(payload)
    }

    /// First-run setup, step 2: confirm a code against the pending secret and persist it.
    pub async fn setup_confirm(&self, code: &str) -> Result<TokenPayload> {
        self.rate_limiter_lock()?.check()?;
        self.ensure_setup_allowed().await?;
        if self.resolve_totp_secret().await?.is_some() {
            return Err(AppError::Conflict(
                "Authenticator is already configured".into(),
            ));
        }

        let pending = self.valid_pending_setup()?;
        let secret = decode_secret(&pending.secret_b32)?;
        let code = normalize_code(code);

        match self.check_code(&secret, &code) {
            Ok(()) => {
                self.settings.set(TOTP_SECRET_KEY, &pending.secret_b32).await?;
                *lock(&self.pending_setup)? = None;
                self.rate_limiter_lock()?.reset();
                self.issue_token("Authenticator configured")
            }
            Err(err) => {
                self.rate_limiter_lock()?.record_failure();
                Err(err)
            }
        }
    }

    /// Re-bind authenticator, step 1 (requires an authenticated session).
    pub async fn reset_start(&self) -> Result<SetupStartPayload> {
        self.ensure_setup_allowed().await?;

        let payload = generate_setup_payload()?;
        *lock(&self.pending_setup)? = Some(PendingSetup {
            secret_b32: payload.secret.clone(),
            created_at: Instant::now(),
        });
        Ok(payload)
    }

    /// Re-bind authenticator, step 2 (requires an authenticated session).
    pub async fn reset_confirm(&self, code: &str) -> Result<MessagePayload> {
        self.rate_limiter_lock()?.check()?;
        self.ensure_setup_allowed().await?;

        let pending = self.valid_pending_setup()?;
        let secret = decode_secret(&pending.secret_b32)?;
        let code = normalize_code(code);

        match self.check_code(&secret, &code) {
            Ok(()) => {
                self.settings.set(TOTP_SECRET_KEY, &pending.secret_b32).await?;
                *lock(&self.pending_setup)? = None;
                self.rate_limiter_lock()?.reset();
                Ok(MessagePayload {
                    message: "Authenticator updated",
                })
            }
            Err(err) => {
                self.rate_limiter_lock()?.record_failure();
                Err(err)
            }
        }
    }

    /// Validate a session token. Used by the auth middleware and the SSE stream.
    pub fn validate_token(&self, token: &str) -> Result<Claims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[self.auth_config.jwt_issuer.as_str()]);

        Ok(
            decode::<Claims>(token.trim(), &self.decoding_key, &validation)?
                .claims,
        )
    }

    async fn ensure_setup_allowed(&self) -> Result<()> {
        if self.env_totp_secret.is_some() {
            return Err(AppError::Forbidden(
                "Authenticator is managed via environment variable".into(),
            ));
        }
        Ok(())
    }

    async fn resolve_totp_secret(&self) -> Result<Option<Vec<u8>>> {
        if let Some(secret) = &self.env_totp_secret {
            return Ok(Some(secret.clone()));
        }

        let Some(stored) = self.settings.get(TOTP_SECRET_KEY).await? else {
            return Ok(None);
        };

        Ok(Some(decode_secret(&stored)?))
    }

    fn valid_pending_setup(&self) -> Result<PendingSetup> {
        let mut guard = lock(&self.pending_setup)?;
        let Some(pending) = guard.as_ref() else {
            return Err(AppError::BadRequest(
                "No pending authenticator setup, start setup first".into(),
            ));
        };
        if pending.created_at.elapsed() > PENDING_SETUP_TTL {
            *guard = None;
            return Err(AppError::BadRequest(
                "Pending authenticator setup expired, start again".into(),
            ));
        }
        Ok(pending.clone())
    }

    fn check_code(&self, secret: &[u8], code: &str) -> Result<()> {
        if code.len() != TOTP_DIGITS || !code.bytes().all(|b| b.is_ascii_digit()) {
            return Err(AppError::Unauthorized("Invalid authenticator code".into()));
        }

        let totp = build_totp(secret)?;
        let now = now_unix_ts();
        let current_step = now / TOTP_STEP_SECS;

        // Replay protection: a code can only be accepted once within its validity window.
        {
            let mut used = lock(&self.used_codes)?;
            used.retain(|_, step| *step + 1 >= current_step);
            if used.contains_key(code) {
                return Err(AppError::Unauthorized(
                    "Authenticator code has already been used".into(),
                ));
            }
        }

        if !totp.check(code, now) {
            return Err(AppError::Unauthorized("Invalid authenticator code".into()));
        }

        lock(&self.used_codes)?.insert(code.to_string(), current_step);
        Ok(())
    }

    fn issue_token(&self, message: &'static str) -> Result<TokenPayload> {
        let issued_at = now_unix_ts();
        let claims = Claims {
            iss: self.auth_config.jwt_issuer.clone(),
            iat: issued_at,
            exp: issued_at.saturating_add(self.auth_config.jwt_ttl_secs),
            jti: Uuid::now_v7().to_string(),
        };

        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &self.encoding_key,
        )
        .map_err(|err| AppError::Internal(format!("Failed to generate token: {err}")))?;

        Ok(TokenPayload { token, message })
    }

    fn rate_limiter_lock(&self) -> Result<MutexGuard<'_, RateLimiter>> {
        lock(&self.rate_limiter)
    }
}

fn generate_setup_payload() -> Result<SetupStartPayload> {
    let secret = Secret::generate_secret().to_encoded();
    let secret_b32 = secret.to_string();
    let bytes = secret
        .to_bytes()
        .map_err(|err| AppError::Internal(format!("Failed to encode TOTP secret: {err}")))?;
    let totp = build_totp(&bytes)?;

    Ok(SetupStartPayload {
        secret: secret_b32,
        otpauth_url: totp.get_url(),
    })
}

fn build_totp(secret: &[u8]) -> Result<TOTP> {
    TOTP::new(
        TotpAlgorithm::SHA1,
        TOTP_DIGITS,
        1,
        TOTP_STEP_SECS,
        secret.to_vec(),
        Some(TOTP_ISSUER.to_string()),
        TOTP_ACCOUNT.to_string(),
    )
    .map_err(|err| AppError::Internal(format!("Failed to build TOTP verifier: {err}")))
}

fn decode_secret(secret_b32: &str) -> Result<Vec<u8>> {
    Secret::Encoded(secret_b32.trim().to_string())
        .to_bytes()
        .map_err(|err| AppError::Internal(format!("Invalid TOTP secret encoding: {err}")))
}

fn parse_env_totp_secret(config: &AuthConfig) -> Result<Option<Vec<u8>>> {
    let raw = config.totp_secret.trim();
    if raw.is_empty() {
        return Ok(None);
    }

    let bytes = Secret::Encoded(raw.to_string()).to_bytes().map_err(|err| {
        AppError::Internal(format!(
            "AUTH_TOTP_SECRET is not a valid base32 secret: {err}"
        ))
    })?;
    Ok(Some(bytes))
}

async fn resolve_session_secret(
    config: &AuthConfig,
    settings: &Arc<AppSettingsRepo>,
) -> Result<Vec<u8>> {
    let from_env = config.session_secret.trim();
    if !from_env.is_empty() {
        return Ok(from_env.as_bytes().to_vec());
    }

    if let Some(stored) = settings.get(SESSION_SECRET_KEY).await? {
        return BASE64.decode(stored.trim()).map_err(|err| {
            AppError::Internal(format!("Stored session secret is invalid: {err}"))
        });
    }

    let mut secret = [0u8; 32];
    rand::fill(&mut secret);
    settings
        .set(SESSION_SECRET_KEY, &BASE64.encode(secret))
        .await?;
    Ok(secret.to_vec())
}

fn normalize_code(code: &str) -> String {
    code.chars().filter(|c| !c.is_whitespace()).collect()
}

fn lock<T>(mutex: &Mutex<T>) -> Result<MutexGuard<'_, T>> {
    mutex
        .lock()
        .map_err(|_| AppError::Internal("Failed to lock auth state".into()))
}

fn now_unix_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> AuthConfig {
        AuthConfig {
            totp_secret: String::new(),
            session_secret: "test-session-secret".to_string(),
            jwt_issuer: "quantaura".to_string(),
            jwt_ttl_secs: 3600,
        }
    }

    async fn test_service() -> AuthService {
        let db = sea_orm::Database::connect("sqlite::memory:")
            .await
            .expect("in-memory db");

        AuthService {
            auth_config: test_config(),
            settings: Arc::new(AppSettingsRepo::new(db)),
            encoding_key: EncodingKey::from_secret(b"test-session-secret"),
            decoding_key: DecodingKey::from_secret(b"test-session-secret"),
            env_totp_secret: None,
            pending_setup: Mutex::new(None),
            rate_limiter: Mutex::new(RateLimiter::new()),
            used_codes: Mutex::new(HashMap::new()),
        }
    }

    #[tokio::test]
    async fn valid_totp_code_passes_and_replay_is_rejected() {
        let service = test_service().await;
        let secret = Secret::generate_secret()
            .to_bytes()
            .expect("raw secret bytes");
        let totp = build_totp(&secret).expect("build totp");
        let code = totp.generate_current().expect("generate code");

        service
            .check_code(&secret, &code)
            .expect("fresh code should pass");

        let replay = service.check_code(&secret, &code);
        assert!(replay.is_err(), "replayed code must be rejected");
    }

    #[tokio::test]
    async fn wrong_code_is_rejected() {
        let service = test_service().await;
        let secret = Secret::generate_secret()
            .to_bytes()
            .expect("raw secret bytes");
        let totp = build_totp(&secret).expect("build totp");
        let correct = totp.generate_current().expect("generate code");
        let wrong = if correct == "000000" { "000001" } else { "000000" };

        assert!(service.check_code(&secret, wrong).is_err());
        assert!(service.check_code(&secret, "abcdef").is_err());
        assert!(service.check_code(&secret, "12345").is_err());
    }

    #[test]
    fn rate_limiter_locks_out_after_max_failures() {
        let mut limiter = RateLimiter::new();
        for _ in 0..RATE_LIMIT_MAX_FAILURES {
            limiter.check().expect("not locked yet");
            limiter.record_failure();
        }
        assert!(limiter.check().is_err(), "limiter should lock out");

        limiter.reset();
        limiter.check().expect("reset clears lockout");
    }

    #[tokio::test]
    async fn issued_token_validates_with_issuer() {
        let service = test_service().await;
        let payload = service.issue_token("ok").expect("issue token");

        let claims = service.validate_token(&payload.token).expect("validate");
        assert_eq!(claims.iss, "quantaura");

        let tampered = format!("{}x", payload.token);
        assert!(service.validate_token(&tampered).is_err());
    }
}
