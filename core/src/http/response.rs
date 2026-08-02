use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::error::AppError;

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: Option<String>,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: Option<T>, message: impl Into<Option<String>>) -> Self {
        ApiResponse {
            success: true,
            message: message.into(),
            data,
            error: None,
        }
    }
}

impl ApiResponse<()> {
    pub fn failure<S: Into<String>>(error: S) -> Self {
        ApiResponse {
            success: false,
            message: None,
            data: None,
            error: Some(error.into()),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::Unauthorized(message) => {
                tracing::error!("Unauthorized error: {}", message);
                (StatusCode::UNAUTHORIZED, message)
            }
            AppError::Forbidden(message) => {
                tracing::error!("Forbidden error: {}", message);
                (StatusCode::FORBIDDEN, "Forbidden".to_string())
            }
            AppError::NotFound(message) => {
                tracing::error!("Not found error: {}", message);
                (StatusCode::NOT_FOUND, message)
            }
            AppError::Conflict(message) => {
                tracing::error!("Conflict error: {}", message);
                (StatusCode::CONFLICT, message)
            }
            AppError::AlreadyRunning(trader_id) => {
                tracing::error!("Trader already running: {}", trader_id);
                (
                    StatusCode::CONFLICT,
                    format!("Trader `{trader_id}` is already running"),
                )
            }
            AppError::NotRunning(trader_id) => {
                tracing::error!("Trader not running: {}", trader_id);
                (
                    StatusCode::CONFLICT,
                    format!("Trader `{trader_id}` is not running"),
                )
            }
            AppError::TraderNotFound(trader_id) => {
                tracing::error!("Trader not found or no permission: {}", trader_id);
                (StatusCode::NOT_FOUND, "Trader not found".to_string())
            }
            AppError::BadRequest(message) => {
                tracing::error!("Bad request error: {}", message);
                (StatusCode::BAD_REQUEST, message)
            }
            AppError::InvalidConfig(message) => {
                tracing::error!("Invalid trader configuration: {}", message);
                (
                    StatusCode::BAD_REQUEST,
                    format!("Invalid trader configuration: {message}"),
                )
            }
            AppError::BadGateway(message) => {
                tracing::error!("Bad gateway error: {}", message);
                (StatusCode::BAD_GATEWAY, message)
            }
            AppError::UnsupportedExchange(exchange) => {
                tracing::error!("Unsupported exchange: {}", exchange);
                (
                    StatusCode::BAD_REQUEST,
                    format!("Unsupported exchange: {exchange}"),
                )
            }
            AppError::InvalidExchangeConfig(message) => {
                tracing::error!("Invalid exchange configuration: {}", message);
                (
                    StatusCode::BAD_REQUEST,
                    format!("Invalid exchange configuration: {message}"),
                )
            }
            AppError::ExchangeHttp(err) => {
                tracing::error!("Exchange request failed: {:?}", err);
                (
                    StatusCode::BAD_GATEWAY,
                    format!("Exchange request failed: {err}"),
                )
            }
            AppError::ExchangeJson(err) => {
                tracing::error!("Exchange returned invalid JSON: {:?}", err);
                (
                    StatusCode::BAD_GATEWAY,
                    format!("Exchange returned invalid JSON: {err}"),
                )
            }
            AppError::ExchangeApi { message, .. } => {
                tracing::error!("Exchange API error: {}", message);
                (
                    StatusCode::BAD_GATEWAY,
                    format!("Exchange API error: {message}"),
                )
            }
            AppError::ExchangeTime(message) => {
                tracing::error!("Failed to build exchange timestamp: {}", message);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server error".to_string(),
                )
            }
            AppError::ExchangeCrypto(message) => {
                tracing::error!("Failed to sign exchange request: {}", message);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to sign exchange request: {message}"),
                )
            }
            AppError::Internal(message) => {
                tracing::error!("Internal server error. {}", message);
                (StatusCode::INTERNAL_SERVER_ERROR, message)
            }
            AppError::BudgetExhausted(message) => {
                tracing::warn!("Budget exhausted: {}", message);
                (StatusCode::OK, format!("Budget exhausted: {message}"))
            }
            AppError::Database(err) => {
                tracing::error!("Database error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error".to_string(),
                )
            }
            AppError::Jwt(err) => {
                tracing::error!("JWT error: {:?}", err);
                (StatusCode::UNAUTHORIZED, "Invalid token".to_string())
            }
            AppError::Request(err) => {
                tracing::error!("Request error: {:?}", err);
                (
                    StatusCode::BAD_GATEWAY,
                    "Upstream request failed".to_string(),
                )
            }
            AppError::Serialization(err) => {
                tracing::error!("Serialization error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server error".to_string(),
                )
            }
            AppError::Uuid(err) => {
                tracing::error!("UUID error: {:?}", err);
                (StatusCode::BAD_REQUEST, "Invalid id".to_string())
            }
            AppError::Join(err) => {
                tracing::error!("Runtime task failed: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server error".to_string(),
                )
            }
        };

        let body = Json(ApiResponse::failure(error_message));
        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use axum::response::IntoResponse;

    use super::*;

    #[test]
    fn bad_request_maps_to_bad_request_status() {
        let response = AppError::BadRequest("Invalid request body".into()).into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
