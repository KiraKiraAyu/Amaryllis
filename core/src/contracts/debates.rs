use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct CreateDebateRequest {
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub max_rounds: Option<i64>,
    pub prompt_variant: Option<String>,
    pub participants: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct StartDebateRequest {}

#[derive(Debug, Clone, Serialize)]
pub struct DebateListPayload {
    pub debates: Vec<Value>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DebateDetailPayload {
    pub debate: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct DebateActionPayload {
    pub id: String,
    pub message: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct DebateMessagePayload {
    pub message: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct DebatePersonalityPayload {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct DebatePersonalitiesPayload {
    pub personalities: Vec<DebatePersonalityPayload>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DebateExecutionPayload {
    pub id: String,
    pub final_decision: String,
    pub final_reasoning: String,
    pub message: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct DebateMessagesPayload {
    pub messages: Vec<Value>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DebateVotesPayload {
    pub votes: Value,
}
