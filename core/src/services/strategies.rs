use serde_json::{Value, json};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    clients::market_data::{now_ts, parse_json_value, ts_to_rfc3339},
    contracts::strategies::{
        PreviewPromptPayload, StrategyCreatedPayload,
        StrategyDefaultConfigPayload, StrategyListPayload,
        StrategyMessagePayload, StrategyPayload, StrategyTestRunPayload,
    },
    error::{AppError, AppErrorKind, Result},
    repositories::strategies::{
        CreateStrategyRecord, StrategyRecord, StrategyRepo, UpdateStrategyRecord,
    },
    services::llm::{LlmMessage, LlmService},
    services::trading_runtime::ai_decision::build_system_prompt_from_config,
};

#[derive(Debug, Clone)]
pub struct StrategyService {
    strategy_repo: Arc<StrategyRepo>,
    llm_service: Arc<LlmService>,
}

impl StrategyService {
    pub fn new(strategy_repo: Arc<StrategyRepo>, llm_service: Arc<LlmService>) -> Self {
        Self {
            strategy_repo,
            llm_service,
        }
    }

    pub async fn list_strategies(&self) -> Result<StrategyListPayload> {
        let rows = self
            .strategy_repo
            .list_for_user_with_defaults()
            .await
            .map_err(|_| strategy_error(AppErrorKind::Internal, "Failed to get strategy list"))?;

        Ok(strategy_list_payload(rows))
    }

    pub async fn get_strategy(&self, id: String) -> Result<StrategyPayload> {
        let row = self
            .strategy_repo
            .get_accessible(&id)
            .await
            .map_err(|_| strategy_error(AppErrorKind::Internal, "Failed to get strategy"))?
            .ok_or_else(|| strategy_error(AppErrorKind::NotFound, "Strategy not found"))?;

        Ok(strategy_payload(row))
    }

    pub async fn create_strategy(
        &self,
        name: String,
        description: String,
        config: Value,
    ) -> Result<StrategyCreatedPayload> {
        let id = Uuid::now_v7().to_string();
        let now = now_ts();
        let config_str = serde_json::to_string(&config).unwrap_or_else(|_| "{}".to_string());
        self.strategy_repo
            .create(CreateStrategyRecord {
                id: id.clone(),
                name,
                description,
                config: config_str,
                created_at: now,
                updated_at: now,
            })
            .await
            .map_err(|_| strategy_error(AppErrorKind::Internal, "Failed to create strategy"))?;

        Ok(StrategyCreatedPayload {
            id,
            message: "Strategy created successfully",
        })
    }

    pub async fn update_strategy(
        &self,
        id: String,
        name: String,
        description: String,
        config: Value,
    ) -> Result<StrategyMessagePayload> {
        let existing = self
            .strategy_repo
            .get_owned(&id)
            .await
            .map_err(|_| strategy_error(AppErrorKind::Internal, "Failed to update strategy"))?
            .ok_or_else(|| strategy_error(AppErrorKind::NotFound, "Strategy not found"))?;

        let name = if name.is_empty() {
            existing.name
        } else {
            name
        };
        let description = if description.is_empty() {
            existing.description
        } else {
            description
        };
        let config = if config.is_object() {
            serde_json::to_string(&config).unwrap_or_else(|_| "{}".to_string())
        } else {
            existing.config
        };

        self.strategy_repo
            .update_owned(
                &id,
                UpdateStrategyRecord {
                    name,
                    description,
                    config,
                    updated_at: now_ts(),
                },
            )
            .await
            .map_err(|_| strategy_error(AppErrorKind::Internal, "Failed to update strategy"))?;

        Ok(StrategyMessagePayload {
            message: "Strategy updated successfully",
        })
    }

    pub async fn delete_strategy(&self, id: String) -> Result<StrategyMessagePayload> {
        let affected = self
            .strategy_repo
            .delete_owned(&id)
            .await
            .map_err(|_| strategy_error(AppErrorKind::Internal, "Failed to delete strategy"))?;

        if affected == 0 {
            return Err(strategy_error(AppErrorKind::NotFound, "Strategy not found"));
        }

        Ok(StrategyMessagePayload {
            message: "Strategy deleted successfully",
        })
    }

