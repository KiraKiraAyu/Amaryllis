use axum::{
    Json,
    extract::{Query, State},
};

use crate::{
    contracts::backtest::{
        BacktestDecisionsPayload, BacktestEquityPayload, BacktestExportPayload,
        BacktestLabelRequest, BacktestMessagePayload, BacktestMetricsPayload, BacktestQueryParams,
        BacktestRunActionPayload, BacktestRunIdRequest, BacktestRunsPayload, BacktestStartRequest,
        BacktestStatusPayload, BacktestTracePayload, BacktestTradesPayload, KlinePayload,
        KlinesQuery,
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
        run_id,
        symbols,
        start_ts,
        end_ts,
        initial_balance,
        fee_bps,
        slippage_bps,
        ai_model_id,
        prompt_variant,
        leverage,
        interval,
        decision_every,
    } = request;

    if let Some(ref id) = run_id {
        if id.trim().is_empty() {
            return Err(AppError::BadRequest("run_id must not be empty".into()));
        }
    }
    if let Some(ref syms) = symbols {
        if syms.is_empty() {
            return Err(AppError::BadRequest("symbols must not be empty".into()));
        }
    }
    if let Some(ref model_id) = ai_model_id {
        if model_id.trim().is_empty() {
            return Err(AppError::BadRequest("ai_model_id must not be empty".into()));
        }
    }
    if let Some(ref variant) = prompt_variant {
        if variant.trim().is_empty() {
            return Err(AppError::BadRequest("prompt_variant must not be empty".into()));
        }
    }
    if let Some(ref iv) = interval {
        if iv.trim().is_empty() {
            return Err(AppError::BadRequest("interval must not be empty".into()));
        }
    }
    if let Some(l) = leverage {
        if l <= 0 {
            return Err(AppError::BadRequest("leverage must be positive".into()));
        }
    }
    if let Some(b) = initial_balance {
        if b <= 0.0 {
            return Err(AppError::BadRequest("initial_balance must be positive".into()));
        }
    }
    if let Some(f) = fee_bps {
        if f < 0.0 {
            return Err(AppError::BadRequest("fee_bps must be non-negative".into()));
        }
    }
    if let Some(s) = slippage_bps {
        if s < 0.0 {
            return Err(AppError::BadRequest("slippage_bps must be non-negative".into()));
        }
    }
    if let Some(d) = decision_every {
        if d == 0 {
            return Err(AppError::BadRequest("decision_every must be positive".into()));
        }
    }

    let payload = app
        .services
        .backtest_service
        .start(
            run_id,
            symbols,
            start_ts,
            end_ts,
            initial_balance,
            fee_bps,
            slippage_bps,
            ai_model_id,
            prompt_variant,
            leverage,
            interval,
            decision_every,
        )
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_backtest_pause(
    State(app): State<state::AppState>,
    Json(request): Json<BacktestRunIdRequest>,
) -> Result<Json<ApiResponse<BacktestRunActionPayload>>> {
    let run_id = request.run_id;
    if run_id.trim().is_empty() {
        return Err(AppError::BadRequest("run_id must not be empty".into()));
    }
    let payload = app.services.backtest_service.pause(run_id);
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_backtest_resume(
    State(app): State<state::AppState>,
    Json(request): Json<BacktestRunIdRequest>,
) -> Result<Json<ApiResponse<BacktestRunActionPayload>>> {
    let run_id = request.run_id;
    if run_id.trim().is_empty() {
        return Err(AppError::BadRequest("run_id must not be empty".into()));
    }
    let payload = app.services.backtest_service.resume(run_id);
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

pub async fn handle_backtest_label(
    State(app): State<state::AppState>,
    Json(request): Json<BacktestLabelRequest>,
) -> Result<Json<ApiResponse<BacktestMessagePayload>>> {
    let BacktestLabelRequest { run_id, label } = request;
    if run_id.trim().is_empty() {
        return Err(AppError::BadRequest("run_id must not be empty".into()));
    }
    if label.trim().is_empty() {
        return Err(AppError::BadRequest("label must not be empty".into()));
    }
    let payload = app.services.backtest_service.label(run_id, label).await?;
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

pub async fn handle_backtest_status(
    State(app): State<state::AppState>,
    Query(q): Query<BacktestQueryParams>,
) -> Result<Json<ApiResponse<BacktestStatusPayload>>> {
    let BacktestQueryParams { run_id, limit } = q;
    if let Some(ref id) = run_id {
        if id.trim().is_empty() {
            return Err(AppError::BadRequest("run_id must not be empty".into()));
        }
    }
    if let Some(l) = limit {
        if l <= 0 {
            return Err(AppError::BadRequest("limit must be positive".into()));
        }
    }
    let payload = app.services.backtest_service.status(run_id, limit).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_backtest_runs(
    State(app): State<state::AppState>,
    Query(q): Query<BacktestQueryParams>,
) -> Result<Json<ApiResponse<BacktestRunsPayload>>> {
    let BacktestQueryParams { run_id, limit } = q;
    if let Some(ref id) = run_id {
        if id.trim().is_empty() {
            return Err(AppError::BadRequest("run_id must not be empty".into()));
        }
    }
    if let Some(l) = limit {
        if l <= 0 {
            return Err(AppError::BadRequest("limit must be positive".into()));
        }
    }
    let payload = app.services.backtest_service.runs(run_id, limit).await;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_backtest_equity(
    State(app): State<state::AppState>,
    Query(q): Query<BacktestQueryParams>,
) -> Result<Json<ApiResponse<BacktestEquityPayload>>> {
    let BacktestQueryParams { run_id, limit } = q;
    if let Some(ref id) = run_id {
        if id.trim().is_empty() {
            return Err(AppError::BadRequest("run_id must not be empty".into()));
        }
    }
    if let Some(l) = limit {
        if l <= 0 {
            return Err(AppError::BadRequest("limit must be positive".into()));
        }
    }
    let payload = app.services.backtest_service.equity(run_id, limit).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_backtest_trades(
    State(app): State<state::AppState>,
    Query(q): Query<BacktestQueryParams>,
) -> Result<Json<ApiResponse<BacktestTradesPayload>>> {
    let BacktestQueryParams { run_id, limit } = q;
    if let Some(ref id) = run_id {
        if id.trim().is_empty() {
            return Err(AppError::BadRequest("run_id must not be empty".into()));
        }
    }
    if let Some(l) = limit {
        if l <= 0 {
            return Err(AppError::BadRequest("limit must be positive".into()));
        }
    }
    let payload = app.services.backtest_service.trades(run_id, limit).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_backtest_metrics(
    State(app): State<state::AppState>,
    Query(q): Query<BacktestQueryParams>,
) -> Result<Json<ApiResponse<BacktestMetricsPayload>>> {
    let BacktestQueryParams { run_id, limit } = q;
    if let Some(ref id) = run_id {
        if id.trim().is_empty() {
            return Err(AppError::BadRequest("run_id must not be empty".into()));
        }
    }
    if let Some(l) = limit {
        if l <= 0 {
            return Err(AppError::BadRequest("limit must be positive".into()));
        }
    }
    let payload = app.services.backtest_service.metrics(run_id, limit).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_backtest_trace(
    State(app): State<state::AppState>,
    Query(q): Query<BacktestQueryParams>,
) -> Result<Json<ApiResponse<BacktestTracePayload>>> {
    let BacktestQueryParams { run_id, limit } = q;
    if let Some(ref id) = run_id {
        if id.trim().is_empty() {
            return Err(AppError::BadRequest("run_id must not be empty".into()));
        }
    }
    if let Some(l) = limit {
        if l <= 0 {
            return Err(AppError::BadRequest("limit must be positive".into()));
        }
    }
    let payload = app.services.backtest_service.trace(run_id, limit).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_backtest_decisions(
    State(app): State<state::AppState>,
    Query(q): Query<BacktestQueryParams>,
) -> Result<Json<ApiResponse<BacktestDecisionsPayload>>> {
    let BacktestQueryParams { run_id, limit } = q;
    if let Some(ref id) = run_id {
        if id.trim().is_empty() {
            return Err(AppError::BadRequest("run_id must not be empty".into()));
        }
    }
    if let Some(l) = limit {
        if l <= 0 {
            return Err(AppError::BadRequest("limit must be positive".into()));
        }
    }
    let payload = app
        .services
        .backtest_service
        .decisions(run_id, limit)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_backtest_export(
    State(app): State<state::AppState>,
    Query(q): Query<BacktestQueryParams>,
) -> Result<Json<ApiResponse<BacktestExportPayload>>> {
    let BacktestQueryParams { run_id, limit } = q;
    if let Some(ref id) = run_id {
        if id.trim().is_empty() {
            return Err(AppError::BadRequest("run_id must not be empty".into()));
        }
    }
    if let Some(l) = limit {
        if l <= 0 {
            return Err(AppError::BadRequest("limit must be positive".into()));
        }
    }
    let payload = app.services.backtest_service.export(run_id, limit).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_backtest_klines(
    State(app): State<state::AppState>,
    Query(q): Query<KlinesQuery>,
) -> Result<Json<ApiResponse<Vec<KlinePayload>>>> {
    let KlinesQuery {
        symbol,
        interval,
        limit,
    } = q;
    if symbol.trim().is_empty() {
        return Err(AppError::BadRequest("symbol must not be empty".into()));
    }
    if let Some(ref iv) = interval {
        if iv.trim().is_empty() {
            return Err(AppError::BadRequest("interval must not be empty".into()));
        }
    }
    if let Some(l) = limit {
        if l <= 0 {
            return Err(AppError::BadRequest("limit must be positive".into()));
        }
    }
    let payload = app
        .services
        .backtest_service
        .klines(symbol, interval, limit)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}
