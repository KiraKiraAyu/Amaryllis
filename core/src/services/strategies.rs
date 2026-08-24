use serde_json::{Value, json};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    clients::market_data::{now_ts, parse_json_value, ts_to_rfc3339},
    contracts::strategies::{
        PreviewPromptPayload, StrategyCreatedPayload, StrategyDefaultConfigPayload,
        StrategyListPayload, StrategyMessagePayload, StrategyPayload, StrategyTestRunPayload,
    },
    error::{AppError, AppErrorKind, Result},
    repositories::strategies::{
        CreateStrategyRecord, StrategyRecord, StrategyRepo, UpdateStrategyRecord,
    },
    services::data_template::{data_template_value, validate_strategy_data_template},
    services::llm::{LlmMessage, LlmService},
    services::trading_runtime::prompt_assemble::{build_system_prompt_from_config, build_test_run_prompt},
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
        let mut config = config;
        let template = validate_strategy_data_template(&config)
            .map_err(|message| strategy_error(AppErrorKind::BadRequest, message))?;
        config["data_template"] = data_template_value(&template);

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

        let name = if name.is_empty() { existing.name } else { name };
        let description = if description.is_empty() {
            existing.description
        } else {
            description
        };
        let config = if config.is_object() {
            let mut config = config;
            let template = validate_strategy_data_template(&config)
                .map_err(|message| strategy_error(AppErrorKind::BadRequest, message))?;
            config["data_template"] = data_template_value(&template);
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

    pub fn default_strategy_config(&self) -> Result<StrategyDefaultConfigPayload> {
        let payload = StrategyDefaultConfigPayload {
            config: default_strategy_config(),
        };

        Ok(payload)
    }

    pub fn preview_prompt(
        &self,
        config: Value,
        _account_equity: Option<f64>,
    ) -> Result<PreviewPromptPayload> {
        // Use the unified prompt builder shared with live trading, backtest, and test run
        let system_prompt = build_system_prompt_from_config(
            &config, true, // is_cross_margin: default for preview (no trader context)
            "",   // no trader-specific custom prompt
            false,
        );

        let payload = PreviewPromptPayload {
            system_prompt,
        };

        Ok(payload)
    }

    pub async fn test_run(
        &self,
        config: Value,
        ai_model_id: Option<String>,
        run_real_ai: Option<bool>,
    ) -> Result<StrategyTestRunPayload> {
        let started = std::time::Instant::now();
        let resolved_model = self
            .llm_service
            .resolve_for_user(ai_model_id.as_deref())
            .await?;
        let ai_model_id = resolved_model.id.clone();
        let run_real_ai = run_real_ai.unwrap_or(false);

        let system_prompt = build_system_prompt_from_config(
            &config, true, // is_cross_margin: default for test run (no trader context)
            "",   // no trader-specific custom prompt
            false,
        );

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
        if symbols.is_empty()
            && let Some(s) = config.get("trading_symbols").and_then(|v| v.as_str())
        {
            symbols = s
                .split(',')
                .map(|sym| sym.trim().to_uppercase())
                .filter(|s| !s.is_empty())
                .collect();
        }

        if symbols.is_empty() {
            return Err(strategy_error(
                AppErrorKind::BadRequest,
                "No symbols configured. Please add at least one trading symbol to run the test.",
            ));
        }

        let symbols_str = symbols.join(", ");
        let user_prompt = build_test_run_prompt(
            &ai_model_id,
            &symbols_str,
        );

        if run_real_ai {
            if resolved_model.api_key.trim().is_empty() {
                return Err(strategy_error(
                    AppErrorKind::BadRequest,
                    format!(
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
                        format!("AI call failed: {e}"),
                    ));
                }
            };

            let decisions = parse_ai_decisions(&raw_response);
            let duration_ms = started.elapsed().as_millis() as u64;

            let payload = StrategyTestRunPayload {
                system_prompt,
                user_prompt,
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
        .trim_start_matches("```JSON")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    if let Some(start) = stripped.find('[')
        && let Some(end) = stripped.rfind(']')
        && end >= start
    {
        let candidate = &stripped[start..=end];
        if let Ok(v) = serde_json::from_str::<Value>(candidate) {
            return v;
        }
    }

    if let Some(start) = stripped.find('{')
        && let Some(end) = stripped.rfind('}')
        && end >= start
    {
        let candidate = &stripped[start..=end];
        if let Ok(v) = serde_json::from_str::<Value>(candidate) {
            return json!([v]);
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

fn default_strategy_config() -> Value {
    json!({
        "strategy_type": "ai_trading",
        "symbols": [],
        "max_positions": 5,
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
        "data_template": data_template_value(&crate::services::data_template::default_data_template()),
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
            "role_definition": "# You are a professional crypto trading AI\n\nFocus on technical analysis and strict risk management.",
            "trading_frequency": "# Trading Frequency\n\nAvoid overtrading, prioritize high-quality setups.",
            "entry_standards": "# Entry Standards\n\nEnter only with multi-signal confluence.",
            "decision_process": "# Decision Process\n\nAssess risk first, then output structured decisions."
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
    use serde_json::json;

    use super::{default_strategy_config, validate_strategy_data_template};

    #[test]
    fn default_fixed_tp_sl_values_are_empty() {
        let config = default_strategy_config();
        let tp_sl = config.get("tp_sl").expect("default TP/SL config");
        assert!(
            tp_sl
                .pointer("/take_profit/pnl_rate")
                .is_some_and(|value| value.is_null())
        );
        assert!(
            tp_sl
                .pointer("/stop_loss/pnl_rate")
                .is_some_and(|value| value.is_null())
        );
    }

    #[test]
    fn default_strategy_includes_a_valid_primary_kline_data_item() {
        let config = default_strategy_config();

        let template = validate_strategy_data_template(&config).expect("default data template");
        let primary = template.items.first().expect("primary kline item");

        assert_eq!(primary.timeframes, vec!["5m"]);
        assert_eq!(primary.count, 100);
    }

    #[test]
    fn data_template_rejects_an_empty_item_list() {
        let config = json!({
            "data_template": {
                "schema_version": 1,
                "items": []
            }
        });

        let error = validate_strategy_data_template(&config).expect_err("empty template invalid");

        assert_eq!(error, "data_template requires at least one data item");
    }

    #[test]
    fn data_template_accepts_multi_timeframe_klines_and_derived_indicators() {
        let config = json!({
            "data_template": {
                "schema_version": 1,
                "items": [
                    {
                        "id": "primary",
                        "type": "raw_kline",
                        "timeframes": ["5m", "1h"],
                        "count": 100,
                        "closed_only": true
                    },
                    {
                        "id": "rsi",
                        "type": "indicator",
                        "timeframes": ["5m", "1h"],
                        "indicator": "rsi",
                        "params": { "period": 14 },
                        "output_mode": "latest"
                    }
                ]
            }
        });

        let template = validate_strategy_data_template(&config).expect("valid template");
        assert_eq!(template.items.len(), 2);
    }

    #[test]
    fn data_template_drops_legacy_presentation_fields() {
        let config = json!({
            "data_template": {
                "schema_version": 1,
                "items": [{
                    "id": "primary",
                    "type": "raw_kline",
                    "name": "Primary Kline",
                    "enabled": false,
                    "timeframes": ["5m"],
                    "count": 100
                }]
            }
        });

        let template = validate_strategy_data_template(&config).expect("legacy field is ignored");
        let normalized = super::data_template_value(&template);
        assert!(normalized["items"][0].get("enabled").is_none());
        assert!(normalized["items"][0].get("name").is_none());
    }

    #[test]
    fn data_template_rejects_multiple_raw_kline_items() {
        let config = json!({
            "data_template": {
                "schema_version": 1,
                "items": [
                    { "id": "primary", "type": "raw_kline", "timeframes": ["5m"], "count": 100 },
                    { "id": "secondary", "type": "raw_kline", "timeframes": ["1h"], "count": 100 }
                ]
            }
        });

        let error =
            validate_strategy_data_template(&config).expect_err("multiple raw Klines invalid");
        assert_eq!(error, "data_template supports at most one raw_kline item");
    }

    #[test]
    fn data_template_accepts_indicators_without_raw_kline_output() {
        let config = json!({
            "data_template": {
                "schema_version": 1,
                "items": [{
                    "id": "rsi",
                    "type": "indicator",
                    "timeframes": ["15m"],
                    "indicator": "rsi",
                    "params": { "period": 14 },
                    "output_mode": "latest"
                }]
            }
        });

        assert!(validate_strategy_data_template(&config).is_ok());
    }

    #[test]
    fn parses_ai_decisions_json_array_and_object() {
        use super::parse_ai_decisions;

        let array_input = r#"[{"action": "LONG", "symbol": "BTCUSDT", "confidence": 85, "reasoning": "Breakout"}]"#;
        let parsed = parse_ai_decisions(array_input);
        assert!(parsed.is_array());
        assert_eq!(parsed[0]["action"], "LONG");

        let object_input = r#"```json
        {"action": "SHORT", "symbol": "ETHUSDT", "confidence": 75, "reasoning": "Resistance"}
        ```"#;
        let parsed_obj = parse_ai_decisions(object_input);
        assert!(parsed_obj.is_array());
        assert_eq!(parsed_obj[0]["action"], "SHORT");
    }
}
