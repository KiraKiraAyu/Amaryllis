use axum::{
    Json,
    extract::{Path, Query, State},
};

use crate::{
    contracts::public::{EquityHistoryPointPayload, EquityHistoryQuery},
    contracts::trading::{
        common::TraderQuery,
        traders::{
            CreateTraderRequest, TraderCreatedPayload, TraderListPayload, TraderMessagePayload,
            TraderPayload, TraderStatusPayload, UpdatePromptRequest, UpdateTraderRequest,
        },
    },
    error::Result,
    http::response::ApiResponse,
    state,
};

use super::trading_service;

pub async fn list(
    State(app): State<state::AppState>,
) -> Result<Json<ApiResponse<TraderListPayload>>> {
    let payload = trading_service(&app).list_traders().await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn get(
    State(app): State<state::AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<TraderPayload>>> {
    let payload = trading_service(&app).get_trader(&id).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn config(
    State(app): State<state::AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<TraderPayload>>> {
    let payload = trading_service(&app)
        .get_trader_config(&id)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn create(
    State(app): State<state::AppState>,
    Json(request): Json<CreateTraderRequest>,
) -> Result<Json<ApiResponse<TraderCreatedPayload>>> {
    let name = request.name.trim().to_string();
    let ai_model_id = request.ai_model_id.trim().to_string();
    let exchange_id = request.exchange_id.trim().to_string();
    if name.is_empty() || ai_model_id.is_empty() || exchange_id.is_empty() {
        return Err(crate::error::AppError::BadRequest(
            "name, ai_model_id, exchange_id are required".into(),
        ));
    }
    let payload = trading_service(&app)
        .create_trader(
            &name,
            &ai_model_id,
            &exchange_id,
            &request.strategy_id,
            request.initial_balance,
            request.scan_interval_minutes,
            request.is_cross_margin,
            request.use_ai500,
            request.use_oi_top,
            &request.custom_prompt,
            request.override_base_prompt,
            &request.system_prompt_template,
        )
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn update(
    State(app): State<state::AppState>,
    Path(id): Path<String>,
    Json(request): Json<UpdateTraderRequest>,
) -> Result<Json<ApiResponse<TraderMessagePayload>>> {
    let payload = trading_service(&app)
        .update_trader(
            &id,
            request.name,
            request.ai_model_id,
            request.exchange_id,
            request.strategy_id,
            request.initial_balance,
            request.scan_interval_minutes,
            request.is_cross_margin,
            request.use_ai500,
            request.use_oi_top,
            request.custom_prompt,
            request.override_base_prompt,
            request.system_prompt_template,
        )
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn delete(
    State(app): State<state::AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<TraderMessagePayload>>> {
    let payload = trading_service(&app).delete_trader(&id).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn start(
    State(app): State<state::AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<TraderMessagePayload>>> {
    let payload = trading_service(&app).start_trader(&id).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn stop(
    State(app): State<state::AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<TraderMessagePayload>>> {
    let payload = trading_service(&app).stop_trader(&id).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn update_prompt(
    State(app): State<state::AppState>,
    Path(id): Path<String>,
    Json(request): Json<UpdatePromptRequest>,
) -> Result<Json<ApiResponse<TraderMessagePayload>>> {
    let payload = trading_service(&app)
        .update_trader_prompt(&id, &request.custom_prompt, request.override_base_prompt)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn equity_history(
    State(app): State<state::AppState>,
    Query(q): Query<EquityHistoryQuery>,
) -> Result<Json<ApiResponse<Vec<EquityHistoryPointPayload>>>> {
    let payload = trading_service(&app).equity_history(q.trader_id).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn status(
    State(app): State<state::AppState>,
    Query(q): Query<TraderQuery>,
) -> Result<Json<ApiResponse<TraderStatusPayload>>> {
    let payload = trading_service(&app).status(q.trader_id).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}
