//! Typed intermediate structures for prompt construction.
//!
//! All JSON traversal is centralized here. The structs are serialized to TOML
//! in `prompt_assemble.rs` to produce the final prompt text.

use crate::services::data_template::validate_strategy_data_template;
use serde::Serialize;

// ---------------------------------------------------------------------------
// Exit-rule types
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub(crate) enum ExitRuleKind {
    TakeProfit,
    StopLoss,
}

impl ExitRuleKind {
    fn config_key(self) -> &'static str {
        match self {
            Self::TakeProfit => "take_profit",
            Self::StopLoss => "stop_loss",
        }
    }
}

/// Parsed exit rule from strategy config JSON.
#[derive(Clone, Debug)]
pub enum ExitRule {
    /// Rule is absent, mode is unknown, or mode is "fixed" without a rate.
    NotConfigured,
    /// Mode is "fixed" but the `pnl_rate` field is missing.
    FixedUnconfigured,
    /// Mode is "fixed" with a valid rate.
    Fixed { rate: f64 },
    /// Mode is "custom". `prompt` may be empty.
    Custom { prompt: String },
}

impl ExitRule {
    fn from_json(rule: Option<&serde_json::Value>) -> Self {
        let Some(rule) = rule else {
            return Self::NotConfigured;
        };
        let mode = rule
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("unconfigured");
        if mode.eq_ignore_ascii_case("fixed") {
            return match rule.get("pnl_rate").and_then(|v| v.as_f64()) {
                Some(rate) => Self::Fixed { rate },
                None => Self::FixedUnconfigured,
            };
        }
        if mode.eq_ignore_ascii_case("custom") {
            let prompt = rule
                .get("custom_prompt")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            return Self::Custom { prompt };
        }
        Self::NotConfigured
    }
}

