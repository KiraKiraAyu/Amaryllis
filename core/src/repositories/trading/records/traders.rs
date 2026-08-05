#[derive(Debug, Clone)]
pub struct TraderRecord {
    pub id: String,
    pub name: String,
    pub ai_model_id: String,
    pub exchange_id: String,
    pub strategy_id: String,
    pub initial_balance: f64,
    pub scan_interval_minutes: i64,
    pub is_running: i64,
    pub is_cross_margin: i64,
    pub use_ai500: i64,
    pub use_oi_top: i64,
    pub custom_prompt: String,
    pub override_base_prompt: i64,
    pub system_prompt_template: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct CreateTraderRecord {
    pub id: String,
    pub snapshot_id: String,
    pub name: String,
    pub ai_model_id: String,
    pub exchange_id: String,
    pub strategy_id: String,
    pub initial_balance: f64,
    pub scan_interval_minutes: i64,
    pub is_cross_margin: bool,
    pub use_ai500: bool,
    pub use_oi_top: bool,
    pub custom_prompt: String,
    pub override_base_prompt: bool,
    pub system_prompt_template: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct UpdateTraderRecord {
    pub name: String,
    pub ai_model_id: String,
    pub exchange_id: String,
    pub strategy_id: String,
    pub initial_balance: f64,
    pub scan_interval_minutes: i64,
    pub is_cross_margin: bool,
    pub use_ai500: bool,
    pub use_oi_top: bool,
    pub custom_prompt: String,
    pub override_base_prompt: bool,
    pub system_prompt_template: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct EquityHistoryPointRecord {
    pub timestamp: i64,
    pub total_equity: f64,
    pub available_balance: f64,
    pub total_pnl: f64,
    pub total_pnl_pct: f64,
    pub position_count: i64,
    pub margin_used_pct: f64,
    pub balance: f64,
}
