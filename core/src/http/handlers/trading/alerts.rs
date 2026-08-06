use axum::{
    Json,
    extract::{Query, State},
};

use crate::{
    contracts::trading::alerts::{
        RuntimeAlertAckPayload, RuntimeAlertAckRequest, RuntimeAlertControlTargetRequest,
        RuntimeAlertControlsPayload, RuntimeAlertControlsQuery, RuntimeAlertDeliveriesPayload,
        RuntimeAlertDeliveriesQuery, RuntimeAlertHistoryPayload, RuntimeAlertHistoryQuery,
        RuntimeAlertMutePayload, RuntimeAlertMuteRequest, RuntimeAlertsPayload, RuntimeAlertsQuery,
    },
    error::Result,
    http::response::ApiResponse,
    state,
};

use super::trading_service;

pub async fn runtime_alerts(
    State(app): State<state::AppState>,
    Query(q): Query<RuntimeAlertsQuery>,
) -> Result<Json<ApiResponse<RuntimeAlertsPayload>>> {
    let payload = trading_service(&app).runtime_alerts(
        q.trader_id,
        q.window_hours,
        q.open_market_fallback_rate_max_pct,
        q.replace_throttle_rate_max_pct,
        q.stale_reconcile_terminal_rate_max_pct,
        q.persist_min_interval_secs,
    ).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn runtime_alert_history(
    State(app): State<state::AppState>,
    Query(q): Query<RuntimeAlertHistoryQuery>,
) -> Result<Json<ApiResponse<RuntimeAlertHistoryPayload>>> {
    let payload = trading_service(&app)
        .runtime_alert_history(q.trader_id, q.window_hours, q.limit, q.offset, q.breached_only, q.severity)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn runtime_alert_deliveries(
    State(app): State<state::AppState>,
    Query(q): Query<RuntimeAlertDeliveriesQuery>,
) -> Result<Json<ApiResponse<RuntimeAlertDeliveriesPayload>>> {
    let payload = trading_service(&app)
        .runtime_alert_deliveries(q.trader_id, q.window_hours, q.limit, q.offset, q.success, q.destination)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn runtime_alert_controls(
    State(app): State<state::AppState>,
    Query(q): Query<RuntimeAlertControlsQuery>,
) -> Result<Json<ApiResponse<RuntimeAlertControlsPayload>>> {
    let payload = trading_service(&app)
        .runtime_alert_controls(q.trader_id)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn mute_runtime_alerts(
    State(app): State<state::AppState>,
    Json(request): Json<RuntimeAlertMuteRequest>,
) -> Result<Json<ApiResponse<RuntimeAlertMutePayload>>> {
    let payload = trading_service(&app)
        .mute_runtime_alerts(request.trader_id, request.mute_minutes, request.mute_until, request.reason)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn unmute_runtime_alerts(
    State(app): State<state::AppState>,
    Json(request): Json<RuntimeAlertControlTargetRequest>,
) -> Result<Json<ApiResponse<RuntimeAlertMutePayload>>> {
    let payload = trading_service(&app)
        .unmute_runtime_alerts(request.trader_id)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn ack_runtime_alerts(
    State(app): State<state::AppState>,
    Json(request): Json<RuntimeAlertAckRequest>,
) -> Result<Json<ApiResponse<RuntimeAlertAckPayload>>> {
    let payload = trading_service(&app)
        .ack_runtime_alerts(request.trader_id, request.note)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}
