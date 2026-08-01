use std::{
    net::{IpAddr, SocketAddr},
    path::Path,
};

use envconfig::Envconfig;

macro_rules! ensure {
    ($cond:expr, $msg:expr) => {
        assert!($cond, "Invalid config: {}", $msg);
    };
}

macro_rules! validate_all {
    ($($cond:expr => $msg:expr),+ $(,)?) => {
        $(ensure!($cond, $msg);)+
    };
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub app: AppMetadataConfig,
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub database: DatabaseConfig,
    pub logging: LoggingConfig,
    pub live: LiveRuntimeConfig,
    pub runtime_alerts: RuntimeAlertConfig,
}

#[derive(Debug, Clone, Envconfig)]
struct EnvAppConfig {
    #[envconfig(nested)]
    app: AppMetadataConfig,
    #[envconfig(nested)]
    server: ServerConfig,
    #[envconfig(nested)]
    auth: AuthConfig,
    #[envconfig(nested)]
    database: DatabaseConfig,
    #[envconfig(nested)]
    logging: LoggingConfig,
    #[envconfig(nested)]
    live: LiveRuntimeConfig,
    #[envconfig(nested)]
    runtime_alerts: RuntimeAlertConfig,
}

impl EnvAppConfig {
    fn into_app_config(self) -> Result<AppConfig, String> {
        Ok(AppConfig {
            app: self.app,
            server: self.server,
            auth: self.auth,
            database: self.database,
            logging: self.logging,
            live: self.live,
            runtime_alerts: self.runtime_alerts,
        })
    }
}

impl AppConfig {
    pub fn from_env() -> Self {
        load_dotenv();

        let env_config = EnvAppConfig::init_from_env()
            .unwrap_or_else(|err| panic!("Failed to load config from env: {err}"));
        let config = env_config
            .into_app_config()
            .unwrap_or_else(|err| panic!("Failed to load config: {err}"));

        config.validate();

        config
    }

    pub fn validate(&self) {
        self.server.validate();
        self.auth.validate();
        self.database.validate();
        self.live.validate();
        self.runtime_alerts.validate();
    }

    pub fn server_addr(&self) -> SocketAddr {
        self.server.socket_addr()
    }
}

fn load_dotenv() {
    if let Ok(path) = std::env::var("ENV_FILE") {
        dotenvy::from_path(&path)
            .unwrap_or_else(|err| panic!("Failed to load env file from {path}: {err}"));
        return;
    }

    for path in ["../.env", ".env"] {
        if Path::new(path).exists() {
            dotenvy::from_path(path)
                .unwrap_or_else(|err| panic!("Failed to load env file from {path}: {err}"));
            return;
        }
    }
}

#[derive(Debug, Clone, Envconfig)]
pub struct AppMetadataConfig {
    #[envconfig(from = "APP_NAME", default = "quantaura")]
    pub name: String,
    #[envconfig(from = "ENV", default = "development")]
    pub environment: String,
}

#[derive(Debug, Clone, Envconfig)]
pub struct ServerConfig {
    #[envconfig(from = "HOST", default = "0.0.0.0")]
    pub host: IpAddr,
    #[envconfig(from = "PORT", default = "8080")]
    pub port: u16,
    #[envconfig(from = "CORS_ALLOW_ORIGIN", default = "*")]
    pub cors_allow_origin: String,
    #[envconfig(from = "REQUEST_TIMEOUT_SECS", default = "15")]
    pub request_timeout_secs: u64,
}

impl ServerConfig {
    fn validate(&self) {
        validate_all! {
            self.port != 0 => "PORT cannot be 0",
        }
    }

    fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.host, self.port)
    }
}

#[derive(Debug, Clone, Envconfig)]
pub struct AuthConfig {
    /// Optional base32 TOTP secret. When set, the authenticator is managed
    /// via environment and the in-app setup/reset endpoints are disabled.
    #[envconfig(from = "AUTH_TOTP_SECRET", default = "")]
    pub totp_secret: String,
    /// Optional session signing secret. When empty, a random secret is
    /// generated on first boot and persisted in the `app_settings` table.
    #[envconfig(from = "AUTH_SESSION_SECRET", default = "")]
    pub session_secret: String,
    #[envconfig(from = "JWT_ISSUER", default = "quantaura")]
    pub jwt_issuer: String,
    #[envconfig(from = "JWT_TTL_SECS", default = "604800")]
    pub jwt_ttl_secs: u64,
}

impl AuthConfig {
    fn validate(&self) {
        validate_all! {
            !self.jwt_issuer.trim().is_empty() => "JWT_ISSUER cannot be empty",
            self.jwt_ttl_secs != 0 => "JWT_TTL_SECS cannot be 0",
        }
    }
}

