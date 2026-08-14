//! Prompt assembly: parse → serialize to TOML → concatenate with static text.
//!
//! `build_system_prompt_from_config` is the single source of truth shared by
//! live trading, preview, test run, and backtesting.
//!
//! Design: fixed strategy parameters are serialized to TOML and embedded in
//! the system prompt; variable market/account data is serialized to TOML and
//! embedded in the user prompt.

use super::account_sim::AccountMetrics;
use super::config_loaders::leverage_for_symbol;
use super::models::{MarketState, TraderRuntimeConfig};
use super::prompt_data::{StrategyPromptData, TradingPromptData};

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
pub fn build_trading_prompt(
    symbol: &str,
    m: &MarketState,
    metrics: &AccountMetrics,
    cfg: &TraderRuntimeConfig,
    strategy_data_context: &str,
) -> String {
    let price_change_pct = if m.prev_price.abs() > f64::EPSILON {
        (m.price - m.prev_price) / m.prev_price * 100.0
    } else {
        0.0
    };

    let data = TradingPromptData {
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
        leverage: leverage_for_symbol(cfg, symbol),
        margin_mode: if cfg.is_cross_margin {
            "Cross"
        } else {
            "Isolated"
        },
    };

    let toml_str = toml::to_string_pretty(&data).unwrap_or_default();

    let mut parts: Vec<String> = vec![format!(
        "Analyze the following market data and decide: LONG, SHORT, or NO ACTION.\n\n```toml\n{toml_str}```"
    )];

    if !strategy_data_context.trim().is_empty() {
        parts.push(format!("## Strategy Data Context\n{strategy_data_context}"));
    }

    parts.push(
        "Respond with JSON: {\"action\":\"LONG|SHORT|NO ACTION\",\"confidence\":0.0-1.0,\"reason\":\"...\"}"
            .to_string(),
    );

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
    use serde_json::json;

    #[test]
    fn system_prompt_contains_toml_config() {
        let prompt = build_system_prompt_from_config(
            &json!({
                "symbols": [{"symbol": "BTCUSDT", "leverage": 10, "fixed_cost": 100}],
                "risk_control": {"max_positions": 3, "margin_mode": "cross"},
                "tp_sl": {
                    "take_profit": {"mode": "fixed", "pnl_rate": 0.2},
                    "stop_loss": {"mode": "custom", "custom_prompt": "Close at support"}
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
        assert!(prompt.contains("rate = 0.2"));
        assert!(prompt.contains("[tp_sl.stop_loss]"));
        assert!(prompt.contains("mode = \"custom\""));
        assert!(prompt.contains("Close at support"));
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
}