/// Serializable view of an exit rule for TOML output.
/// `unrealized_pnl_rate` is the trigger threshold expressed as PnL relative
/// to position margin (e.g. 2.0 = +200% on margin, -2.0 = -200% on margin).
#[derive(Serialize)]
pub struct ExitRuleView {
    pub mode: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unrealized_pnl_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

impl ExitRule {
    pub fn to_view(&self) -> ExitRuleView {
        match self {
            Self::NotConfigured => ExitRuleView {
                mode: "not_configured",
                unrealized_pnl_rate: None,
                prompt: None,
            },
            Self::FixedUnconfigured => ExitRuleView {
                mode: "fixed",
                unrealized_pnl_rate: None,
                prompt: None,
            },
            Self::Fixed { rate } => ExitRuleView {
                mode: "fixed",
                unrealized_pnl_rate: Some(*rate),
                prompt: None,
            },
            Self::Custom { prompt } => ExitRuleView {
                mode: "custom",
                unrealized_pnl_rate: None,
                prompt: Some(prompt.clone()),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Section data structs
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
pub struct SymbolEntry {
    pub symbol: String,
    pub leverage: i64,
    pub cost_desc: String,
}

#[derive(Clone, Debug)]
pub struct RiskControlData {
    pub max_positions: i64,
    pub max_margin_usage: f64,
    pub min_position_size: f64,
    pub min_risk_reward: f64,
    pub min_confidence: f64,
}

/// Serializable view of risk control for TOML output.
/// Ratio fields are converted to percentages; unit-suffixed names make
/// semantics explicit for the AI.
#[derive(Serialize)]
pub struct RiskControlView {
    pub max_positions: i64,
    pub max_margin_usage_pct: f64,
    pub min_position_size_usdt: f64,
    pub min_risk_reward: f64,
    pub min_confidence: f64,
}

impl RiskControlData {
    pub fn to_view(&self) -> RiskControlView {
        RiskControlView {
            max_positions: self.max_positions,
            max_margin_usage_pct: self.max_margin_usage * 100.0,
            min_position_size_usdt: self.min_position_size,
            min_risk_reward: self.min_risk_reward,
            min_confidence: self.min_confidence,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct TpSlData {
    pub take_profit: ExitRule,
    pub stop_loss: ExitRule,
}

/// Serializable view of TP/SL config for TOML output.
#[derive(Serialize)]
pub struct TpSlView {
    pub take_profit: ExitRuleView,
    pub stop_loss: ExitRuleView,
}

impl TpSlData {
    pub fn to_view(&self) -> TpSlView {
        TpSlView {
            take_profit: self.take_profit.to_view(),
            stop_loss: self.stop_loss.to_view(),
        }
    }
}

impl Default for ExitRule {
    fn default() -> Self {
        Self::NotConfigured
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct PromptSections {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub role_definition: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub trading_frequency: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub entry_standards: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub decision_process: String,
}

/// Strongly-typed view of the strategy config, parsed once and consumed
/// by prompt assembly.
#[derive(Clone, Debug)]
pub struct StrategyPromptData {
    pub margin_mode: &'static str,
    pub symbols: Vec<SymbolEntry>,
    pub risk_control: RiskControlData,
    pub tp_sl: TpSlData,
    pub prompt_sections: PromptSections,
    pub primary_timeframe: String,
}

/// Serializable view of the full strategy config for TOML output.
/// This is what gets embedded in the system prompt.
#[derive(Serialize)]
pub struct StrategyConfigView {
    pub margin_mode: String,
    pub primary_timeframe: String,
    pub symbols: Vec<SymbolEntry>,
    pub risk_control: RiskControlView,
    pub tp_sl: TpSlView,
    #[serde(skip_serializing_if = "is_prompt_sections_empty")]
    pub prompt_sections: PromptSections,
}

fn is_prompt_sections_empty(sections: &PromptSections) -> bool {
    sections.role_definition.is_empty()
        && sections.trading_frequency.is_empty()
        && sections.entry_standards.is_empty()
        && sections.decision_process.is_empty()
}

impl StrategyPromptData {
    pub fn to_view(&self) -> StrategyConfigView {
        StrategyConfigView {
            margin_mode: self.margin_mode.to_string(),
            primary_timeframe: self.primary_timeframe.clone(),
            symbols: self.symbols.clone(),
            risk_control: self.risk_control.to_view(),
            tp_sl: self.tp_sl.to_view(),
            prompt_sections: self.prompt_sections.clone(),
        }
    }
}

impl StrategyPromptData {
    /// Parse all fields from the strategy config JSON.
    /// This is the single place that touches `serde_json::Value`.
    pub fn from_config(config: &serde_json::Value, is_cross_margin: bool) -> Self {
        let sc = config;

        // --- Symbols ---
        let symbols = parse_symbols(sc);

        // --- Risk control ---
        let rc = sc.get("risk_control");
        let risk_control = RiskControlData {
            max_positions: rc
                .and_then(|r| r.get("max_positions"))
                .and_then(|v| v.as_i64())
                .unwrap_or(3),
            max_margin_usage: rc
                .and_then(|r| r.get("max_margin_usage"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.9),
            min_position_size: rc
                .and_then(|r| r.get("min_position_size"))
                .and_then(|v| v.as_f64())
                .unwrap_or(20.0),
            min_risk_reward: rc
                .and_then(|r| r.get("min_risk_reward_ratio"))
                .and_then(|v| v.as_f64())
                .unwrap_or(1.5),
            min_confidence: rc
                .and_then(|r| r.get("min_confidence"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.6),
        };

        // --- Margin mode (prefer config value, fall back to is_cross_margin) ---
        let margin_mode = rc
            .and_then(|r| r.get("margin_mode"))
            .and_then(|v| v.as_str())
            .map(|m| if m.eq_ignore_ascii_case("cross") { "Cross" } else { "Isolated" })
            .unwrap_or_else(|| if is_cross_margin { "Cross" } else { "Isolated" });

        // --- TP/SL rules ---
        let tp_sl = parse_tp_sl(sc);

        // --- Prompt sections ---
        let prompt_sections = PromptSections {
            role_definition: sc
                .pointer("/prompt_sections/role_definition")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            trading_frequency: sc
                .pointer("/prompt_sections/trading_frequency")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            entry_standards: sc
                .pointer("/prompt_sections/entry_standards")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            decision_process: sc
                .pointer("/prompt_sections/decision_process")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        };

        // --- Data template: primary timeframe ---
        let primary_timeframe = parse_data_template(sc);

        Self {
            margin_mode,
            symbols,
            risk_control,
            tp_sl,
            prompt_sections,
            primary_timeframe,
        }
    }
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

fn parse_symbols(sc: &serde_json::Value) -> Vec<SymbolEntry> {
    let Some(symbols) = sc.get("symbols").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    symbols
        .iter()
        .filter_map(|s| {
            let symbol = s.get("symbol").and_then(|v| v.as_str()).unwrap_or("");
            if symbol.is_empty() {
                return None;
            }
            let leverage = s.get("leverage").and_then(|v| v.as_i64()).unwrap_or(5);
            let cost_desc = format_cost_desc(s);
            Some(SymbolEntry {
                symbol: symbol.to_string(),
                leverage,
                cost_desc,
            })
        })
        .collect()
}

fn format_cost_desc(s: &serde_json::Value) -> String {
    let fixed_cost = s.get("fixed_cost").and_then(|v| v.as_f64());
    if let Some(fixed) = fixed_cost {
        return format!("fixed ${:.0}", fixed);
    }
    let min_cost = s.get("min_cost").and_then(|v| v.as_f64());
    let max_cost = s.get("max_cost").and_then(|v| v.as_f64());
    let min = min_cost.map(|v| format!("{:.0}", v)).unwrap_or("-".into());
    let max = max_cost.map(|v| format!("{:.0}", v)).unwrap_or("-".into());
    format!("${}-${}", min, max)
}

fn parse_tp_sl(sc: &serde_json::Value) -> TpSlData {
    let tp_sl = sc.get("tp_sl");
    TpSlData {
        take_profit: ExitRule::from_json(
            tp_sl.and_then(|v| v.get(ExitRuleKind::TakeProfit.config_key())),
        ),
        stop_loss: ExitRule::from_json(
            tp_sl.and_then(|v| v.get(ExitRuleKind::StopLoss.config_key())),
        ),
    }
}

fn parse_data_template(sc: &serde_json::Value) -> String {
    validate_strategy_data_template(sc)
        .map(|template| {
            template
                .items
                .iter()
                .find(|item| item.item_type == "raw_kline")
                .and_then(|item| item.timeframes.first())
                .or_else(|| {
                    template
                        .items
                        .iter()
                        .find(|item| item.item_type == "indicator")
                        .and_then(|item| item.timeframes.first())
                })
                .cloned()
                .unwrap_or_else(|| "5m".to_string())
        })
        .unwrap_or_else(|_| "5m".to_string())
}

// ---------------------------------------------------------------------------
// Trading prompt data (user-side prompt)
// ---------------------------------------------------------------------------

/// Serializable view of an open position for the user prompt TOML.
#[derive(Clone, Debug, Serialize)]
pub struct PositionEntryView {
    pub symbol: String,
    pub side: String,
    pub quantity: f64,
    pub entry_price: f64,
    pub mark_price: f64,
    pub unrealized_pnl: f64,
}

/// Typed snapshot used to build the user-side trading prompt.
/// Constructed from live `MarketState` + `AccountMetrics` + open positions
/// during trading. Fixed strategy parameters (leverage, margin mode) live
/// only in the system prompt.
/// `strategy_data_context` is excluded from TOML serialization and appended
/// as a separate text section in the final prompt.
#[derive(Clone, Debug, Serialize)]
pub struct TradingPromptData {
    pub current_time: String,
    pub symbol: String,
    pub price: f64,
    pub prev_price: f64,
    pub price_change_pct: f64,
    pub volatility_pct: f64,
    #[serde(skip)]
    pub strategy_data_context: String,
    pub total_balance: f64,
    pub available_balance: f64,
    pub used_margin: f64,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
    pub margin_usage_pct: f64,
    pub positions: Vec<PositionEntryView>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_fixed_exit_rule() {
        let rule = ExitRule::from_json(Some(&json!({
            "mode": "fixed",
            "pnl_rate": 0.2
        })));
        assert!(matches!(rule, ExitRule::Fixed { rate } if (rate - 0.2).abs() < f64::EPSILON));
    }

    #[test]
    fn parse_custom_exit_rule() {
        let rule = ExitRule::from_json(Some(&json!({
            "mode": "custom",
            "custom_prompt": "Close on breakout"
        })));
        assert!(matches!(rule, ExitRule::Custom { .. }));
    }

    #[test]
    fn parse_missing_rule() {
        let rule = ExitRule::from_json(None);
        assert!(matches!(rule, ExitRule::NotConfigured));
    }

    #[test]
    fn parse_fixed_without_rate() {
        let rule = ExitRule::from_json(Some(&json!({
            "mode": "fixed",
            "pnl_rate": null
        })));
        assert!(matches!(rule, ExitRule::FixedUnconfigured));
    }

    #[test]
    fn strategy_prompt_data_extracts_all_fields() {
        let data = StrategyPromptData::from_config(
            &json!({
                "symbols": [
                    {"symbol": "BTCUSDT", "leverage": 10, "fixed_cost": 100}
                ],
                "risk_control": {
                    "max_positions": 5,
                    "max_margin_usage": 0.8,
                    "min_position_size": 50,
                    "min_risk_reward_ratio": 2.0,
                    "min_confidence": 0.7,
                    "margin_mode": "isolated"
                },
                "tp_sl": {
                    "take_profit": {"mode": "fixed", "pnl_rate": 0.3},
                    "stop_loss": {"mode": "custom", "custom_prompt": "Cut at support"}
                },
                "prompt_sections": {
                    "role_definition": "You are aggressive",
                    "trading_frequency": "Every 5m",
                    "entry_standards": "RSI < 30",
                    "decision_process": "Check momentum first"
                }
            }),
            false,
        );
        assert_eq!(data.symbols.len(), 1);
        assert_eq!(data.symbols[0].symbol, "BTCUSDT");
        assert_eq!(data.symbols[0].leverage, 10);
        assert_eq!(data.risk_control.max_positions, 5);
        assert_eq!(data.margin_mode, "Isolated");
        assert!(matches!(data.tp_sl.take_profit, ExitRule::Fixed { .. }));
        assert!(matches!(data.tp_sl.stop_loss, ExitRule::Custom { .. }));
        assert_eq!(data.prompt_sections.role_definition, "You are aggressive");
        assert_eq!(data.prompt_sections.decision_process, "Check momentum first");
    }

    #[test]
    fn strategy_config_view_serializes_to_toml() {
        let data = StrategyPromptData::from_config(
            &json!({
                "symbols": [
                    {"symbol": "BTCUSDT", "leverage": 10, "fixed_cost": 100}
                ],
                "risk_control": {
                    "max_positions": 5,
                    "max_margin_usage": 0.8,
                    "min_position_size": 50,
                    "min_risk_reward_ratio": 2.0,
                    "min_confidence": 0.7,
                    "margin_mode": "isolated"
                },
                "tp_sl": {
                    "take_profit": {"mode": "fixed", "pnl_rate": 0.3},
                    "stop_loss": {"mode": "custom", "custom_prompt": "Cut at support"}
                },
                "prompt_sections": {
                    "role_definition": "You are aggressive",
                    "trading_frequency": "Every 5m",
                    "entry_standards": "RSI < 30",
                    "decision_process": "Check momentum first"
                }
            }),
            false,
        );
        let view = data.to_view();
        let toml_str = toml::to_string_pretty(&view).unwrap();
        assert!(toml_str.contains("margin_mode = \"Isolated\""));
        assert!(toml_str.contains("[[symbols]]"));
        assert!(toml_str.contains("BTCUSDT"));
        assert!(toml_str.contains("[risk_control]"));
        assert!(toml_str.contains("max_margin_usage_pct = 80"));
        assert!(toml_str.contains("min_position_size_usdt = 50"));
        assert!(toml_str.contains("[tp_sl.take_profit]"));
        assert!(toml_str.contains("mode = \"fixed\""));
        assert!(toml_str.contains("unrealized_pnl_rate = 0.3"));
        assert!(toml_str.contains("[tp_sl.stop_loss]"));
        assert!(toml_str.contains("mode = \"custom\""));
        assert!(toml_str.contains("Cut at support"));
        assert!(toml_str.contains("[prompt_sections]"));
    }

    #[test]
    fn trading_prompt_data_skips_strategy_data_context_in_toml() {
        let data = TradingPromptData {
            current_time: "2026-08-17 14:30 UTC".to_string(),
            symbol: "BTCUSDT".to_string(),
            price: 45000.0,
            prev_price: 44800.0,
            price_change_pct: 0.45,
            volatility_pct: 2.3,
            strategy_data_context: "some multi-line\ntechnical data".to_string(),
            total_balance: 10000.0,
            available_balance: 8000.0,
            used_margin: 2000.0,
            unrealized_pnl: 150.0,
            realized_pnl: -50.0,
            margin_usage_pct: 20.0,
            positions: vec![PositionEntryView {
                symbol: "BTCUSDT".to_string(),
                side: "LONG".to_string(),
                quantity: 0.5,
                entry_price: 44000.0,
                mark_price: 45000.0,
                unrealized_pnl: 50.0,
            }],
        };
        let toml_str = toml::to_string_pretty(&data).unwrap();
        assert!(toml_str.contains("current_time = \"2026-08-17 14:30 UTC\""));
        assert!(toml_str.contains("symbol = \"BTCUSDT\""));
        assert!(toml_str.contains("price = 45000"));
        assert!(toml_str.contains("[[positions]]"));
        assert!(toml_str.contains("side = \"LONG\""));
        assert!(toml_str.contains("unrealized_pnl = 50"));
        assert!(!toml_str.contains("strategy_data_context"));
        assert!(!toml_str.contains("technical data"));
        assert!(!toml_str.contains("leverage"));
        assert!(!toml_str.contains("margin_mode"));
    }

    #[test]
    fn empty_prompt_sections_skipped_in_toml() {
        let data = StrategyPromptData::from_config(
            &json!({
                "symbols": [],
                "risk_control": {},
            }),
            true,
        );
        let view = data.to_view();
        let toml_str = toml::to_string_pretty(&view).unwrap();
        assert!(!toml_str.contains("prompt_sections"));
    }
}
