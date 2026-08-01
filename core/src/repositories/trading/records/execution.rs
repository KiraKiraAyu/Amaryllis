#[derive(Debug, Clone)]
pub struct ExecutionIntentRecord {
    pub id: String,
    pub intent_key: String,
    pub symbol: String,
    pub side: String,
    pub decision: String,
    pub status: String,
    pub exchange_order_id: String,
    pub payload_json: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct InsertExecutionIntentRecord {
    pub id: String,
    pub trader_id: String,
    pub intent_key: String,
    pub symbol: String,
    pub side: String,
    pub decision: String,
    pub status: String,
    pub exchange_order_id: String,
    pub payload_json: String,
    pub created_at: i64,
    pub updated_at: i64,
}