    pub async fn activate_strategy(&self, id: String) -> Result<StrategyMessagePayload> {
        let exists =
            self.strategy_repo.get_owned(&id).await.map_err(|_| {
                strategy_error(AppErrorKind::Internal, "Failed to activate strategy")
            })?;
        if exists.is_none() {
            return Err(strategy_error(AppErrorKind::NotFound, "Strategy not found"));
        }

        let now = now_ts();
        self.strategy_repo
            .deactivate_all_for_user(now)
            .await
            .map_err(|_| strategy_error(AppErrorKind::Internal, "Failed to activate strategy"))?;
        self.strategy_repo
            .activate_owned(&id, now)
            .await
            .map_err(|_| strategy_error(AppErrorKind::Internal, "Failed to activate strategy"))?;

        Ok(StrategyMessagePayload {
            message: "Strategy activated successfully",
        })
    }

    pub async fn duplicate_strategy(
        &self,
        id: String,
        name: String,
    ) -> Result<StrategyCreatedPayload> {
        let source = self
            .strategy_repo
            .get_duplicable(&id)
            .await
            .map_err(|_| strategy_error(AppErrorKind::Internal, "Failed to duplicate strategy"))?
            .ok_or_else(|| strategy_error(AppErrorKind::NotFound, "Strategy not found"))?;

        let new_id = Uuid::now_v7().to_string();
        let new_name = if name.is_empty() {
            format!("{} Copy", source.name)
        } else {
            name
        };
        let now = now_ts();

        self.strategy_repo
            .create(CreateStrategyRecord {
                id: new_id.clone(),
                name: new_name,
                description: source.description,
                config: source.config,
                created_at: now,
                updated_at: now,
            })
            .await
            .map_err(|_| strategy_error(AppErrorKind::Internal, "Failed to duplicate strategy"))?;

        Ok(StrategyCreatedPayload {
            id: new_id,
            message: "Strategy duplicated successfully",
        })
    }

    pub async fn active_strategy(&self) -> Result<StrategyPayload> {
        let row = self
            .strategy_repo
            .active_for_user()
            .await
            .map_err(|_| strategy_error(AppErrorKind::Internal, "Failed to get active strategy"))?
            .ok_or_else(|| strategy_error(AppErrorKind::NotFound, "No active strategy"))?;

        Ok(strategy_payload(row))
    }

    pub fn default_strategy_config(
        &self,
        lang: Option<String>,
    ) -> Result<StrategyDefaultConfigPayload> {
        let language = lang.unwrap_or_else(|| "en".to_string());
        let payload = StrategyDefaultConfigPayload {
            config: default_strategy_config(&language),
            language,
        };

        Ok(payload)
    }

    pub fn preview_prompt(
        &self,
        config: Value,
        _account_equity: Option<f64>,
        prompt_variant: Option<String>,
    ) -> Result<PreviewPromptPayload> {
        let prompt_variant = prompt_variant
            .unwrap_or_else(|| "balanced".to_string())
            .trim()
            .to_string();

        // Use the unified prompt builder shared with live trading, backtest, and test run
        let system_prompt = build_system_prompt_from_config(
            &config,
            true, // is_cross_margin: default for preview (no trader context)
            "",   // no trader-specific custom prompt
            false,
        );

        let payload = PreviewPromptPayload {
            system_prompt,
            prompt_variant,
        };

        Ok(payload)
    }

