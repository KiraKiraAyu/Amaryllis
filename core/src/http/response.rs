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
                tracing::warn!("Unauthorized error: {}", message);
                (StatusCode::UNAUTHORIZED, message)
            }
            AppError::Forbidden(message) => {
                tracing::warn!("Forbidden error: {}", message);
                (StatusCode::FORBIDDEN, "Forbidden".to_string())
            }
            AppError::NotFound(message) => {
                tracing::warn!("Not found error: {}", message);
                (StatusCode::NOT_FOUND, message)
            }
            AppError::Conflict(message) => {
                tracing::warn!("Conflict error: {}", message);
                (StatusCode::CONFLICT, message)
            }
            AppError::AlreadyRunning(trader_id) => {
                tracing::warn!("Trader already running: {}", trader_id);
                (
                    StatusCode::CONFLICT,
                    format!("Trader `{trader_id}` is already running"),
                )
            }
            AppError::NotRunning(trader_id) => {
                tracing::warn!("Trader not running: {}", trader_id);
                (
                    StatusCode::CONFLICT,
                    format!("Trader `{trader_id}` is not running"),
                )
            }
            AppError::TraderNotFound(trader_id) => {
                tracing::warn!("Trader not found or no permission: {}", trader_id);
                (StatusCode::NOT_FOUND, "Trader not found".to_string())
            }
            AppError::BadRequest(message) => {
                tracing::warn!("Bad request error: {}", message);
                (StatusCode::BAD_REQUEST, message)
            }
            AppError::InvalidConfig(message) => {
                tracing::warn!("Invalid trader configuration: {}", message);
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
                tracing::warn!("Unsupported exchange: {}", exchange);
                (
                    StatusCode::BAD_REQUEST,
                    format!("Unsupported exchange: {exchange}"),
                )
            }
            AppError::InvalidExchangeConfig(message) => {
                tracing::warn!("Invalid exchange configuration: {}", message);
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
    fn maps_all_app_error_status_codes_correctly() {
        assert_eq!(
            AppError::BadRequest("Invalid".into()).into_response().status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            AppError::Unauthorized("No auth".into()).into_response().status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AppError::Forbidden("Denied".into()).into_response().status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            AppError::NotFound("Missing".into()).into_response().status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            AppError::Conflict("Duplicate".into()).into_response().status(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            AppError::BadGateway("Upstream failed".into()).into_response().status(),
            StatusCode::BAD_GATEWAY
        );
        assert_eq!(
            AppError::Internal("Server crash".into()).into_response().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn api_response_serializes_success_and_failure_structures() {
        let success_resp = ApiResponse::success(Some(42), "ok".to_string());
        assert!(success_resp.success);
        assert_eq!(success_resp.data, Some(42));
        assert_eq!(success_resp.message, Some("ok".to_string()));

        let failure_resp = ApiResponse::failure("something went wrong");
        assert!(!failure_resp.success);
        assert_eq!(failure_resp.error, Some("something went wrong".to_string()));
    }
}