#[derive(Debug, Clone, Envconfig)]
pub struct DatabaseConfig {
    #[envconfig(from = "DB_URL", default = "sqlite://data/quantaura.db")]
    pub url: String,
}

impl DatabaseConfig {
    fn validate(&self) {
        validate_all! {
            !self.url.trim().is_empty() => "DB_URL cannot be empty",
        }
    }
}

#[derive(Debug, Clone, Envconfig)]
pub struct LoggingConfig {
    #[envconfig(from = "LOG_LEVEL", default = "info")]
    pub level: String,
}

#[derive(Debug, Clone, Envconfig)]
pub struct LiveRuntimeConfig {
    #[envconfig(from = "LIVE_OPEN_ORDER_COOLDOWN_SECS", default = "90")]
    pub open_order_cooldown_secs: u64,
    #[envconfig(from = "LIVE_STALE_OPEN_ORDER_CANCEL_SECS", default = "180")]
    pub stale_open_order_cancel_secs: u64,
    #[envconfig(from = "LIVE_STALE_LIMIT_ORDER_REPLACE_SECS", default = "90")]
    pub stale_limit_order_replace_secs: u64,
    #[envconfig(from = "LIVE_REPLACE_MAX_ATTEMPTS_PER_WINDOW", default = "3")]
    pub replace_max_attempts_per_window: u64,
    #[envconfig(from = "LIVE_REPLACE_ATTEMPT_WINDOW_SECS", default = "180")]
    pub replace_attempt_window_secs: u64,
    #[envconfig(from = "LIVE_SUBMITTED_INTENT_RECONCILE_SECS", default = "300")]
    pub submitted_intent_reconcile_secs: u64,
    #[envconfig(from = "LIVE_RISK_SOFT_DRAWDOWN_PCT", default = "15.0")]
    pub risk_soft_drawdown_pct: f64,
    #[envconfig(from = "LIVE_RISK_MEDIUM_DRAWDOWN_PCT", default = "25.0")]
    pub risk_medium_drawdown_pct: f64,
    #[envconfig(from = "LIVE_RISK_HARD_DRAWDOWN_PCT", default = "35.0")]
    pub risk_hard_drawdown_pct: f64,
    #[envconfig(from = "LIVE_RISK_SOFT_MARGIN_RATIO", default = "0.70")]
    pub risk_soft_margin_ratio: f64,
    #[envconfig(from = "LIVE_RISK_MEDIUM_MARGIN_RATIO", default = "0.82")]
    pub risk_medium_margin_ratio: f64,
    #[envconfig(from = "LIVE_RISK_HARD_MARGIN_RATIO", default = "0.90")]
    pub risk_hard_margin_ratio: f64,
    #[envconfig(from = "LIVE_RISK_SOFT_OPEN_COOLDOWN_MULTIPLIER", default = "2")]
    pub risk_soft_open_cooldown_multiplier: u64,
    #[envconfig(from = "LIVE_RISK_MEDIUM_REDUCE_POSITIONS_COUNT", default = "1")]
    pub risk_medium_reduce_positions_count: u64,
    #[envconfig(from = "LIVE_RISK_HARD_CLOSE_WORST_POSITIONS_COUNT", default = "2")]
    pub risk_hard_close_worst_positions_count: u64,
}

impl LiveRuntimeConfig {
    fn validate(&self) {
        validate_all! {
            self.open_order_cooldown_secs != 0 => "LIVE_OPEN_ORDER_COOLDOWN_SECS cannot be 0",
            self.stale_open_order_cancel_secs != 0 => "LIVE_STALE_OPEN_ORDER_CANCEL_SECS cannot be 0",
            self.stale_limit_order_replace_secs != 0 => "LIVE_STALE_LIMIT_ORDER_REPLACE_SECS cannot be 0",
            self.replace_max_attempts_per_window != 0 => "LIVE_REPLACE_MAX_ATTEMPTS_PER_WINDOW cannot be 0",
            self.replace_attempt_window_secs != 0 => "LIVE_REPLACE_ATTEMPT_WINDOW_SECS cannot be 0",
            self.submitted_intent_reconcile_secs != 0 => "LIVE_SUBMITTED_INTENT_RECONCILE_SECS cannot be 0",
            self.risk_soft_drawdown_pct > 0.0
                && self.risk_medium_drawdown_pct > 0.0
                && self.risk_hard_drawdown_pct > 0.0
                => "live risk drawdown thresholds must be > 0",
            self.risk_soft_drawdown_pct <= self.risk_medium_drawdown_pct
                && self.risk_medium_drawdown_pct <= self.risk_hard_drawdown_pct
                => "live risk drawdown thresholds must satisfy soft <= medium <= hard",
            self.risk_soft_margin_ratio > 0.0
                && self.risk_medium_margin_ratio > 0.0
                && self.risk_hard_margin_ratio > 0.0
                && self.risk_soft_margin_ratio <= 1.0
                && self.risk_medium_margin_ratio <= 1.0
                && self.risk_hard_margin_ratio <= 1.0
                => "live risk margin ratios must be in (0, 1]",
            self.risk_soft_margin_ratio <= self.risk_medium_margin_ratio
                && self.risk_medium_margin_ratio <= self.risk_hard_margin_ratio
                => "live risk margin ratios must satisfy soft <= medium <= hard",
            self.risk_soft_open_cooldown_multiplier != 0 => "LIVE_RISK_SOFT_OPEN_COOLDOWN_MULTIPLIER cannot be 0",
            self.risk_medium_reduce_positions_count != 0 => "LIVE_RISK_MEDIUM_REDUCE_POSITIONS_COUNT cannot be 0",
            self.risk_hard_close_worst_positions_count != 0 => "LIVE_RISK_HARD_CLOSE_WORST_POSITIONS_COUNT cannot be 0",
        }
    }
}

