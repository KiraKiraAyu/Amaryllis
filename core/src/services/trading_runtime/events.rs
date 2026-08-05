use super::service::*;
use crate::{
    contracts::trading::runtime_observability::RuntimeEventPayload,
    repositories::trading::records::runtime_observability::InsertRuntimeEventRecord,
};

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
    let event = RuntimeEventPayload {
        id: Uuid::now_v7().to_string(),
        event_type: event_type.trim().to_string(),
        symbol: symbol.trim().to_uppercase(),
        side: side.trim().to_uppercase(),
        risk_level: risk_level.trim().to_ascii_lowercase(),
        trigger_source: trigger_source.trim().to_string(),
        action_taken: action_taken.trim().to_string(),
        correlation_id: correlation_id.trim().to_string(),
        payload,
        created_at: ts,
    };

    state
        .trading_repo
        .insert_runtime_event(InsertRuntimeEventRecord {
            id: event.id.clone(),
            trader_id: cfg.trader_id.clone(),
            event_type: event.event_type.clone(),
            symbol: event.symbol.clone(),
            side: event.side.clone(),
            risk_level: event.risk_level.clone(),
            trigger_source: event.trigger_source.clone(),
            action_taken: event.action_taken.clone(),
            correlation_id: event.correlation_id.clone(),
            payload_json: event.payload.to_string(),
            created_at: ts,
        })
        .await?;

    state
        .realtime_hub
        .publish(crate::realtime::RealtimeEvent::RuntimeEvent {
            trader_id: cfg.trader_id.clone(),
            event,
        });

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
