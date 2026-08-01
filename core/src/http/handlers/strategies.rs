use axum::{
    Json,
    extract::{Path, Query, State},
};

use crate::{
    contracts::strategies::{
        CreateStrategyRequest, DefaultStrategyConfigQuery, DuplicateStrategyRequest,
        PreviewPromptPayload, PreviewPromptRequest, StrategyCreatedPayload,
        StrategyDefaultConfigPayload, StrategyListPayload, StrategyMessagePayload, StrategyPayload,
        StrategyTestRunPayload, StrategyTestRunRequest, UpdateStrategyRequest,
    },
    error::Result,
    http::response::ApiResponse,
    state::AppState,
};

pub async fn handle_get_strategies(
    State(app): State<AppState>,
) -> Result<Json<ApiResponse<StrategyListPayload>>> {
    let payload = app
        .services
        .strategy_service
        .list_strategies()
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_get_strategy(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<StrategyPayload>>> {
    let payload = app
        .services
        .strategy_service
        .get_strategy(id)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_create_strategy(
    State(app): State<AppState>,
    Json(request): Json<CreateStrategyRequest>,
) -> Result<Json<ApiResponse<StrategyCreatedPayload>>> {
    let payload = app
        .services
        .strategy_service
        .create_strategy(request)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_update_strategy(
    State(app): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<UpdateStrategyRequest>,
) -> Result<Json<ApiResponse<StrategyMessagePayload>>> {
    let payload = app
        .services
        .strategy_service
        .update_strategy(id, request)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_delete_strategy(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<StrategyMessagePayload>>> {
    let payload = app
        .services
        .strategy_service
        .delete_strategy(id)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_activate_strategy(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<StrategyMessagePayload>>> {
    let payload = app
        .services
        .strategy_service
        .activate_strategy(id)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_duplicate_strategy(
    State(app): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<DuplicateStrategyRequest>,
) -> Result<Json<ApiResponse<StrategyCreatedPayload>>> {
    let payload = app
        .services
        .strategy_service
        .duplicate_strategy(id, request)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_get_active_strategy(
    State(app): State<AppState>,
) -> Result<Json<ApiResponse<StrategyPayload>>> {
    let payload = app
        .services
        .strategy_service
        .active_strategy()
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_get_default_strategy_config(
    State(app): State<AppState>,
    Query(query): Query<DefaultStrategyConfigQuery>,
) -> Result<Json<ApiResponse<StrategyDefaultConfigPayload>>> {
    let payload = app
        .services
        .strategy_service
        .default_strategy_config(query)?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_preview_prompt(
    State(app): State<AppState>,
    Json(request): Json<PreviewPromptRequest>,
) -> Result<Json<ApiResponse<PreviewPromptPayload>>> {
    let payload = app.services.strategy_service.preview_prompt(request)?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_strategy_test_run(
    State(app): State<AppState>,
    Json(request): Json<StrategyTestRunRequest>,
) -> Result<Json<ApiResponse<StrategyTestRunPayload>>> {
    let payload = app
        .services
        .strategy_service
        .test_run(request)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}
