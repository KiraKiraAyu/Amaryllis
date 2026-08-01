use std::sync::{Arc, RwLock};

use super::config_loaders::now_u64;
use crate::{
    error::AppError,
    realtime::RealtimeHub,
    repositories::{ExchangeRepo, TradingRepo},
    services::llm::LlmService,
    state::{RuntimeEngineManager, RuntimeEngineState},
};

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub live: crate::config::LiveRuntimeConfig,
}

#[derive(Debug, Clone)]
pub struct RuntimeState {
    pub trading_repo: Arc<TradingRepo>,
    pub exchange_repo: Arc<ExchangeRepo>,
    pub config: RuntimeConfig,
    pub runtime_engine_manager: Arc<RwLock<RuntimeEngineManager>>,
    pub llm_service: Arc<LlmService>,
    pub realtime_hub: RealtimeHub,
}

pub type SharedState = RuntimeState;

impl RuntimeState {
    pub fn new(
        trading_repo: Arc<TradingRepo>,
        exchange_repo: Arc<ExchangeRepo>,
        live: crate::config::LiveRuntimeConfig,
        runtime_engine_manager: Arc<RwLock<RuntimeEngineManager>>,
        llm_service: Arc<LlmService>,
        realtime_hub: RealtimeHub,
    ) -> Self {
        Self {
            trading_repo,
            exchange_repo,
            config: RuntimeConfig { live },
            runtime_engine_manager,
            llm_service,
            realtime_hub,
        }
    }

    pub fn upsert_runtime_engine(&self, engine: RuntimeEngineState) -> Result<(), AppError> {
        let mut manager = self
            .runtime_engine_manager
            .write()
            .map_err(|_| AppError::Internal("Concurrent state access failed".into()))?;
        manager.upsert(engine);
        Ok(())
    }

    pub fn runtime_engine(&self, trader_id: &str) -> Result<Option<RuntimeEngineState>, AppError> {
        let manager = self
            .runtime_engine_manager
            .read()
            .map_err(|_| AppError::Internal("Concurrent state access failed".into()))?;
        Ok(manager.get(trader_id))
    }

    pub fn set_runtime_engine_running(
        &self,
        trader_id: &str,
        is_running: bool,
        last_error: Option<String>,
    ) -> Result<(), AppError> {
        let mut manager = self
            .runtime_engine_manager
            .write()
            .map_err(|_| AppError::Internal("Concurrent state access failed".into()))?;
        let updated = manager.set_running(trader_id, is_running, now_u64(), last_error);
        if updated {
            Ok(())
        } else {
            Err(AppError::NotFound("Runtime engine not found".into()))
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SymbolConfig {
    pub symbol: String,
    pub leverage: i64,
    pub min_cost: Option<f64>,
    pub max_cost: Option<f64>,
    pub fixed_cost: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct TraderRuntimeConfig {
    pub trader_id: String,
    #[allow(dead_code)]
    pub name: String,
    pub ai_model_id: String,
    pub ai_model_name: String,
    pub ai_provider_type: String,
    pub ai_api_key: String,
    pub ai_base_url: String,
    pub exchange_id: String,
    pub scan_interval_minutes: i64,
    pub initial_balance: f64,
    pub btc_eth_leverage: i64,
    pub altcoin_leverage: i64,
    pub is_cross_margin: bool,
    pub trading_symbols: String,
    pub custom_prompt: String,
    pub override_base_prompt: bool,
    #[allow(dead_code)]
    pub system_prompt_template: String,
    pub symbols_config: Vec<SymbolConfig>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeExecutionMode {
    Simulated,
    LiveExchange,
}

#[derive(Debug, Clone)]
pub struct RuntimeExecutionContext {
    pub mode: RuntimeExecutionMode,
}

#[derive(Debug, Clone)]
pub struct PositionView {
    pub id: String,
    pub symbol: String,
    pub side: String, // LONG / SHORT
    pub quantity: f64,
    pub entry_price: f64,
    pub mark_price: f64,
    #[allow(dead_code)]
    pub leverage: i64,
    pub opened_at: i64,
}

#[derive(Debug, Clone)]
pub struct MarketState {
    pub price: f64,
    pub prev_price: f64,
    pub volatility: f64,
}

#[derive(Debug, Clone)]
pub struct DecisionSignal {
    pub symbol: String,
    pub action: &'static str, // BUY / SELL / HOLD
    pub confidence: f64,
    pub reason: String,
    pub timeframe: &'static str,
    pub price: f64,
    pub momentum: f64,
    pub risk_level: String,
    pub trigger_source: String,
    pub action_taken: String,
    pub correlation_id: String,
}
