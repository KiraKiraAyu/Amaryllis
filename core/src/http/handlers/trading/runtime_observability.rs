use axum::{
    Json,
    extract::{Query, State},
};

use crate::{
    contracts::trading::runtime_observability::{
        RuntimeEventTypesPayload, RuntimeEventTypesQuery, RuntimeEventsPayload, RuntimeEventsQuery,
        RuntimeMetricsPayload, RuntimeMetricsQuery, RuntimeMetricsSeriesPayload,
        RuntimeMetricsSeriesQuery,
    },
    error::Result,
    http::response::ApiResponse,
    state,
};

use super::trading_service;

pub async fn runtime_events(
    State(app): State<state::AppState>,
    Query(q): Query<RuntimeEventsQuery>,
) -> Result<Json<ApiResponse<RuntimeEventsPayload>>> {
    let payload = trading_service(&app)
        .runtime_events(
            q.trader_id,
            q.window_hours,
            q.limit,
            q.offset,
            q.event_type,
            q.risk_level,
            q.correlation_id,
        )
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn runtime_event_types(
    State(app): State<state::AppState>,
    Query(q): Query<RuntimeEventTypesQuery>,
) -> Result<Json<ApiResponse<RuntimeEventTypesPayload>>> {
    let payload = trading_service(&app)
        .runtime_event_types(q.trader_id, q.window_hours)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn runtime_metrics(
    State(app): State<state::AppState>,
    Query(q): Query<RuntimeMetricsQuery>,
) -> Result<Json<ApiResponse<RuntimeMetricsPayload>>> {
    let payload = trading_service(&app)
        .runtime_metrics(q.trader_id, q.window_hours)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn runtime_metrics_series(
    State(app): State<state::AppState>,
    Query(q): Query<RuntimeMetricsSeriesQuery>,
) -> Result<Json<ApiResponse<RuntimeMetricsSeriesPayload>>> {
    let payload = trading_service(&app)
        .runtime_metrics_series(q.trader_id, q.window_hours, q.bucket_minutes)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}
