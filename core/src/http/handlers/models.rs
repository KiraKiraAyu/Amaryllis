use axum::{Json, extract::State};

use crate::{
    contracts::models::{
        AvailableModelListPayload, MessagePayload, ModelConfigPayload, ModelProviderProbeRequest,
        ProviderAvailabilityPayload, ProviderAvailabilityRequest, UpdateModelConfigRequest,
    },
    error::Result,
    http::response::ApiResponse,
    state,
};

pub async fn get_model_configs(
    State(app): State<state::AppState>,
) -> Result<Json<ApiResponse<ModelConfigPayload>>> {
    let payload = app.services.model_service.list_configs().await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn update_model_configs(
    State(app): State<state::AppState>,
    Json(request): Json<UpdateModelConfigRequest>,
) -> Result<Json<ApiResponse<MessagePayload>>> {
    let payload = app
        .services
        .model_service
        .update_configs(request)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn list_available_models(
    State(app): State<state::AppState>,
    Json(request): Json<ModelProviderProbeRequest>,
) -> Result<Json<ApiResponse<AvailableModelListPayload>>> {
    let payload = app
        .services
        .model_service
        .list_available_models(request)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn check_provider_availability(
    State(app): State<state::AppState>,
    Json(request): Json<ProviderAvailabilityRequest>,
) -> Result<Json<ApiResponse<ProviderAvailabilityPayload>>> {
    let payload = app
        .services
        .model_service
        .check_provider_availability(request)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}
