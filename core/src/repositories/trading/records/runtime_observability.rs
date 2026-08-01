#[derive(Debug, Clone)]
pub struct RuntimeEventRecord {
    pub id: String,
    pub event_type: String,
    pub symbol: String,
    pub side: String,
    pub risk_level: String,
    pub trigger_source: String,
    pub action_taken: String,
    pub correlation_id: String,
    pub payload_json: String,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct InsertRuntimeEventRecord {
    pub id: String,
    pub trader_id: String,
    pub event_type: String,
    pub symbol: String,
    pub side: String,
    pub risk_level: String,
    pub trigger_source: String,
    pub action_taken: String,
    pub correlation_id: String,
    pub payload_json: String,
    pub created_at: i64,
}
