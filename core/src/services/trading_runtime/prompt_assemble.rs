//! Prompt assembly: parse → serialize to TOML → concatenate with static text.
//!
//! `build_system_prompt_from_config` is the single source of truth shared by
//! live trading, preview, test run, and backtesting.
//!
//! Design: fixed strategy parameters are serialized to TOML and embedded in
//! the system prompt; variable market/account data is serialized to TOML and
//! embedded in the user prompt.

use super::account_sim::AccountMetrics;
use super::models::{MarketState, PositionView, TraderRuntimeConfig};
use super::prompt_data::{PositionEntryView, StrategyPromptData, TradingPromptData};

// ---------------------------------------------------------------------------
// Static text blocks
// ---------------------------------------------------------------------------

const HEADER: &str =
    "You are QUANTAURA trading AI, a professional crypto perpetual futures trading system.";

const ACTION_DEFINITIONS: &str = "\
## Action Definitions\n\
You must choose exactly one of three actions:\n\
- **LONG**: Open a long position (if currently short, close short first then open long; if already long, hold)\n\
- **SHORT**: Open a short position (if currently long, close long first then open short; if already short, hold)\n\
- **NO ACTION**: Do nothing (observe or maintain current positions)\n\n\
All trading symbols are perpetual futures contracts (USDT-margined). Spot and delivery contracts are not supported.";

const RESPONSE_FORMAT: &str = "\
## Response Format\n\
Respond with a plain JSON object, no markdown:\n\
{\"action\": \"LONG\" | \"SHORT\" | \"NO ACTION\", \"confidence\": 0.0-1.0, \"reason\": \"your analysis\"}";

// ---------------------------------------------------------------------------
// System prompt
// ---------------------------------------------------------------------------

/// Build the system prompt from raw strategy config JSON.
///
/// This is the single source of truth for system prompt construction, shared by:
/// - Live trading (`generate_ai_decision`)
/// - Strategy preview (`preview_prompt`)
/// - Strategy test run (`test_run`)
/// - Backtesting (`process_cycle`)
pub fn build_system_prompt_from_config(
    config: &serde_json::Value,
    is_cross_margin: bool,
    custom_prompt: &str,
    override_base_prompt: bool,
) -> String {
    let data = StrategyPromptData::from_config(config, is_cross_margin);
    let toml_str = toml::to_string_pretty(&data.to_view()).unwrap_or_default();

    let mut parts: Vec<String> = vec![
        HEADER.to_string(),
        format!("## Strategy Configuration\n```toml\n{toml_str}```"),
        ACTION_DEFINITIONS.to_string(),
        RESPONSE_FORMAT.to_string(),
    ];

    if override_base_prompt && !custom_prompt.trim().is_empty() {
        parts.push(format!(
            "## Additional Instructions\n{}",
            custom_prompt.trim()
        ));
    }

    parts.join("\n\n")
}

/// Convenience wrapper: build system prompt from a loaded runtime config.
pub fn build_system_prompt(cfg: &TraderRuntimeConfig) -> String {
    build_system_prompt_from_config(
        &cfg.strategy_config,
        cfg.is_cross_margin,
        &cfg.custom_prompt,
        cfg.override_base_prompt,
    )
}

// ---------------------------------------------------------------------------
// User prompt (trading)
// ---------------------------------------------------------------------------

