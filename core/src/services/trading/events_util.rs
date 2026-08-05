use super::service::*;

pub async fn count_runtime_events(
    app: &SharedState,
    trader_id: &str,
    event_type: Option<&str>,
    action_taken: Option<&str>,
    from_ts: i64,
) -> i64 {
    app.trading_repo
        .count_runtime_events(trader_id, event_type, action_taken, from_ts)
        .await
        .unwrap_or(0)
}

pub fn app_error(kind: AppErrorKind, message: &str) -> AppError {
    AppError::from_kind(kind, message)
}

pub fn runtime_engine_payload(value: &RuntimeEngineState) -> RuntimeEnginePayload {
    RuntimeEnginePayload {
        trader_id: value.trader_id.clone(),
        exchange_id: value.exchange_id.clone(),
        ai_model_id: value.ai_model_id.clone(),
        started_at: value.started_at,
        updated_at: value.updated_at,
        is_running: value.is_running,
        last_error: value.last_error.clone(),
    }
}

pub fn runtime_metric_totals_payload(
    runtime_events: i64,
    replace_succeeded: i64,
    replace_throttled: i64,
    replace_market_fallback: i64,
    open_market_fallback: i64,
    open_submitted: i64,
    stale_reconcile_terminal: i64,
    stale_reconcile_pending: i64,
    medium_risk_open_skips: i64,
    live_risk_snapshots: i64,
) -> RuntimeMetricTotalsPayload {
    RuntimeMetricTotalsPayload {
        runtime_events,
        replace_succeeded,
        replace_throttled,
        replace_market_fallback,
        open_market_fallback,
        open_submitted,
        stale_reconcile_terminal,
        stale_reconcile_pending,
        medium_risk_open_skips,
        live_risk_snapshots,
    }
}

pub fn runtime_metric_rates_payload(
    replace_throttle_rate: f64,
    replace_market_fallback_rate: f64,
    open_market_fallback_rate: f64,
    stale_reconcile_terminal_rate: f64,
) -> RuntimeMetricRatesPayload {
    RuntimeMetricRatesPayload {
        replace_throttle_rate,
        replace_market_fallback_rate,
        open_market_fallback_rate,
        stale_reconcile_terminal_rate,
    }
}

pub fn runtime_alert_thresholds_payload(
    open_market_fallback_rate_max: f64,
    replace_throttle_rate_max: f64,
    stale_reconcile_terminal_rate_max: f64,
) -> RuntimeAlertThresholdsPayload {
    RuntimeAlertThresholdsPayload {
        open_market_fallback_rate_max,
        replace_throttle_rate_max,
        stale_reconcile_terminal_rate_max,
    }
}

pub fn runtime_alert_rates_payload(
    open_market_fallback_rate: f64,
    replace_throttle_rate: f64,
    stale_reconcile_terminal_rate: f64,
) -> RuntimeAlertRatesPayload {
    RuntimeAlertRatesPayload {
        open_market_fallback_rate,
        replace_throttle_rate,
        stale_reconcile_terminal_rate,
    }
}

pub fn runtime_alert_items_payload(
    thresholds: &RuntimeAlertThresholdsPayload,
    rates: &RuntimeAlertRatesPayload,
) -> Vec<RuntimeAlertItemPayload> {
    vec![
        RuntimeAlertItemPayload {
            key: "open_market_fallback_rate".to_string(),
            label: "Open market fallback rate".to_string(),
            rate_pct: rates.open_market_fallback_rate,
            max_pct: thresholds.open_market_fallback_rate_max,
            breached: rates.open_market_fallback_rate > thresholds.open_market_fallback_rate_max,
        },
        RuntimeAlertItemPayload {
            key: "replace_throttle_rate".to_string(),
            label: "Replace throttle rate".to_string(),
            rate_pct: rates.replace_throttle_rate,
            max_pct: thresholds.replace_throttle_rate_max,
            breached: rates.replace_throttle_rate > thresholds.replace_throttle_rate_max,
        },
        RuntimeAlertItemPayload {
            key: "stale_reconcile_terminal_rate".to_string(),
            label: "Stale reconcile terminal rate".to_string(),
            rate_pct: rates.stale_reconcile_terminal_rate,
            max_pct: thresholds.stale_reconcile_terminal_rate_max,
            breached: rates.stale_reconcile_terminal_rate
                > thresholds.stale_reconcile_terminal_rate_max,
        },
    ]
}

pub fn runtime_alert_totals_payload(
    replace_succeeded: i64,
    replace_throttled: i64,
    open_market_fallback: i64,
    open_submitted: i64,
    stale_reconcile_terminal: i64,
    stale_reconcile_pending: i64,
) -> RuntimeAlertTotalsPayload {
    RuntimeAlertTotalsPayload {
        replace_succeeded,
        replace_throttled,
        open_market_fallback,
        open_submitted,
        stale_reconcile_terminal,
        stale_reconcile_pending,
    }
}

pub fn runtime_alert_controls_payload(
    trader_id: String,
    is_muted: bool,
    muted_until: i64,
    mute_reason: String,
    acked_at: i64,
    acked_by: String,
    ack_note: String,
    updated_at: i64,
    created_at: i64,
) -> RuntimeAlertControlsPayload {
    RuntimeAlertControlsPayload {
        trader_id,
        is_muted,
        muted_until,
        mute_reason,
        acked_at,
        acked_by,
        ack_note,
        updated_at,
        created_at,
    }
}

pub fn empty_runtime_alert_controls_payload(trader_id: String) -> RuntimeAlertControlsPayload {
    runtime_alert_controls_payload(
        trader_id,
        false,
        0,
        String::new(),
        0,
        String::new(),
        String::new(),
        0,
        0,
    )
}

pub fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn is_valid_leverage(v: i64) -> bool {
    (1..=200).contains(&v)
}
