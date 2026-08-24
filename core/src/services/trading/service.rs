pub use hmac::{Hmac, Mac};
pub use reqwest::Client;
pub use serde_json::{Value, json};
pub use sha2::Sha256;
pub use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Duration,
};
pub use tokio::time::sleep;
pub use tracing::warn;
pub use uuid::Uuid;

pub type HmacSha256 = Hmac<Sha256>;

pub use crate::{
    clients::{
        binance::normalize_order_quantity_by_constraints,
        exchanges::{
            ExchangeCredentials, ExchangeOrderType, ExchangeSide, LiveExchangeAdapter,
            PlaceOrderRequest, PositionSide, create_exchange_adapter,
        },
    },
    contracts::public::{EquityHistoryPointPayload, EquityHistoryQuery},
    contracts::trading::{
        accounts::{
            ClosePositionPayload, ClosePositionRequest, GridRiskInfoPayload,
            SymbolConcentrationPayload, TraderAccountPayload, TraderBalanceSyncPayload,
        },
        alerts::{
            RuntimeAlertAckPayload, RuntimeAlertAckRequest, RuntimeAlertControlTargetRequest,
            RuntimeAlertControlsPayload, RuntimeAlertControlsQuery,
            RuntimeAlertDeliveriesFiltersPayload, RuntimeAlertDeliveriesPayload,
            RuntimeAlertDeliveriesQuery, RuntimeAlertDeliveryLogPayload,
            RuntimeAlertHistoryFiltersPayload, RuntimeAlertHistoryItemPayload,
            RuntimeAlertHistoryPayload, RuntimeAlertHistoryQuery, RuntimeAlertItemPayload,
            RuntimeAlertMutePayload, RuntimeAlertMuteRequest, RuntimeAlertNotificationPayload,
            RuntimeAlertRatesPayload, RuntimeAlertStatePayload, RuntimeAlertThresholdsPayload,
            RuntimeAlertTotalsPayload, RuntimeAlertsPayload, RuntimeAlertsQuery,
        },
        common::{PaginationQuery, TraderQuery},
        history::{
            DecisionListPayload, DecisionPayload, DecisionQuery, LatestDecisionsPayload,
            StatisticsQuery, TradeListPayload, TradePayload, TraderStatisticsPayload,
        },
        orders::{FillListPayload, FillPayload, OrderListPayload, OrderPayload},
        positions::{PositionListPayload, PositionPayload, PositionQuery},
        runtime_observability::{
            RiskLevelCountPayload, RuntimeEventPayload, RuntimeEventTypePayload,
            RuntimeEventTypesPayload, RuntimeEventTypesQuery, RuntimeEventsFilterPayload,
            RuntimeEventsPayload, RuntimeEventsQuery, RuntimeMetricRatesPayload,
            RuntimeMetricTotalsPayload, RuntimeMetricsPayload, RuntimeMetricsQuery,
            RuntimeMetricsSeriesBucketPayload, RuntimeMetricsSeriesPayload,
            RuntimeMetricsSeriesQuery,
        },
        traders::{
            CreateTraderRequest, RuntimeEnginePayload, TraderCreatedPayload, TraderListPayload,
            TraderMessagePayload, TraderPayload, TraderStatusPayload, UpdatePromptRequest,
            UpdateTraderRequest,
        },
    },
    error::{AppError, AppErrorKind, Result as AppResult},
    repositories::trading::records::{
        accounts::TraderAccountRecord,
        alerts::{
            InsertRuntimeAlertDeliveryRecord, InsertRuntimeAlertHistoryRecord,
            RuntimeAlertControlsRecord, RuntimeAlertDeliveryRecord, RuntimeAlertHistoryRecord,
        },
        history::{TraderDecisionRecord, TraderTradeRecord},
        orders::{OrderFillRecord, TraderOrderRecord},
        positions::TraderPositionRecord,
        runtime_observability::RuntimeEventRecord,
        traders::{CreateTraderRecord, EquityHistoryPointRecord, TraderRecord, UpdateTraderRecord},
    },
    runtime_events::{
        EVENT_CANCEL_REPLACE_SUCCEEDED, EVENT_CANCEL_REPLACE_THROTTLED,
        EVENT_CANCEL_REPLACE_USED_MARKET_FALLBACK, EVENT_LIVE_OPEN_SKIPPED_MEDIUM_RISK,
        EVENT_LIVE_OPEN_USED_MARKET_FALLBACK, EVENT_LIVE_ORDER_SUBMITTED, EVENT_LIVE_RISK_SNAPSHOT,
        EVENT_STALE_INTENT_RECONCILE_PENDING, EVENT_STALE_INTENT_RECONCILE_TERMINAL,
        canonical_runtime_event_types,
    },
    services::trading_runtime::{
        config_loaders::exchange_credentials_missing, execution_live::persist_live_order_record,
        models::TraderRuntimeConfig, service::TradingRuntimeService,
    },
    state::{RuntimeEngineManager, RuntimeEngineState},
};