/// Build the user-side trading prompt from live market state and account metrics.
/// `now_ts` is the cycle timestamp (epoch seconds) so backtests report the
/// simulated clock instead of wall-clock time.
pub fn build_trading_prompt(
    symbol: &str,
    m: &MarketState,
    metrics: &AccountMetrics,
    open_positions: &[PositionView],
    now_ts: i64,
    strategy_data_context: &str,
) -> String {
    let price_change_pct = if m.prev_price.abs() > f64::EPSILON {
        (m.price - m.prev_price) / m.prev_price * 100.0
    } else {
        0.0
    };

    let current_time = chrono::DateTime::from_timestamp(now_ts, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_default();

    let positions: Vec<PositionEntryView> = open_positions
        .iter()
        .map(|p| PositionEntryView {
            symbol: p.symbol.clone(),
            side: p.side.clone(),
            quantity: p.quantity,
            entry_price: p.entry_price,
            mark_price: p.mark_price,
            unrealized_pnl: (p.mark_price - p.entry_price)
                * p.quantity
                * if p.side == "LONG" { 1.0 } else { -1.0 },
        })
        .collect();

    let data = TradingPromptData {
        current_time,
        symbol: symbol.to_string(),
        price: m.price,
        prev_price: m.prev_price,
        price_change_pct,
        volatility_pct: m.volatility * 100.0,
        strategy_data_context: strategy_data_context.to_string(),
        total_balance: metrics.total_balance,
        available_balance: metrics.available_balance,
        used_margin: metrics.used_margin,
        unrealized_pnl: metrics.unrealized_pnl,
        realized_pnl: metrics.realized_pnl,
        margin_usage_pct: metrics.margin_used_ratio * 100.0,
        positions,
    };

    let toml_str = toml::to_string_pretty(&data).unwrap_or_default();

    let mut parts: Vec<String> = vec![format!(
        "Analyze the following market data and decide: LONG, SHORT, or NO ACTION.\n\n```toml\n{toml_str}```"
    )];

    if !strategy_data_context.trim().is_empty() {
        parts.push(format!("## Strategy Data Context\n{strategy_data_context}"));
    }

    parts.join("\n\n")
}

// ---------------------------------------------------------------------------
// User prompt (test run)
// ---------------------------------------------------------------------------

/// Build the user-side prompt for strategy test runs.
/// The strategy config is already in the system prompt as TOML,
/// so the user prompt only needs to specify what to analyze.
pub fn build_test_run_prompt(ai_model_id: &str, symbols_str: &str) -> String {
    format!(
        "Model: {ai_model_id}\n\
         Symbols to analyze: {symbols_str}\n\n\
         Based on the strategy configuration in the system prompt and current market conditions, \
         provide trading decisions for each symbol: {symbols_str}.\n\
         Return a JSON array only."
    )
}

#[cfg(test)]
mod tests {
    use super::build_system_prompt_from_config;
    use super::build_trading_prompt;
    use crate::services::trading_runtime::account_sim::AccountMetrics;
    use crate::services::trading_runtime::models::{MarketState, PositionView};
    use serde_json::json;

    #[test]
    fn system_prompt_contains_toml_config() {
        let prompt = build_system_prompt_from_config(
            &json!({
                "symbols": [{"symbol": "BTCUSDT", "leverage": 10, "fixed_cost": 100}],
                "risk_control": {"max_positions": 3, "margin_mode": "cross"},
                "tp_sl": {
                    "take_profit": {"mode": "fixed", "pnl_rate": 0.2},
                    "stop_loss": {"mode": "custom", "custom_prompt": "Close on support"}
                }
            }),
            true,
            "",
            false,
        );

        assert!(prompt.contains("You are QUANTAURA trading AI"));
        assert!(prompt.contains("## Strategy Configuration"));
        assert!(prompt.contains("```toml"));
        assert!(prompt.contains("[[symbols]]"));
        assert!(prompt.contains("BTCUSDT"));
        assert!(prompt.contains("[risk_control]"));
        assert!(prompt.contains("[tp_sl.take_profit]"));
        assert!(prompt.contains("mode = \"fixed\""));
        assert!(prompt.contains("unrealized_pnl_rate = 0.2"));
        assert!(prompt.contains("[tp_sl.stop_loss]"));
        assert!(prompt.contains("mode = \"custom\""));
        assert!(prompt.contains("Close on support"));
    }

    #[test]
    fn system_prompt_contains_action_definitions_and_response_format() {
        let prompt = build_system_prompt_from_config(&json!({}), true, "", false);

        assert!(prompt.contains("## Action Definitions"));
        assert!(prompt.contains("LONG"));
        assert!(prompt.contains("SHORT"));
        assert!(prompt.contains("NO ACTION"));
        assert!(prompt.contains("## Response Format"));
    }

    #[test]
    fn system_prompt_includes_custom_instructions_when_override() {
        let prompt =
            build_system_prompt_from_config(&json!({}), true, "Always hedge with options", true);
        assert!(prompt.contains("## Additional Instructions"));
        assert!(prompt.contains("Always hedge with options"));
    }

    #[test]
    fn system_prompt_excludes_custom_instructions_when_not_override() {
        let prompt =
            build_system_prompt_from_config(&json!({}), true, "Always hedge with options", false);
        assert!(!prompt.contains("## Additional Instructions"));
    }

    #[test]
    fn system_prompt_includes_prompt_sections_when_configured() {
        let prompt = build_system_prompt_from_config(
            &json!({
                "prompt_sections": {
                    "role_definition": "## Role\nYou are an expert trader",
                    "decision_process": "## Decision Process\n1. Check trend"
                }
            }),
            true,
            "",
            false,
        );
        assert!(prompt.contains("## Role"));
        assert!(prompt.contains("You are an expert trader"));
        assert!(prompt.contains("## Decision Process"));
        assert!(prompt.contains("1. Check trend"));
    }

    #[test]
    fn system_prompt_excludes_empty_prompt_sections() {
        let prompt = build_system_prompt_from_config(&json!({}), true, "", false);
        assert!(!prompt.contains("prompt_sections"));
    }

    #[test]
    fn trading_prompt_contains_time_positions_and_no_duplicated_fixed_params() {
        let m = MarketState {
            price: 45000.0,
            prev_price: 44800.0,
            volatility: 0.02,
        };
        let metrics = AccountMetrics {
            total_balance: 10000.0,
            available_balance: 8000.0,
            used_margin: 2000.0,
            unrealized_pnl: 50.0,
            realized_pnl: -20.0,
            margin_used_ratio: 0.2,
        };
        let positions = vec![PositionView {
            id: "pos-1".to_string(),
            symbol: "BTCUSDT".to_string(),
            side: "LONG".to_string(),
            quantity: 0.5,
            entry_price: 44000.0,
            mark_price: 45000.0,
            leverage: 10,
            opened_at: 0,
        }];

        let prompt = build_trading_prompt(
            "BTCUSDT",
            &m,
            &metrics,
            &positions,
            1_755_436_200,
            "## Strategy Data Context\nklines here",
        );

        assert!(prompt.contains("current_time = \"2025-08-17"));
        assert!(prompt.contains("UTC"));
        assert!(prompt.contains("[[positions]]"));
        assert!(prompt.contains("unrealized_pnl = 50"));
        assert!(prompt.contains("## Strategy Data Context"));
        assert!(!prompt.contains("leverage"));
        assert!(!prompt.contains("margin_mode"));
        assert!(!prompt.contains("Respond with JSON"));
    }
}
