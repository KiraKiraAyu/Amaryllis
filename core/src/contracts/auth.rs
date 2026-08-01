use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub iss: String,
    pub iat: u64,
    pub exp: u64,
    pub jti: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct SetupConfirmRequest {
    pub code: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthStatusPayload {
    pub configured: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SetupStartPayload {
    pub secret: String,
    pub otpauth_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenPayload {
    pub token: String,
    pub message: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct MessagePayload {
    pub message: &'static str,
}
