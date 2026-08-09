use axum::{Json, extract::State};

use crate::{
    contracts::auth::{
        AuthStatusPayload, MessagePayload, SetupConfirmRequest, SetupStartPayload, TokenPayload,
        VerifyRequest,
    },
    error::Result,
    http::response::ApiResponse,
    state::AppState,
};

pub async fn status(State(app): State<AppState>) -> Result<Json<ApiResponse<AuthStatusPayload>>> {
    let payload = app.services.auth_service.status().await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn verify(
    State(app): State<AppState>,
    Json(request): Json<VerifyRequest>,
) -> Result<Json<ApiResponse<TokenPayload>>> {
    let payload = app.services.auth_service.verify(&request.code).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn setup_start(
    State(app): State<AppState>,
) -> Result<Json<ApiResponse<SetupStartPayload>>> {
    let payload = app.services.auth_service.setup_start().await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn setup_confirm(
    State(app): State<AppState>,
    Json(request): Json<SetupConfirmRequest>,
) -> Result<Json<ApiResponse<TokenPayload>>> {
    let payload = app
        .services
        .auth_service
        .setup_confirm(&request.code)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn reset_start(
    State(app): State<AppState>,
) -> Result<Json<ApiResponse<SetupStartPayload>>> {
    let payload = app.services.auth_service.reset_start().await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn reset_confirm(
    State(app): State<AppState>,
    Json(request): Json<SetupConfirmRequest>,
) -> Result<Json<ApiResponse<MessagePayload>>> {
    let payload = app
        .services
        .auth_service
        .reset_confirm(&request.code)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}
