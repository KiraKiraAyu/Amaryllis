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
        req: CreateTraderRequest,
    ) -> AppResult<TraderCreatedPayload> {
        let state = self.state();
        create_trader(&state, req).await
    }

    pub async fn update_trader(
        &self,
        id: &str,
        req: UpdateTraderRequest,
    ) -> AppResult<TraderMessagePayload> {
        let state = self.state();
        update_trader(&state, id, req).await
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

    pub async fn update_trader_prompt(
        &self,
        id: &str,
        req: UpdatePromptRequest,
    ) -> AppResult<TraderMessagePayload> {
        let state = self.state();
        update_trader_prompt(&state, id, req).await
    }

    pub async fn equity_history(
        &self,
        query: EquityHistoryQuery,
    ) -> AppResult<Vec<EquityHistoryPointPayload>> {
        let trader_id = if let Some(v) = query.trader_id {
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

    pub async fn sync_balance(
        &self,
        id: &str,
    ) -> AppResult<TraderBalanceSyncPayload> {
        let state = self.state();
        sync_balance(&state, id).await
    }

    pub async fn close_position(
        &self,
        id: &str,
        req: ClosePositionRequest,
    ) -> AppResult<ClosePositionPayload> {
        let state = self.state();
        close_position(
            &state,
            self.trading_runtime_service.as_ref(),
            id,
            req,
        )
        .await
    }

    pub async fn grid_risk_info(&self, id: &str) -> AppResult<GridRiskInfoPayload> {
        let state = self.state();
        grid_risk_info(&state, id).await
    }

    pub async fn status(
        &self,
        query: TraderQuery,
    ) -> AppResult<TraderStatusPayload> {
        let state = self.state();
        status(&state, query).await
    }

    pub async fn account(
        &self,
        query: TraderQuery,
    ) -> AppResult<TraderAccountPayload> {
        let state = self.state();
        account(&state, query).await
    }

    pub async fn positions(
        &self,
        query: PositionQuery,
    ) -> AppResult<PositionListPayload> {
        let state = self.state();
        positions(&state, query).await
    }

    pub async fn positions_history(
        &self,
        query: PaginationQuery,
    ) -> AppResult<PositionListPayload> {
        let state = self.state();
        positions_history(&state, query).await
    }

    pub async fn decisions(
        &self,
        query: DecisionQuery,
    ) -> AppResult<DecisionListPayload> {
        let state = self.state();
        decisions(&state, query).await
    }

    pub async fn latest_decisions(
        &self,
        query: TraderQuery,
    ) -> AppResult<LatestDecisionsPayload> {
        let state = self.state();
        latest_decisions(&state, query).await
    }

    pub async fn trades(
        &self,
        query: PaginationQuery,
    ) -> AppResult<TradeListPayload> {
        let state = self.state();
        trades(&state, query).await
    }

    pub async fn orders(
        &self,
        query: PaginationQuery,
    ) -> AppResult<OrderListPayload> {
        let state = self.state();
        orders(&state, query).await
    }

    pub async fn order_fills(
        &self,
        order_id: &str,
        query: TraderQuery,
    ) -> AppResult<FillListPayload> {
        let state = self.state();
        order_fills(&state, order_id, query).await
    }

    pub async fn open_orders(
        &self,
        query: PaginationQuery,
    ) -> AppResult<OrderListPayload> {
        let state = self.state();
        open_orders(&state, query).await
    }

    pub async fn runtime_events(
        &self,
        query: RuntimeEventsQuery,
    ) -> AppResult<RuntimeEventsPayload> {
        let state = self.state();
        runtime_events(&state, query).await
    }

    pub async fn runtime_event_types(
        &self,
        query: RuntimeEventTypesQuery,
    ) -> AppResult<RuntimeEventTypesPayload> {
        let state = self.state();
        runtime_event_types(&state, query).await
    }

    pub async fn runtime_metrics(
        &self,
        query: RuntimeMetricsQuery,
    ) -> AppResult<RuntimeMetricsPayload> {
        let state = self.state();
        runtime_metrics(&state, query).await
    }

    pub async fn runtime_metrics_series(
        &self,
        query: RuntimeMetricsSeriesQuery,
    ) -> AppResult<RuntimeMetricsSeriesPayload> {
        let state = self.state();
        runtime_metrics_series(&state, query).await
    }

    pub async fn runtime_alerts(
        &self,
        query: RuntimeAlertsQuery,
    ) -> AppResult<RuntimeAlertsPayload> {
        let state = self.state();
        runtime_alerts(&state, query).await
    }

    pub async fn runtime_alert_history(
        &self,
        query: RuntimeAlertHistoryQuery,
    ) -> AppResult<RuntimeAlertHistoryPayload> {
        let state = self.state();
        runtime_alert_history(&state, query).await
    }

    pub async fn runtime_alert_deliveries(
        &self,
        query: RuntimeAlertDeliveriesQuery,
    ) -> AppResult<RuntimeAlertDeliveriesPayload> {
        let state = self.state();
        runtime_alert_deliveries(&state, query).await
    }

    pub async fn runtime_alert_controls(
        &self,
        query: RuntimeAlertControlsQuery,
    ) -> AppResult<RuntimeAlertControlsPayload> {
        let state = self.state();
        runtime_alert_controls(&state, query).await
    }

    pub async fn mute_runtime_alerts(
        &self,
        req: RuntimeAlertMuteRequest,
    ) -> AppResult<RuntimeAlertMutePayload> {
        let state = self.state();
        mute_runtime_alerts(&state, req).await
    }

    pub async fn unmute_runtime_alerts(
        &self,
        req: RuntimeAlertControlTargetRequest,
    ) -> AppResult<RuntimeAlertMutePayload> {
        let state = self.state();
        unmute_runtime_alerts(&state, req).await
    }

    pub async fn ack_runtime_alerts(
        &self,
        req: RuntimeAlertAckRequest,
    ) -> AppResult<RuntimeAlertAckPayload> {
        let state = self.state();
        ack_runtime_alerts(&state, req).await
    }

    pub async fn statistics(
        &self,
        query: StatisticsQuery,
    ) -> AppResult<TraderStatisticsPayload> {
        let state = self.state();
        statistics(&state, query).await
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
