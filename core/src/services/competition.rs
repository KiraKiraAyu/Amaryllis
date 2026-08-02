use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    clients::market_data::{now_ts, ts_to_rfc3339},
    contracts::public::{
        EquityHistoryBatchPayload, EquityHistoryBatchRequest, EquityHistoryPointPayload,
        EquityHistoryQuery, PublicCompetitionTraderPayload, PublicTraderConfigPayload,
    },
    error::{AppError, Result},
    repositories::competition::{
        CompetitionRepo, CompetitionTraderRecord, EquityHistoryPointRecord,
        PublicTraderConfigRecord,
    },
};

#[derive(Debug, Clone)]
pub struct CompetitionService {
    competition_repo: Arc<CompetitionRepo>,
}

impl CompetitionService {
    pub fn new(competition_repo: Arc<CompetitionRepo>) -> Self {
        Self { competition_repo }
    }

    pub async fn competition(&self) -> Result<Vec<PublicCompetitionTraderPayload>> {
        self.load_public_competition_data()
            .await
            .map_err(|err| AppError::Internal(format!("Get competition data: {err}")))
    }

    pub async fn top_traders(&self) -> Result<Vec<PublicCompetitionTraderPayload>> {
        let mut items = self
            .load_public_competition_data()
            .await
            .map_err(|err| AppError::Internal(format!("Get top traders data: {err}")))?;

        items.sort_by(|a, b| {
            b.total_pnl_pct
                .partial_cmp(&a.total_pnl_pct)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if items.len() > 5 {
            items.truncate(5);
        }

        Ok(items)
    }

    pub async fn equity_history(
        &self,
        query: EquityHistoryQuery,
    ) -> Result<Vec<EquityHistoryPointPayload>> {
        let trader_id = if let Some(v) = query.trader_id {
            v.trim().to_string()
        } else {
            match self
                .competition_repo
                .latest_public_trader_id()
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

        self.load_equity_history_points(&trader_id, None)
            .await
            .map_err(|err| AppError::Internal(format!("Get historical data: {err}")))
    }

    pub async fn equity_history_batch(
        &self,
        request: EquityHistoryBatchRequest,
    ) -> Result<EquityHistoryBatchPayload> {
        let mut trader_ids = request.trader_ids.unwrap_or_default();
        let hours = request.hours.unwrap_or(0).max(0);

        if trader_ids.is_empty() {
            let mut items = self
                .top_traders()
                .await
                .map_err(|err| AppError::Internal(format!("Get top traders: {err}")))?;
            items.sort_by(|a, b| {
                b.total_pnl_pct
                    .partial_cmp(&a.total_pnl_pct)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            trader_ids = items.into_iter().take(5).map(|v| v.trader_id).collect();
        }

        if trader_ids.len() > 20 {
            trader_ids.truncate(20);
        }

        let mut histories = HashMap::new();
        let mut errors = HashMap::new();

        for trader_id in trader_ids {
            if trader_id.trim().is_empty() {
                continue;
            }
            match self
                .load_equity_history_points(trader_id.trim(), Some(hours))
                .await
            {
                Ok(v) => {
                    histories.insert(trader_id.to_string(), v);
                }
                Err(_) => {
                    errors.insert(
                        trader_id.to_string(),
                        "Failed to get historical data".to_string(),
                    );
                }
            }
        }

        Ok(EquityHistoryBatchPayload { histories, errors })
    }

    pub async fn public_trader_config(&self, id: &str) -> Result<PublicTraderConfigPayload> {
        let row = self
            .competition_repo
            .public_trader_config(id)
            .await
            .map_err(|err| {
                AppError::Internal(format!("Failed to get public trader config: {err}"))
            })?
            .ok_or_else(|| AppError::NotFound("Trader not found".into()))?;

        Ok(public_trader_config_payload(row))
    }

    async fn load_public_competition_data(&self) -> Result<Vec<PublicCompetitionTraderPayload>> {
        self.competition_repo
            .public_traders()
            .await
            .map(|rows| rows.into_iter().map(competition_trader_payload).collect())
            .map_err(AppError::from)
    }

    async fn load_equity_history_points(
        &self,
        trader_id: &str,
        hours: Option<i64>,
    ) -> Result<Vec<EquityHistoryPointPayload>> {
        let limit = if hours.unwrap_or(0) > 0 {
            5000_i64
        } else {
            500_i64
        };
        let since_ts = hours
            .filter(|hours| *hours > 0)
            .map(|hours| now_ts() - hours.saturating_mul(3600));

        self.competition_repo
            .equity_history_points(trader_id, since_ts, limit)
            .await
            .map(|rows| rows.into_iter().map(equity_history_payload).collect())
            .map_err(AppError::from)
    }
}

fn competition_trader_payload(row: CompetitionTraderRecord) -> PublicCompetitionTraderPayload {
    PublicCompetitionTraderPayload {
        trader_id: row.trader_id,
        trader_name: row.trader_name,
        ai_model: row.ai_model,
        exchange: row.exchange,
        total_equity: row.total_equity,
        total_pnl: row.total_pnl,
        total_pnl_pct: row.total_pnl_pct,
        unrealized_pnl: row.unrealized_pnl,
        realized_pnl: row.realized_pnl,
        position_count: row.position_count,
        margin_used_pct: row.margin_used_pct,
        is_running: row.is_running,
    }
}

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

fn public_trader_config_payload(row: PublicTraderConfigRecord) -> PublicTraderConfigPayload {
    PublicTraderConfigPayload {
        trader_id: row.trader_id,
        trader_name: row.trader_name,
        ai_model: row.ai_model,
        exchange_id: row.exchange_id,
        strategy_id: row.strategy_id,
        is_cross_margin: row.is_cross_margin,
        show_in_competition: row.show_in_competition,
        scan_interval_minutes: row.scan_interval_minutes,
        initial_balance: row.initial_balance,
        is_running: row.is_running,
        btc_eth_leverage: row.btc_eth_leverage,
        altcoin_leverage: row.altcoin_leverage,
        trading_symbols: row.trading_symbols,
        custom_prompt: row.custom_prompt,
        override_base_prompt: row.override_base_prompt,
        system_prompt_template: row.system_prompt_template,
        use_ai500: row.use_ai500,
        use_oi_top: row.use_oi_top,
    }
}