pub use super::{
    account::*, alerts::*, events_util::*, history::*, lifecycle::*, metrics::*, models::*,
    statistics::*, utils::*,
};

use crate::clients::market_data::ts_to_rfc3339;

#[derive(Debug, Clone)]
pub struct TradingService {
    state: SharedState,
    trading_runtime_service: Arc<TradingRuntimeService>,
}

impl TradingService {
    pub fn new(
        trading_repo: Arc<crate::repositories::TradingRepo>,
        exchange_repo: Arc<crate::repositories::ExchangeRepo>,
        runtime_alerts: crate::config::RuntimeAlertConfig,
        runtime_engine_manager: Arc<RwLock<RuntimeEngineManager>>,
        llm_service: Arc<crate::services::llm::LlmService>,
        trading_runtime_service: Arc<TradingRuntimeService>,
    ) -> Self {
        Self {
            state: TradingState::new(
                trading_repo,
                exchange_repo,
                runtime_alerts,
                runtime_engine_manager,
                llm_service,
            ),
            trading_runtime_service,
        }
    }

    fn state(&self) -> SharedState {
        self.state.clone()
    }

    pub async fn list_traders(&self) -> AppResult<TraderListPayload> {
        let state = self.state();
        list_traders(&state).await
    }

    pub async fn get_trader(&self, id: &str) -> AppResult<TraderPayload> {
        let state = self.state();
        get_trader(&state, id).await
    }

    pub async fn get_trader_config(&self, id: &str) -> AppResult<TraderPayload> {
        let state = self.state();
        get_trader_config(&state, id).await
    }

    pub async fn create_trader(
        &self,
        name: &str,
        ai_model_id: &str,
        exchange_id: &str,
        strategy_id: &str,
        initial_balance: f64,
        scan_interval_minutes: i64,
        is_cross_margin: Option<bool>,
        use_ai500: bool,
        use_oi_top: bool,
        custom_prompt: &str,
        override_base_prompt: bool,
        system_prompt_template: &str,
    ) -> AppResult<TraderCreatedPayload> {
        let state = self.state();
        create_trader(
            &state,
            name,
            ai_model_id,
            exchange_id,
            strategy_id,
            initial_balance,
            scan_interval_minutes,
            is_cross_margin,
            use_ai500,
            use_oi_top,
            custom_prompt,
            override_base_prompt,
            system_prompt_template,
        )
        .await
    }

    pub async fn update_trader(
        &self,
        id: &str,
        name: Option<String>,
        ai_model_id: Option<String>,
        exchange_id: Option<String>,
        strategy_id: Option<String>,
        initial_balance: Option<f64>,
        scan_interval_minutes: Option<i64>,
        is_cross_margin: Option<bool>,
        use_ai500: Option<bool>,
        use_oi_top: Option<bool>,
        custom_prompt: Option<String>,
        override_base_prompt: Option<bool>,
        system_prompt_template: Option<String>,
    ) -> AppResult<TraderMessagePayload> {
        let state = self.state();
        update_trader(
            &state,
            id,
            name,
            ai_model_id,
            exchange_id,
            strategy_id,
            initial_balance,
            scan_interval_minutes,
            is_cross_margin,
            use_ai500,
            use_oi_top,
            custom_prompt,
            override_base_prompt,
            system_prompt_template,
        )
        .await
    }

    pub async fn delete_trader(&self, id: &str) -> AppResult<TraderMessagePayload> {
        let state = self.state();
        delete_trader(&state, self.trading_runtime_service.as_ref(), id).await
    }

    pub async fn start_trader(&self, id: &str) -> AppResult<TraderMessagePayload> {
        start_trader(self.trading_runtime_service.as_ref(), id).await
    }

    pub async fn stop_trader(&self, id: &str) -> AppResult<TraderMessagePayload> {
        stop_trader(self.trading_runtime_service.as_ref(), id).await
    }

    pub async fn wake_trader(&self, id: &str) -> AppResult<TraderMessagePayload> {
        wake_trader(self.trading_runtime_service.as_ref(), id).await
    }

