use std::sync::{Arc, RwLock};

use crate::{
    contracts::trading::traders::TraderPayload,
    error::{AppError, Result as AppResult},
    repositories::trading::records::traders::TraderRecord,
    repositories::{ExchangeRepo, trading::TradingRepo},
    services::llm::LlmService,
    state::{RuntimeEngineManager, RuntimeEngineState},
};

#[derive(Debug, Clone)]
pub struct TradingConfig {
    pub runtime_alerts: crate::config::RuntimeAlertConfig,
}

#[derive(Debug, Clone)]
pub struct TradingState {
    pub trading_repo: Arc<TradingRepo>,
    pub exchange_repo: Arc<ExchangeRepo>,
    pub config: TradingConfig,
    pub runtime_engine_manager: Arc<RwLock<RuntimeEngineManager>>,
    pub llm_service: Arc<LlmService>,
}

pub type SharedState = TradingState;

impl TradingState {
    pub fn new(
        trading_repo: Arc<TradingRepo>,
        exchange_repo: Arc<ExchangeRepo>,
        runtime_alerts: crate::config::RuntimeAlertConfig,
        runtime_engine_manager: Arc<RwLock<RuntimeEngineManager>>,
        llm_service: Arc<LlmService>,
    ) -> Self {
        Self {
            trading_repo,
            exchange_repo,
            config: TradingConfig { runtime_alerts },
            runtime_engine_manager,
            llm_service,
        }
    }

    pub fn remove_runtime_engine(&self, trader_id: &str) -> AppResult<bool> {
        let mut manager = self
            .runtime_engine_manager
            .write()
            .map_err(|_| AppError::Internal("Concurrent state access failed".into()))?;
        Ok(manager.remove(trader_id).is_some())
    }

    pub fn runtime_engine(&self, trader_id: &str) -> AppResult<Option<RuntimeEngineState>> {
        let manager = self
            .runtime_engine_manager
            .read()
            .map_err(|_| AppError::Internal("Concurrent state access failed".into()))?;
        Ok(manager.get(trader_id))
    }
}

pub trait TraderPayloadExt {
    fn into_payload(self) -> TraderPayload;
}

impl TraderPayloadExt for TraderRecord {
    fn into_payload(self) -> TraderPayload {
        TraderPayload {
            id: self.id,
            name: self.name,
            ai_model_id: self.ai_model_id,
            exchange_id: self.exchange_id,
            strategy_id: self.strategy_id,
            initial_balance: self.initial_balance,
            scan_interval_minutes: self.scan_interval_minutes,
            is_running: self.is_running != 0,
            is_cross_margin: self.is_cross_margin != 0,
            show_in_competition: self.show_in_competition != 0,
            btc_eth_leverage: self.btc_eth_leverage,
            altcoin_leverage: self.altcoin_leverage,
            trading_symbols: self.trading_symbols,
            use_ai500: self.use_ai500 != 0,
            use_oi_top: self.use_oi_top != 0,
            custom_prompt: self.custom_prompt,
            override_base_prompt: self.override_base_prompt != 0,
            system_prompt_template: self.system_prompt_template,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
