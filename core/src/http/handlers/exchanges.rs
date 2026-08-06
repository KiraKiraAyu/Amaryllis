use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    contracts::exchanges::{
        CreateExchangePayload, CreateExchangeRequest, MessagePayload, SafeExchangeConfig,
        UpdateExchangeConfigRequest,
    },
    error::{AppError, Result},
    http::response::ApiResponse,
    state,
};

pub async fn get_exchange_configs(
    State(app): State<state::AppState>,
) -> Result<Json<ApiResponse<Vec<SafeExchangeConfig>>>> {
    let payload = app
        .services
        .exchange_config_service
        .list_configs()
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn create_exchange(
    State(app): State<state::AppState>,
    Json(CreateExchangeRequest {
        exchange_type,
        account_name,
        enabled,
        api_key,
        secret_key,
        passphrase,
        testnet,
        hyperliquid_wallet_addr,
    }): Json<CreateExchangeRequest>,
) -> Result<Json<ApiResponse<CreateExchangePayload>>> {
    if exchange_type.trim().is_empty() {
        return Err(AppError::BadRequest(
            "exchange_type is required".into(),
        ));
    }
    let payload = app
        .services
        .exchange_config_service
        .create_exchange(
            exchange_type,
            account_name,
            enabled,
            api_key,
            secret_key,
            passphrase,
            testnet,
            hyperliquid_wallet_addr,
        )
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn update_exchange_configs(
    State(app): State<state::AppState>,
    Json(UpdateExchangeConfigRequest { exchanges }): Json<UpdateExchangeConfigRequest>,
) -> Result<Json<ApiResponse<MessagePayload>>> {
    if exchanges.is_empty() {
        return Err(AppError::BadRequest(
            "exchanges map is required".into(),
        ));
    }
    let payload = app
        .services
        .exchange_config_service
        .update_configs(exchanges)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn delete_exchange(
    State(app): State<state::AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<MessagePayload>>> {
    let payload = app
        .services
        .exchange_config_service
        .delete_exchange(&id)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}
