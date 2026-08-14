use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct CreateStrategyRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub config: Value,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStrategyRequest {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub config: Value,
}

#[derive(Debug, Deserialize)]
pub struct DuplicateStrategyRequest {
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct PreviewPromptRequest {
    pub config: Value,
    pub account_equity: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct StrategyTestRunRequest {
    pub config: Value,
    pub ai_model_id: Option<String>,
    pub run_real_ai: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StrategyPayload {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author_email: String,
    pub is_active: bool,
    pub config: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StrategyListPayload {
    pub strategies: Vec<StrategyPayload>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct StrategyCreatedPayload {
    pub id: String,
    pub message: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct StrategyMessagePayload {
    pub message: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct StrategyDefaultConfigPayload {
    pub config: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreviewPromptPayload {
    pub system_prompt: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StrategyTestRunPayload {
    pub system_prompt: String,
    pub user_prompt: String,
    pub ai_model_id: String,
    pub ai_response: String,
    pub decisions: Value,
    pub reasoning: String,
    pub duration_ms: u64,
    pub used_real_ai: bool,
}
