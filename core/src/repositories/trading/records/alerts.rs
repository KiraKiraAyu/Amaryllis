#[derive(Debug, Clone)]
pub struct RuntimeAlertControlsRecord {
    pub trader_id: String,
    pub is_muted: bool,
    pub muted_until: i64,
    pub mute_reason: String,
    pub acked_at: i64,
    pub acked_by: String,
    pub ack_note: String,
    pub updated_at: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct RuntimeAlertHistoryRecord {
    pub id: String,
    pub window_hours: i32,
    pub thresholds_json: String,
    pub rates_json: String,
    pub alerts_json: String,
    pub breached: bool,
    pub severity: String,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct RuntimeAlertDeliveryRecord {
    pub id: String,
    pub alert_history_id: String,
    pub destination: String,
    pub endpoint: String,
    pub response_status: i32,
    pub response_body: String,
    pub attempt: i32,
    pub max_attempts: i32,
    pub success: bool,
    pub error_message: String,
    pub latency_ms: i32,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct InsertRuntimeAlertHistoryRecord {
    pub id: String,
    pub trader_id: String,
    pub window_hours: i64,
    pub thresholds_json: String,
    pub rates_json: String,
    pub alerts_json: String,
    pub breached: bool,
    pub severity: String,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct InsertRuntimeAlertDeliveryRecord {
    pub id: String,
    pub trader_id: String,
    pub alert_history_id: String,
    pub destination: String,
    pub endpoint: String,
    pub request_headers_json: String,
    pub request_body_json: String,
    pub response_status: i64,
    pub response_body: String,
    pub attempt: i64,
    pub max_attempts: i64,
    pub success: bool,
    pub error_message: String,
    pub latency_ms: i64,
    pub created_at: i64,
}
