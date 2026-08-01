use axum::{
    Json,
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{http::response::ApiResponse, state::AppState};

/// Route-level auth guard: accepts a session token from the
/// `Authorization: Bearer` header or the `?token=` query parameter
pub async fn require_auth(State(app): State<AppState>, request: Request, next: Next) -> Response {
    let Some(token) = extract_token(&request) else {
        return unauthorized();
    };

    match app.services.auth_service.validate_token(&token) {
        Ok(_) => next.run(request).await,
        Err(_) => unauthorized(),
    }
}

fn extract_token(request: &Request) -> Option<String> {
    if let Some(token) = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        return Some(token.to_string());
    }

    request.uri().query().and_then(|query| {
        query.split('&').find_map(|pair| {
            pair.strip_prefix("token=")
                .map(str::trim)
                .filter(|token| !token.is_empty())
                .map(str::to_string)
        })
    })
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(ApiResponse::<()>::failure("Invalid token")),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use axum::http::Request as HttpRequest;

    use super::*;

    fn request_with(uri: &str, authorization: Option<&str>) -> Request {
        let mut builder = HttpRequest::builder().uri(uri);
        if let Some(value) = authorization {
            builder = builder.header(header::AUTHORIZATION, value);
        }
        builder.body(axum::body::Body::empty()).expect("request")
    }

    #[test]
    fn extracts_bearer_token_from_header() {
        let request = request_with("/api/config", Some("Bearer abc.def.ghi"));
        assert_eq!(extract_token(&request).as_deref(), Some("abc.def.ghi"));
    }

    #[test]
    fn extracts_token_from_query_param() {
        let request = request_with("/api/events?token=abc.def.ghi", None);
        assert_eq!(extract_token(&request).as_deref(), Some("abc.def.ghi"));
    }

    #[test]
    fn header_takes_precedence_over_query() {
        let request = request_with("/api/events?token=query", Some("Bearer header"));
        assert_eq!(extract_token(&request).as_deref(), Some("header"));
    }

    #[test]
    fn missing_or_malformed_tokens_are_rejected() {
        assert!(extract_token(&request_with("/api/config", None)).is_none());
        assert!(extract_token(&request_with("/api/config", Some("Basic abc"))).is_none());
        assert!(extract_token(&request_with("/api/config", Some("Bearer "))).is_none());
        assert!(extract_token(&request_with("/api/events?token=", None)).is_none());
    }
}
