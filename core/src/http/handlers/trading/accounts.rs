use axum::{
    Json,
    extract::{Path, Query, State},
};

use crate::{
    contracts::trading::{
        accounts::{
            ClosePositionPayload, ClosePositionRequest, GridRiskInfoPayload, TraderAccountPayload,
            TraderBalanceSyncPayload,
        },
        common::{PaginationQuery, TraderQuery},
        positions::{PositionListPayload, PositionQuery},
    },
    error::Result,
    http::response::ApiResponse,
    state,
};

use super::trading_service;

pub async fn sync_balance(
    State(app): State<state::AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<TraderBalanceSyncPayload>>> {
    let payload = trading_service(&app).sync_balance(&id).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn close_position(
    State(app): State<state::AppState>,
    Path(id): Path<String>,
    Json(request): Json<ClosePositionRequest>,
) -> Result<Json<ApiResponse<ClosePositionPayload>>> {
    let symbol = request.symbol.trim().to_string();
    let side = request.side.trim().to_string();
    let local_only = request.local_only;
    if symbol.is_empty() || (side != "LONG" && side != "SHORT") {
        return Err(crate::error::AppError::BadRequest(
            "symbol and side(LONG/SHORT) are required".into(),
        ));
    }
    let payload = trading_service(&app)
        .close_position(&id, &symbol, &side, local_only)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn grid_risk(
    State(app): State<state::AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<GridRiskInfoPayload>>> {
    let payload = trading_service(&app).grid_risk_info(&id).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn account(
    State(app): State<state::AppState>,
    Query(q): Query<TraderQuery>,
) -> Result<Json<ApiResponse<TraderAccountPayload>>> {
    let payload = trading_service(&app).account(q.trader_id).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn positions(
    State(app): State<state::AppState>,
    Query(q): Query<PositionQuery>,
) -> Result<Json<ApiResponse<PositionListPayload>>> {
    let payload = trading_service(&app)
        .positions(q.trader_id, q.status)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn positions_history(
    State(app): State<state::AppState>,
    Query(q): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<PositionListPayload>>> {
    let payload = trading_service(&app)
        .positions_history(q.trader_id, q.limit, q.offset)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}