    pub async fn test_run(
        &self,
        config: Value,
        prompt_variant: Option<String>,
        ai_model_id: Option<String>,
        run_real_ai: Option<bool>,
    ) -> Result<StrategyTestRunPayload> {
        let started = std::time::Instant::now();
        let prompt_variant = prompt_variant
            .unwrap_or_else(|| "balanced".to_string())
            .trim()
            .to_string();
        let resolved_model = self
            .llm_service
            .resolve_for_user(ai_model_id.as_deref())
            .await?;
        let ai_model_id = resolved_model.id.clone();
        let run_real_ai = run_real_ai.unwrap_or(false);

        let system_prompt = build_system_prompt_from_config(
            &config,
            true, // is_cross_margin: default for test run (no trader context)
            "",   // no trader-specific custom prompt
            false,
        );

        let config_str =
            serde_json::to_string_pretty(&config).unwrap_or_else(|_| "{}".to_string());

        let mut symbols: Vec<String> = Vec::new();
        if let Some(symbols_arr) = config.get("symbols").and_then(|v| v.as_array()) {
            for item in symbols_arr {
                if let Some(symbol) = item.get("symbol").and_then(|v| v.as_str()) {
                    let s_trim = symbol.trim().to_uppercase();
                    if !s_trim.is_empty() {
                        symbols.push(s_trim);
                    }
                }
            }
        }
        if symbols.is_empty() {
            if let Some(s) = config
                .get("trading_symbols")
                .and_then(|v| v.as_str())
            {
                symbols = s
                    .split(',')
                    .map(|sym| sym.trim().to_uppercase())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }

        if symbols.is_empty() {
            return Err(strategy_error(
                AppErrorKind::BadRequest,
                "No symbols configured. Please add at least one trading symbol to run the test.",
            ));
        }

        let symbols_str = symbols.join(", ");
        let user_prompt = format!(
            "Strategy variant: {prompt_variant}\n\
        Model: {ai_model_id}\n\
        Symbols to analyze: {symbols_str}\n\n\
        Strategy config:\n{config_str}\n\n\
        Based on this strategy configuration and current market conditions, \
        provide trading decisions for each symbol: {symbols_str}.\n\
        Return a JSON array only.",
            prompt_variant = prompt_variant,
            ai_model_id = ai_model_id,
            symbols_str = symbols_str,
            config_str = config_str,
        );

        if run_real_ai {
            if resolved_model.api_key.trim().is_empty() {
                return Err(strategy_error(
                    AppErrorKind::BadRequest,
                    &format!(
                        "No API key found for model '{}'. Please configure it in Settings.",
                        ai_model_id
                    ),
                ));
            }

            let messages = vec![LlmMessage {
                role: "user".to_string(),
                content: user_prompt.clone(),
            }];

            let raw_response = match self
                .llm_service
                .chat_with_model(&resolved_model, messages, Some(&system_prompt))
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    return Err(strategy_error(
                        AppErrorKind::BadGateway,
                        &format!("AI call failed: {e}"),
                    ));
                }
            };

            let decisions = parse_ai_decisions(&raw_response);
            let duration_ms = started.elapsed().as_millis() as u64;

            let payload = StrategyTestRunPayload {
                system_prompt,
                user_prompt,
                prompt_variant,
                ai_model_id,
                ai_response: raw_response,
                decisions,
                reasoning: "Real AI analysis complete.".to_string(),
                duration_ms,
                used_real_ai: true,
            };

            return Ok(payload);
        }

        let decisions = json!([
            {
                "action": "NO ACTION",
                "symbol": "BTCUSDT",
                "confidence": 62,
                "reasoning": "No clear multi-timeframe breakout and momentum is neutral."
            }
        ]);

        let payload = StrategyTestRunPayload {
            system_prompt,
            user_prompt,
            prompt_variant,
            ai_model_id,
            ai_response: "Simulated test-run complete.".to_string(),
            reasoning: "Strategy dry-run analyzed risk constraints and market context.".to_string(),
            decisions,
            duration_ms: started.elapsed().as_millis() as u64,
            used_real_ai: false,
        };

        Ok(payload)
    }
}

fn parse_ai_decisions(raw: &str) -> Value {
    let stripped = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    if let Some(start) = stripped.find('[') {
        if let Some(end) = stripped.rfind(']') {
            if end >= start {
                let candidate = &stripped[start..=end];
                if let Ok(v) = serde_json::from_str::<Value>(candidate) {
                    return v;
                }
            }
        }
    }

    if let Some(start) = stripped.find('{') {
        if let Some(end) = stripped.rfind('}') {
            if end >= start {
                let candidate = &stripped[start..=end];
                if let Ok(v) = serde_json::from_str::<Value>(candidate) {
                    return json!([v]);
                }
            }
        }
    }

    json!([{
        "action": "NO ACTION",
        "symbol": "UNKNOWN",
        "confidence": 50,
        "reasoning": raw.chars().take(500).collect::<String>()
    }])
}

pub(crate) fn strategy_error(kind: AppErrorKind, message: impl Into<String>) -> AppError {
    AppError::from_kind(kind, message)
}

fn strategy_list_payload(rows: Vec<StrategyRecord>) -> StrategyListPayload {
    let strategies: Vec<StrategyPayload> = rows.into_iter().map(strategy_payload).collect();

    StrategyListPayload {
        count: strategies.len(),
        strategies,
    }
}