    pub async fn update_trader_prompt(
        &self,
        id: &str,
        custom_prompt: &str,
        override_base_prompt: bool,
    ) -> AppResult<TraderMessagePayload> {
        let state = self.state();
        update_trader_prompt(&state, id, custom_prompt, override_base_prompt).await
    }

    pub async fn equity_history(
        &self,
        trader_id: Option<String>,
    ) -> AppResult<Vec<EquityHistoryPointPayload>> {
        let trader_id = if let Some(v) = trader_id {
            v.trim().to_string()
        } else {
            match self
                .state()
                .trading_repo
                .latest_trader_id()
                .await
                .map_err(|err| AppError::Internal(format!("Get historical data: {err}")))?
            {
                Some(id) => id,
                None => return Ok(vec![]),
            }
        };

        if trader_id.is_empty() {
            return Err(AppError::BadRequest("Invalid trader ID".into()));
        }

        let rows = self
            .state()
            .trading_repo
            .equity_history_points(&trader_id, None, 500)
            .await
            .map_err(|err| AppError::Internal(format!("Get historical data: {err}")))?;

        Ok(rows.into_iter().map(equity_history_payload).collect())
    }

    pub async fn sync_balance(&self, id: &str) -> AppResult<TraderBalanceSyncPayload> {
        let state = self.state();
        sync_balance(&state, id).await
    }

    pub async fn close_position(
        &self,
        id: &str,
        symbol: &str,
        side: &str,
        local_only: bool,
    ) -> AppResult<ClosePositionPayload> {
        let state = self.state();
        close_position(
            &state,
            self.trading_runtime_service.as_ref(),
            id,
            symbol,
            side,
            local_only,
        )
        .await
    }

    pub async fn grid_risk_info(&self, id: &str) -> AppResult<GridRiskInfoPayload> {
        let state = self.state();
        grid_risk_info(&state, id).await
    }

    pub async fn status(&self, trader_id: Option<String>) -> AppResult<TraderStatusPayload> {
        let state = self.state();
        status(&state, trader_id).await
    }

    pub async fn account(&self, trader_id: Option<String>) -> AppResult<TraderAccountPayload> {
        let state = self.state();
        account(&state, trader_id).await
    }

    pub async fn positions(
        &self,
        trader_id: Option<String>,
        status_filter: Option<String>,
    ) -> AppResult<PositionListPayload> {
        let state = self.state();
        positions(&state, trader_id, status_filter).await
    }

