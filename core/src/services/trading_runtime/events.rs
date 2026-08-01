use super::service::*;
use crate::repositories::trading::records::runtime_observability::InsertRuntimeEventRecord;

pub async fn emit_runtime_event(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    event_type: &str,
    symbol: &str,
    side: &str,
    risk_level: &str,
    trigger_source: &str,
    action_taken: &str,
    correlation_id: &str,
    payload: serde_json::Value,
    ts: i64,
) -> Result<(), AppError> {
    state
        .trading_repo
        .insert_runtime_event(InsertRuntimeEventRecord {
            id: Uuid::now_v7().to_string(),
            trader_id: cfg.trader_id.clone(),
            event_type: event_type.trim().to_string(),
            symbol: symbol.trim().to_uppercase(),
            side: side.trim().to_uppercase(),
            risk_level: risk_level.trim().to_ascii_lowercase(),
            trigger_source: trigger_source.trim().to_string(),
            action_taken: action_taken.trim().to_string(),
            correlation_id: correlation_id.trim().to_string(),
            payload_json: payload.to_string(),
            created_at: ts,
        })
        .await?;

    Ok(())
}

pub async fn emit_runtime_event_best_effort(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    event_type: &str,
    symbol: &str,
    side: &str,
    risk_level: &str,
    trigger_source: &str,
    action_taken: &str,
    correlation_id: &str,
    payload: serde_json::Value,
    ts: i64,
) {
    if let Err(err) = emit_runtime_event(
        state,
        cfg,
        event_type,
        symbol,
        side,
        risk_level,
        trigger_source,
        action_taken,
        correlation_id,
        payload,
        ts,
    )
    .await
    {
        warn!(
            "runtime event emit failed trader={} event_type={} symbol={} side={} correlation_id={} err={}",
            cfg.trader_id, event_type, symbol, side, correlation_id, err
        );
    }
}