fn strategy_payload(row: StrategyRecord) -> StrategyPayload {
    StrategyPayload {
        id: row.id,
        name: row.name,
        description: row.description,
        author_email: String::new(),
        is_active: row.is_active,
        config: parse_json_value(&row.config),
        created_at: ts_to_rfc3339(row.created_at),
        updated_at: ts_to_rfc3339(row.updated_at),
    }
}

fn default_strategy_config(lang: &str) -> Value {
    let is_zh = lang.eq_ignore_ascii_case("zh");
    json!({
        "strategy_type": "ai_trading",
        "language": if is_zh { "zh" } else { "en" },
        "symbols": [],
        "max_positions": 5,
        "prompt_variant": "balanced",
        "coin_source": {
            "source_type": "mixed",
            "static_coins": ["BTCUSDT", "ETHUSDT"],
            "excluded_coins": [],
            "use_ai500": true,
            "ai500_limit": 10,
            "use_oi_top": true,
            "oi_top_limit": 10,
            "use_oi_low": false,
            "oi_low_limit": 10
        },
        "indicators": {
            "klines": {
                "primary_timeframe": "3m",
                "primary_count": 30,
                "longer_timeframe": "15m",
                "longer_count": 20,
                "enable_multi_timeframe": true,
                "selected_timeframes": ["3m", "15m"]
            },
            "enable_raw_klines": true,
            "enable_ema": true,
            "enable_macd": true,
            "enable_rsi": true,
            "enable_atr": true,
            "enable_boll": false,
            "enable_volume": true,
            "enable_oi": true,
            "enable_funding_rate": true,
            "quantauraos_api_key": "",
            "enable_quant_data": false,
            "enable_oi_ranking": false,
            "enable_netflow_ranking": false,
            "enable_price_ranking": false
        },
        "custom_prompt": "",
        "risk_control": {
            "max_positions": 3,
            "leverage": 5,
            "max_margin_usage": 0.9,
            "min_position_size": 20,
            "min_risk_reward_ratio": 1.5,
            "min_confidence": 0.6
        },
        "tp_sl": {
            "take_profit": {
                "mode": "fixed",
                "pnl_rate": null,
                "custom_prompt": null
            },
            "stop_loss": {
                "mode": "fixed",
                "pnl_rate": null,
                "custom_prompt": null
            }
        },
        "prompt_sections": {
            "role_definition": if is_zh {
                "# 你是专业的加密货币交易AI\n\n你专注于技术分析和风险管理。"
            } else {
                "# You are a professional crypto trading AI\n\nFocus on technical analysis and strict risk management."
            },
            "trading_frequency": if is_zh {
                "# 交易频率\n\n避免过度交易，优先高质量信号。"
            } else {
                "# Trading Frequency\n\nAvoid overtrading, prioritize high-quality setups."
            },
            "entry_standards": if is_zh {
                "# 开仓标准\n\n仅在多信号共振时开仓。"
            } else {
                "# Entry Standards\n\nEnter only with multi-signal confluence."
            },
            "decision_process": if is_zh {
                "# 决策流程\n\n先评估风险，再给出结构化决策。"
            } else {
                "# Decision Process\n\nAssess risk first, then output structured decisions."
            }
        },
        "grid_config": {
            "symbol": "BTCUSDT",
            "grid_count": 20,
            "total_investment": 1000,
            "leverage": 3,
            "upper_price": 0,
            "lower_price": 0,
            "use_atr_bounds": true,
            "atr_multiplier": 2.0,
            "distribution": "uniform",
            "max_drawdown_pct": 15,
            "stop_loss_pct": 5,
            "daily_loss_limit_pct": 8,
            "use_maker_only": true,
            "enable_direction_adjust": true,
            "direction_bias_ratio": 0.7
        }
    })
}

#[cfg(test)]
mod tests {
    use super::default_strategy_config;

    #[test]
    fn default_fixed_tp_sl_values_are_empty() {
        let config = default_strategy_config("en");
        let tp_sl = config
            .get("tp_sl")
            .expect("default TP/SL config");
        assert!(tp_sl
            .pointer("/take_profit/pnl_rate")
            .is_some_and(|value| value.is_null()));
        assert!(tp_sl
            .pointer("/stop_loss/pnl_rate")
            .is_some_and(|value| value.is_null()));
    }
}