#[derive(Debug, Clone, Envconfig)]
pub struct RuntimeAlertConfig {
    #[envconfig(from = "RUNTIME_ALERT_WEBHOOK_URL", default = "")]
    pub url: String,
    #[envconfig(from = "RUNTIME_ALERT_WEBHOOK_AUTH_HEADER", default = "")]
    pub auth_header: String,
    #[envconfig(from = "RUNTIME_ALERT_WEBHOOK_TIMEOUT_SECS", default = "5")]
    pub timeout_secs: u64,
    #[envconfig(from = "RUNTIME_ALERT_WEBHOOK_MAX_RETRIES", default = "3")]
    pub max_retries: u64,
    #[envconfig(from = "RUNTIME_ALERT_WEBHOOK_RETRY_BACKOFF_MS", default = "500")]
    pub retry_backoff_ms: u64,
    #[envconfig(from = "RUNTIME_ALERT_WEBHOOK_SIGNING_SECRET", default = "")]
    pub signing_secret: String,
    #[envconfig(
        from = "RUNTIME_ALERT_WEBHOOK_SIGNING_HEADER",
        default = "X-QuantAura-Signature"
    )]
    pub signing_header: String,
    #[envconfig(
        from = "RUNTIME_ALERT_WEBHOOK_SIGNING_TIMESTAMP_HEADER",
        default = "X-QuantAura-Timestamp"
    )]
    pub signing_timestamp_header: String,
    #[envconfig(from = "RUNTIME_ALERT_WEBHOOK_SIGNING_MAX_AGE_SECS", default = "300")]
    pub signing_max_age_secs: u64,
}

impl RuntimeAlertConfig {
    fn validate(&self) {
        let webhook_url = self.url.trim();

        validate_all! {
            self.timeout_secs != 0 => "RUNTIME_ALERT_WEBHOOK_TIMEOUT_SECS cannot be 0",
            self.max_retries != 0 => "RUNTIME_ALERT_WEBHOOK_MAX_RETRIES cannot be 0",
            self.retry_backoff_ms != 0 => "RUNTIME_ALERT_WEBHOOK_RETRY_BACKOFF_MS cannot be 0",
            webhook_url.is_empty() || webhook_url.starts_with("http://") || webhook_url.starts_with("https://")
                => "RUNTIME_ALERT_WEBHOOK_URL must start with http:// or https://",
            !self.signing_enabled() || !self.signing_header.trim().is_empty()
                => "RUNTIME_ALERT_WEBHOOK_SIGNING_HEADER cannot be empty when signing secret is set",
            !self.signing_enabled() || !self.signing_timestamp_header.trim().is_empty()
                => "RUNTIME_ALERT_WEBHOOK_SIGNING_TIMESTAMP_HEADER cannot be empty when signing secret is set",
            !self.signing_enabled() || self.signing_max_age_secs != 0
                => "RUNTIME_ALERT_WEBHOOK_SIGNING_MAX_AGE_SECS cannot be 0 when signing secret is set",
        }
    }

    pub fn enabled(&self) -> bool {
        !self.url.trim().is_empty()
    }

    pub fn auth_header_set(&self) -> bool {
        !self.auth_header.trim().is_empty()
    }

    pub fn signing_enabled(&self) -> bool {
        !self.signing_secret.trim().is_empty()
    }

    pub fn signing_header_set(&self) -> bool {
        !self.signing_header.trim().is_empty()
    }

    pub fn signing_timestamp_header_set(&self) -> bool {
        !self.signing_timestamp_header.trim().is_empty()
    }
}
