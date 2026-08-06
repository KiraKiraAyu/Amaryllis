use axum::{
    Json,
    extract::{Query, State},
};

use crate::{
    contracts::trading::{
        common::{PaginationQuery, TraderQuery},
        history::{
            DecisionListPayload, DecisionQuery, LatestDecisionsPayload, StatisticsQuery,
            TradeListPayload, TraderStatisticsPayload,
        },
    },
    error::Result,
    http::response::ApiResponse,
    state,
};

use super::trading_service;

pub async fn decisions(
    State(app): State<state::AppState>,
    Query(q): Query<DecisionQuery>,
) -> Result<Json<ApiResponse<DecisionListPayload>>> {
    let payload = trading_service(&app).decisions(q.trader_id, q.limit, q.offset, q.symbol).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn latest_decisions(
    State(app): State<state::AppState>,
    Query(q): Query<TraderQuery>,
) -> Result<Json<ApiResponse<LatestDecisionsPayload>>> {
    let payload = trading_service(&app).latest_decisions(q.trader_id).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn trades(
    State(app): State<state::AppState>,
    Query(q): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<TradeListPayload>>> {
    let payload = trading_service(&app).trades(q.trader_id, q.limit, q.offset).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn statistics(
    State(app): State<state::AppState>,
    Query(q): Query<StatisticsQuery>,
) -> Result<Json<ApiResponse<TraderStatisticsPayload>>> {
    let payload = trading_service(&app).statistics(q.trader_id, q.days).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}
