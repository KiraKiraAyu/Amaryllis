use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct RuntimeMetricsQuery {
    pub trader_id: Option<String>,
    pub window_hours: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct RuntimeMetricsSeriesQuery {
    pub trader_id: Option<String>,
    pub window_hours: Option<i64>,
    pub bucket_minutes: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct RuntimeEventsQuery {
    pub trader_id: Option<String>,
    pub window_hours: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub event_type: Option<String>,
    pub risk_level: Option<String>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RuntimeEventTypesQuery {
    pub trader_id: Option<String>,
    pub window_hours: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeEventPayload {
    pub id: String,
    pub event_type: String,
    pub symbol: String,
    pub side: String,
    pub risk_level: String,
    pub trigger_source: String,
    pub action_taken: String,
    pub correlation_id: String,
    pub payload: Value,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeEventsFilterPayload {
    pub event_type: String,
    pub risk_level: String,
    pub correlation_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeEventsPayload {
    pub trader_id: String,
    pub window_hours: i64,
    pub from_ts: i64,
    pub limit: i64,
    pub offset: i64,
    pub filters: RuntimeEventsFilterPayload,
    pub total: i64,
    pub items: Vec<RuntimeEventPayload>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeEventTypePayload {
    pub event_type: String,
    pub count: i64,
    pub description: String,
    pub canonical: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeEventTypesPayload {
    pub trader_id: String,
    pub window_hours: i64,
    pub from_ts: i64,
    pub items: Vec<RuntimeEventTypePayload>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeMetricTotalsPayload {
    pub runtime_events: i64,
    pub replace_succeeded: i64,
    pub replace_throttled: i64,
    pub replace_market_fallback: i64,
    pub open_market_fallback: i64,
    pub open_submitted: i64,
    pub stale_reconcile_terminal: i64,
    pub stale_reconcile_pending: i64,
    pub medium_risk_open_skips: i64,
    pub live_risk_snapshots: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeMetricRatesPayload {
    pub replace_throttle_rate: f64,
    pub replace_market_fallback_rate: f64,
    pub open_market_fallback_rate: f64,
    pub stale_reconcile_terminal_rate: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RiskLevelCountPayload {
    pub risk_level: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeMetricsPayload {
    pub trader_id: String,
    pub window_hours: i64,
    pub from_ts: i64,
    pub totals: RuntimeMetricTotalsPayload,
    pub rates_pct: RuntimeMetricRatesPayload,
    pub risk_level_distribution: Vec<RiskLevelCountPayload>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeMetricsSeriesBucketPayload {
    pub bucket_from_ts: i64,
    pub bucket_to_ts: i64,
    pub totals: RuntimeMetricTotalsPayload,
    pub rates_pct: RuntimeMetricRatesPayload,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeMetricsSeriesPayload {
    pub trader_id: String,
    pub window_hours: i64,
    pub from_ts: i64,
    pub bucket_minutes: i64,
    pub bucket_secs: i64,
    pub items: Vec<RuntimeMetricsSeriesBucketPayload>,
}
