use axum::{Json, extract::State};

use crate::{
    contracts::models::{
        AvailableModelListPayload, MessagePayload, ModelConfigPayload, ModelProviderProbeRequest,
        ProviderAvailabilityPayload, ProviderAvailabilityRequest, UpdateModelConfigRequest,
    },
    error::{AppError, Result},
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
    Json(UpdateModelConfigRequest { providers }): Json<UpdateModelConfigRequest>,
) -> Result<Json<ApiResponse<MessagePayload>>> {
    if providers.is_empty() {
        return Err(AppError::BadRequest("providers list is required".into()));
    }
    let payload = app.services.model_service.update_configs(providers).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn list_available_models(
    State(app): State<state::AppState>,
    Json(ModelProviderProbeRequest {
        provider_type,
        api_key,
        base_url,
    }): Json<ModelProviderProbeRequest>,
) -> Result<Json<ApiResponse<AvailableModelListPayload>>> {
    if provider_type.trim().is_empty() {
        return Err(AppError::BadRequest("providerType is required".into()));
    }
    let payload = app
        .services
        .model_service
        .list_available_models(provider_type, api_key, base_url)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn check_provider_availability(
    State(app): State<state::AppState>,
    Json(ProviderAvailabilityRequest {
        provider_type,
        api_key,
        base_url,
        model_id,
    }): Json<ProviderAvailabilityRequest>,
) -> Result<Json<ApiResponse<ProviderAvailabilityPayload>>> {
    if provider_type.trim().is_empty() {
        return Err(AppError::BadRequest("providerType is required".into()));
    }
    let payload = app
        .services
        .model_service
        .check_provider_availability(provider_type, api_key, base_url, model_id)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}
