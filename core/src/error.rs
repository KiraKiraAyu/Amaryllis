use sea_orm::DbErr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Trader `{0}` is already running")]
    AlreadyRunning(String),

    #[error("Trader `{0}` is not running")]
    NotRunning(String),

    #[error("Trader `{0}` not found or no permission")]
    TraderNotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Invalid trader configuration: {0}")]
    InvalidConfig(String),

    #[error("Bad gateway: {0}")]
    BadGateway(String),

    #[error("Unsupported exchange: {0}")]
    UnsupportedExchange(String),

    #[error("Invalid exchange configuration: {0}")]
    InvalidExchangeConfig(String),

    #[error("Exchange request failed: {0}")]
    ExchangeHttp(reqwest::Error),

    #[error("Exchange returned invalid JSON: {0}")]
    ExchangeJson(serde_json::Error),

    #[error("Exchange API error (status={status}, code={code}): {message}")]
    ExchangeApi {
        status: u16,
        code: i64,
        message: String,
    },

    #[error("Failed to build exchange timestamp: {0}")]
    ExchangeTime(String),

    #[error("Failed to sign exchange request: {0}")]
    ExchangeCrypto(String),

    #[error("Budget exhausted: {0}")]
    BudgetExhausted(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Database error: {0}")]
    Database(#[from] DbErr),

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("UUID error: {0}")]
    Uuid(#[from] uuid::Error),

    #[error("Runtime task failed: {0}")]
    Join(#[from] tokio::task::JoinError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppErrorKind {
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    BadGateway,
    Internal,
    BadRequest,
}

impl AppError {
    pub fn from_kind(kind: AppErrorKind, message: impl Into<String>) -> Self {
        let message = message.into();
        match kind {
            AppErrorKind::Unauthorized => AppError::Unauthorized(message),
            AppErrorKind::Forbidden => AppError::Forbidden(message),
            AppErrorKind::NotFound => AppError::NotFound(message),
            AppErrorKind::Conflict => AppError::Conflict(message),
            AppErrorKind::BadGateway => AppError::BadGateway(message),
            AppErrorKind::Internal => AppError::Internal(message),
            AppErrorKind::BadRequest => AppError::BadRequest(message),
        }
    }
}

impl AppError {
    pub fn is_exchange_order_missing(&self) -> bool {
        if let AppError::ExchangeApi { code, message, .. } = self {
            // Binance: -2011 "Unknown order sent."
            if *code == -2011 || message.contains("Unknown order") {
                return true;
            }
        }
        false
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
