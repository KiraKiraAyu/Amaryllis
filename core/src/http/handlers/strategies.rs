use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    contracts::strategies::{
        CreateStrategyRequest, DuplicateStrategyRequest,
        PreviewPromptPayload, PreviewPromptRequest, StrategyCreatedPayload,
        StrategyDefaultConfigPayload, StrategyListPayload, StrategyMessagePayload, StrategyPayload,
        StrategyTestRunPayload, StrategyTestRunRequest, UpdateStrategyRequest,
    },
    contracts::trading::positions::StrategyPositionListPayload,
    error::{AppError, Result},
    http::response::ApiResponse,
    state::AppState,
};

pub async fn handle_get_strategies(
    State(app): State<AppState>,
) -> Result<Json<ApiResponse<StrategyListPayload>>> {
    let payload = app.services.strategy_service.list_strategies().await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_get_strategy(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<StrategyPayload>>> {
    let payload = app.services.strategy_service.get_strategy(id).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_create_strategy(
    State(app): State<AppState>,
    Json(req): Json<CreateStrategyRequest>,
) -> Result<Json<ApiResponse<StrategyCreatedPayload>>> {
    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("Strategy name is required".into()));
    }
    if !req.config.is_object() {
        return Err(AppError::BadRequest("Invalid strategy config".into()));
    }
    let description = req.description.trim().to_string();
    let payload = app
        .services
        .strategy_service
        .create_strategy(name, description, req.config)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_get_strategy_positions(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<StrategyPositionListPayload>>> {
    let items = app
        .services
        .trading_service
        .positions_by_strategy(&id)
        .await?;
    let count = items.len();
    Ok(Json(ApiResponse::success(
        Some(StrategyPositionListPayload {
            strategy_id: id,
            items,
            count,
        }),
        None,
    )))
}

pub async fn handle_update_strategy(
    State(app): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateStrategyRequest>,
) -> Result<Json<ApiResponse<StrategyMessagePayload>>> {
    let name = req.name.trim().to_string();
    let description = req.description.trim().to_string();
    let payload = app
        .services
        .strategy_service
        .update_strategy(id, name, description, req.config)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_delete_strategy(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<StrategyMessagePayload>>> {
    let payload = app.services.strategy_service.delete_strategy(id).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_activate_strategy(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<StrategyMessagePayload>>> {
    let payload = app.services.strategy_service.activate_strategy(id).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_duplicate_strategy(
    State(app): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<DuplicateStrategyRequest>,
) -> Result<Json<ApiResponse<StrategyCreatedPayload>>> {
    let name = req.name.trim().to_string();
    let payload = app
        .services
        .strategy_service
        .duplicate_strategy(id, name)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_get_active_strategy(
    State(app): State<AppState>,
) -> Result<Json<ApiResponse<StrategyPayload>>> {
    let payload = app.services.strategy_service.active_strategy().await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_get_default_strategy_config(
    State(app): State<AppState>,
) -> Result<Json<ApiResponse<StrategyDefaultConfigPayload>>> {
    let payload = app
        .services
        .strategy_service
        .default_strategy_config()?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_preview_prompt(
    State(app): State<AppState>,
    Json(req): Json<PreviewPromptRequest>,
) -> Result<Json<ApiResponse<PreviewPromptPayload>>> {
    if !req.config.is_object() {
        return Err(AppError::BadRequest("Invalid strategy config".into()));
    }
    let payload = app.services.strategy_service.preview_prompt(
        req.config,
        req.account_equity,
    )?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_strategy_test_run(
    State(app): State<AppState>,
    Json(req): Json<StrategyTestRunRequest>,
) -> Result<Json<ApiResponse<StrategyTestRunPayload>>> {
    if !req.config.is_object() {
        return Err(AppError::BadRequest("Invalid strategy config".into()));
    }
    let payload = app
        .services
        .strategy_service
        .test_run(
            req.config,
            req.ai_model_id,
            req.run_real_ai,
        )
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}