    pub async fn positions_history(
        &self,
        trader_id: Option<String>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> AppResult<PositionListPayload> {
        let state = self.state();
        positions_history(&state, trader_id, limit, offset).await
    }

    pub async fn positions_by_strategy(
        &self,
        strategy_id: &str,
    ) -> AppResult<Vec<PositionPayload>> {
        let state = self.state();
        positions_by_strategy(&state, strategy_id).await
    }

    pub async fn decisions(
        &self,
        trader_id: Option<String>,
        limit: Option<i64>,
        offset: Option<i64>,
        symbol: Option<String>,
    ) -> AppResult<DecisionListPayload> {
        let state = self.state();
        decisions(&state, trader_id, limit, offset, symbol).await
    }

    pub async fn latest_decisions(
        &self,
        trader_id: Option<String>,
    ) -> AppResult<LatestDecisionsPayload> {
        let state = self.state();
        latest_decisions(&state, trader_id).await
    }

    pub async fn trades(
        &self,
        trader_id: Option<String>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> AppResult<TradeListPayload> {
        let state = self.state();
        trades(&state, trader_id, limit, offset).await
    }

    pub async fn orders(
        &self,
        trader_id: Option<String>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> AppResult<OrderListPayload> {
        let state = self.state();
        orders(&state, trader_id, limit, offset).await
    }

    pub async fn order_fills(
        &self,
        order_id: &str,
        trader_id: Option<String>,
    ) -> AppResult<FillListPayload> {
        let state = self.state();
        order_fills(&state, order_id, trader_id).await
    }

    pub async fn open_orders(
        &self,
        trader_id: Option<String>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> AppResult<OrderListPayload> {
        let state = self.state();
        open_orders(&state, trader_id, limit, offset).await
    }

    pub async fn runtime_events(
        &self,
        trader_id: Option<String>,
        window_hours: Option<i64>,
        limit: Option<i64>,
        offset: Option<i64>,
        event_type: Option<String>,
        risk_level: Option<String>,
        correlation_id: Option<String>,
    ) -> AppResult<RuntimeEventsPayload> {
        let state = self.state();
        runtime_events(
            &state,
            trader_id,
            window_hours,
            limit,
            offset,
            event_type,
            risk_level,
            correlation_id,
        )
        .await
    }

    pub async fn runtime_event_types(
        &self,
        trader_id: Option<String>,
        window_hours: Option<i64>,
    ) -> AppResult<RuntimeEventTypesPayload> {
        let state = self.state();
        runtime_event_types(&state, trader_id, window_hours).await
    }

    pub async fn runtime_metrics(
        &self,
        trader_id: Option<String>,
        window_hours: Option<i64>,
    ) -> AppResult<RuntimeMetricsPayload> {
        let state = self.state();
        runtime_metrics(&state, trader_id, window_hours).await
    }

    pub async fn runtime_metrics_series(
        &self,
        trader_id: Option<String>,
        window_hours: Option<i64>,
        bucket_minutes: Option<i64>,
    ) -> AppResult<RuntimeMetricsSeriesPayload> {
        let state = self.state();
        runtime_metrics_series(&state, trader_id, window_hours, bucket_minutes).await
    }

    pub async fn runtime_alerts(
        &self,
        trader_id: Option<String>,
        window_hours: Option<i64>,
        open_market_fallback_rate_max_pct: Option<f64>,
        replace_throttle_rate_max_pct: Option<f64>,
        stale_reconcile_terminal_rate_max_pct: Option<f64>,
        persist_min_interval_secs: Option<i64>,
    ) -> AppResult<RuntimeAlertsPayload> {
        let state = self.state();
        runtime_alerts(
            &state,
            trader_id,
            window_hours,
            open_market_fallback_rate_max_pct,
            replace_throttle_rate_max_pct,
            stale_reconcile_terminal_rate_max_pct,
            persist_min_interval_secs,
        )
        .await
    }

    pub async fn runtime_alert_history(
        &self,
        trader_id: Option<String>,
        window_hours: Option<i64>,
        limit: Option<i64>,
        offset: Option<i64>,
        breached_only: Option<bool>,
        severity: Option<String>,
    ) -> AppResult<RuntimeAlertHistoryPayload> {
        let state = self.state();
        runtime_alert_history(
            &state,
            trader_id,
            window_hours,
            limit,
            offset,
            breached_only,
            severity,
        )
        .await
    }

    pub async fn runtime_alert_deliveries(
        &self,
        trader_id: Option<String>,
        window_hours: Option<i64>,
        limit: Option<i64>,
        offset: Option<i64>,
        success: Option<bool>,
        destination: Option<String>,
    ) -> AppResult<RuntimeAlertDeliveriesPayload> {
        let state = self.state();
        runtime_alert_deliveries(
            &state,
            trader_id,
            window_hours,
            limit,
            offset,
            success,
            destination,
        )
        .await
    }

    pub async fn runtime_alert_controls(
        &self,
        trader_id: Option<String>,
    ) -> AppResult<RuntimeAlertControlsPayload> {
        let state = self.state();
        runtime_alert_controls(&state, trader_id).await
    }

    pub async fn mute_runtime_alerts(
        &self,
        trader_id: Option<String>,
        mute_minutes: Option<i64>,
        mute_until: Option<i64>,
        reason: Option<String>,
    ) -> AppResult<RuntimeAlertMutePayload> {
        let state = self.state();
        mute_runtime_alerts(&state, trader_id, mute_minutes, mute_until, reason).await
    }

    pub async fn unmute_runtime_alerts(
        &self,
        trader_id: Option<String>,
    ) -> AppResult<RuntimeAlertMutePayload> {
        let state = self.state();
        unmute_runtime_alerts(&state, trader_id).await
    }

    pub async fn ack_runtime_alerts(
        &self,
        trader_id: Option<String>,
        note: Option<String>,
    ) -> AppResult<RuntimeAlertAckPayload> {
        let state = self.state();
        ack_runtime_alerts(&state, trader_id, note).await
    }

    pub async fn statistics(
        &self,
        trader_id: Option<String>,
        days: Option<i64>,
    ) -> AppResult<TraderStatisticsPayload> {
        let state = self.state();
        statistics(&state, trader_id, days).await
    }
}

// ====== trader lifecycle ======

fn equity_history_payload(row: EquityHistoryPointRecord) -> EquityHistoryPointPayload {
    EquityHistoryPointPayload {
        timestamp: ts_to_rfc3339(row.timestamp),
        total_equity: row.total_equity,
        available_balance: row.available_balance,
        total_pnl: row.total_pnl,
        total_pnl_pct: row.total_pnl_pct,
        position_count: row.position_count,
        margin_used_pct: row.margin_used_pct,
        balance: row.balance,
    }
}
