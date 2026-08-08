use axum::{
    Json,
    extract::{Query, State},
};

use crate::{
    contracts::backtest::{
        BacktestMessagePayload, BacktestQueryParams, BacktestRunActionPayload,
        BacktestRunIdRequest, BacktestRunsPayload, BacktestStartRequest,
    },
    error::{AppError, Result},
    http::response::ApiResponse,
    state,
};

pub async fn handle_backtest_start(
    State(app): State<state::AppState>,
    Json(request): Json<BacktestStartRequest>,
) -> Result<Json<ApiResponse<BacktestRunActionPayload>>> {
    let BacktestStartRequest {
        trader_id,
        run_id,
        start_ts,
        end_ts,
        initial_balance,
        interval,
    } = request;

    if trader_id.trim().is_empty() {
        return Err(AppError::BadRequest("trader_id must not be empty".into()));
    }
    if let Some(ref id) = run_id {
        if id.trim().is_empty() {
            return Err(AppError::BadRequest("run_id must not be empty".into()));
        }
    }
    if let Some(ref iv) = interval {
        if iv.trim().is_empty() {
            return Err(AppError::BadRequest("interval must not be empty".into()));
        }
    }
    if let Some(b) = initial_balance {
        if b <= 0.0 {
            return Err(AppError::BadRequest("initial_balance must be positive".into()));
        }
    }

    let payload = app
        .services
        .backtest_service
        .start(
            &trader_id,
            run_id,
            start_ts,
            end_ts,
            initial_balance,
            interval,
        )
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_backtest_stop(
    State(app): State<state::AppState>,
    Json(request): Json<BacktestRunIdRequest>,
) -> Result<Json<ApiResponse<BacktestRunActionPayload>>> {
    let run_id = request.run_id;
    if run_id.trim().is_empty() {
        return Err(AppError::BadRequest("run_id must not be empty".into()));
    }
    let payload = app.services.backtest_service.stop(run_id)?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_backtest_delete(
    State(app): State<state::AppState>,
    Json(request): Json<BacktestRunIdRequest>,
) -> Result<Json<ApiResponse<BacktestMessagePayload>>> {
    let run_id = request.run_id;
    if run_id.trim().is_empty() {
        return Err(AppError::BadRequest("run_id must not be empty".into()));
    }
    let payload = app.services.backtest_service.delete(run_id).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_backtest_runs(
    State(app): State<state::AppState>,
    Query(q): Query<BacktestQueryParams>,
) -> Result<Json<ApiResponse<BacktestRunsPayload>>> {
    let BacktestQueryParams { limit } = q;
    if let Some(l) = limit {
        if l <= 0 {
            return Err(AppError::BadRequest("limit must be positive".into()));
        }
    }
    let payload = app.services.backtest_service.runs(limit).await;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}
