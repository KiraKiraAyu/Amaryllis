use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateTraderRequest {
    pub name: String,
    pub ai_model_id: String,
    pub exchange_id: String,
    pub strategy_id: String,
    #[serde(default = "default_initial_balance")]
    pub initial_balance: f64,
    #[serde(default = "default_scan_interval")]
    pub scan_interval_minutes: i64,
    #[serde(default)]
    pub is_cross_margin: Option<bool>,
    #[serde(default)]
    pub use_ai500: bool,
    #[serde(default)]
    pub use_oi_top: bool,
    #[serde(default)]
    pub custom_prompt: String,
    #[serde(default)]
    pub override_base_prompt: bool,
    #[serde(default = "default_prompt_template")]
    pub system_prompt_template: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTraderRequest {
    pub name: Option<String>,
    pub ai_model_id: Option<String>,
    pub exchange_id: Option<String>,
    pub strategy_id: Option<String>,
    pub initial_balance: Option<f64>,
    pub scan_interval_minutes: Option<i64>,
    pub is_cross_margin: Option<bool>,
    pub use_ai500: Option<bool>,
    pub use_oi_top: Option<bool>,
    pub custom_prompt: Option<String>,
    pub override_base_prompt: Option<bool>,
    pub system_prompt_template: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePromptRequest {
    #[serde(default)]
    pub custom_prompt: String,
    #[serde(default)]
    pub override_base_prompt: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraderPayload {
    pub id: String,
    pub name: String,
    pub ai_model_id: String,
    pub exchange_id: String,
    pub strategy_id: String,
    pub initial_balance: f64,
    pub scan_interval_minutes: i64,
    pub is_running: bool,
    pub is_cross_margin: bool,
    pub use_ai500: bool,
    pub use_oi_top: bool,
    pub custom_prompt: String,
    pub override_base_prompt: bool,
    pub system_prompt_template: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraderListPayload {
    pub traders: Vec<TraderPayload>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraderCreatedPayload {
    pub id: String,
    pub message: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraderMessagePayload {
    pub message: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeEnginePayload {
    pub trader_id: String,
    pub exchange_id: String,
    pub ai_model_id: String,
    pub started_at: u64,
    pub updated_at: u64,
    pub is_running: bool,
    pub last_error: Option<String>,
    pub next_scan_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraderStatusPayload {
    pub trader_id: String,
    pub is_running: bool,
    pub open_positions: i64,
    pub open_orders: i64,
    pub last_updated: i64,
    pub runtime_engine: Option<RuntimeEnginePayload>,
}

fn default_initial_balance() -> f64 {
    1000.0
}

fn default_scan_interval() -> i64 {
    3
}

fn default_prompt_template() -> String {
    "default".to_string()
}
