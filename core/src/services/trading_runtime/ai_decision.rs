use super::service::*;
use crate::repositories::trading::records::history::InsertTraderDecisionRecord;

pub async fn generate_ai_decision(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    symbol: &str,
    market: &HashMap<String, MarketState>,
    hard_risk_trigger: bool,
    risk_level: &str,
    trigger_source: &str,
    correlation_id: &str,
    metrics: &AccountMetrics,
    _now: i64,
) -> DecisionSignal {
    if hard_risk_trigger {
        warn!(
            "[AI_PROMPT] skipped — hard_risk_trigger=true trader={} symbol={}",
            cfg.trader_id, symbol
        );
        let m = market.get(symbol).cloned().unwrap_or(MarketState {
            price: 100.0,
            prev_price: 100.0,
            volatility: 0.01,
        });
        let momentum = if m.prev_price.abs() > f64::EPSILON {
            (m.price - m.prev_price) / m.prev_price
        } else {
            0.0
        };
        return DecisionSignal {
            symbol: symbol.to_string(),
            action: "NO ACTION".to_string(),
            confidence: 0.95,
            reason: "risk control active: drawdown/margin threshold reached".to_string(),
            timeframe: "3m",
            price: m.price,
            momentum,
            risk_level: risk_level.to_string(),
            trigger_source: trigger_source.to_string(),
            action_taken: "hold-risk-guard".to_string(),
            correlation_id: correlation_id.to_string(),
            prompt: String::new(),
            system_prompt: None,
        };
    }

    let m = match market.get(symbol) {
        Some(v) => v.clone(),
        None => {
            warn!(
                "[AI_PROMPT] skipped — no market data trader={} symbol={}",
                cfg.trader_id, symbol
            );
            return DecisionSignal {
                symbol: symbol.to_string(),
                action: "NO ACTION".to_string(),
                confidence: 0.5,
                reason: "no market data".to_string(),
                timeframe: "3m",
                price: 0.0,
                momentum: 0.0,
                risk_level: risk_level.to_string(),
                trigger_source: trigger_source.to_string(),
                action_taken: "hold-no-data".to_string(),
                correlation_id: correlation_id.to_string(),
                prompt: String::new(),
                system_prompt: None,
            };
        }
    };

    let momentum = if m.prev_price.abs() > f64::EPSILON {
        (m.price - m.prev_price) / m.prev_price
    } else {
        0.0
    };

    let prompt = build_trading_prompt(symbol, &m, metrics, cfg);
    let system_prompt = build_system_prompt(cfg);
    let system_prompt_owned = if cfg.override_base_prompt && !cfg.custom_prompt.trim().is_empty() {
        Some(cfg.custom_prompt.clone())
    } else {
        Some(system_prompt)
    };

    // Always publish the prompt to realtime clients so users can see what's sent to the AI
    info!(
        "[AI_PROMPT] publishing prompt event trader={} symbol={} prompt_len={}",
        cfg.trader_id,
        symbol,
        prompt.len()
    );
    state
        .realtime_hub
        .publish(crate::realtime::RealtimeEvent::AiPrompt {
            trader_id: cfg.trader_id.clone(),
            symbol: symbol.to_string(),
            prompt: prompt.clone(),
            system_prompt: system_prompt_owned.clone(),
        });

    if cfg.ai_api_key.trim().is_empty() || cfg.ai_model_id.trim().is_empty() {
        return generate_fallback_decision(
            symbol,
            &m,
            hard_risk_trigger,
            risk_level,
            trigger_source,
            correlation_id,
            prompt,
            system_prompt_owned,
        );
    }

    let user_message = LlmMessage {
        role: "user".to_string(),
        content: prompt.clone(),
    };

    let custom_system = system_prompt_owned.as_deref();

    // Create an mpsc channel for streaming LLM response chunks.
    // Each chunk is forwarded to realtime clients as an AiStreamChunk SSE event
    // so the frontend can display the AI's response character-by-character.
    let (chunk_tx, mut chunk_rx) =
        tokio::sync::mpsc::unbounded_channel::<String>();

    let realtime_hub = state.realtime_hub.clone();
    let stream_trader_id = cfg.trader_id.clone();
    let stream_symbol = symbol.to_string();
    let stream_corr_id = correlation_id.to_string();
    tokio::spawn(async move {
        while let Some(chunk) = chunk_rx.recv().await {
            realtime_hub.publish(crate::realtime::RealtimeEvent::AiStreamChunk {
                trader_id: stream_trader_id.clone(),
                symbol: stream_symbol.clone(),
                chunk,
                correlation_id: stream_corr_id.clone(),
            });
        }
    });

    match state
        .llm_service
        .chat_stream_with_config(
            cfg.ai_provider_type.clone(),
            cfg.ai_api_key.clone(),
            cfg.ai_model_name.clone(),
            cfg.ai_base_url.clone(),
            vec![user_message],
            custom_system,
            chunk_tx,
        )
        .await
    {
        Ok(response) => {
            let decision = parse_ai_response(&response);
            DecisionSignal {
                symbol: symbol.to_string(),
                action: decision.action,
                confidence: decision.confidence,
                reason: decision.reason,
                timeframe: "3m",
                price: m.price,
                momentum,
                risk_level: risk_level.to_string(),
                trigger_source: trigger_source.to_string(),
                action_taken: format!("ai-{}-{}", cfg.ai_model_id, correlation_id),
                correlation_id: correlation_id.to_string(),
                prompt,
                system_prompt: system_prompt_owned,
            }
        }
        Err(e) => {
            warn!("AI chat failed for {}: {}", symbol, e);
            generate_fallback_decision(
                symbol,
                &m,
                hard_risk_trigger,
                risk_level,
                trigger_source,
                correlation_id,
                prompt,
                system_prompt_owned,
            )
        }
    }
}

/// Build a comprehensive system prompt from the full strategy configuration.
/// Includes strategy type, risk controls, TP/SL rules, prompt sections, symbols, and action definitions.
///
/// This is the single source of truth for system prompt construction, shared by:
/// - Live trading (`generate_ai_decision`)
/// - Strategy preview (`preview_prompt`)
/// - Strategy test run (`test_run`)
/// - Backtesting (`call_llm`)
pub fn build_system_prompt(cfg: &TraderRuntimeConfig) -> String {
    build_system_prompt_from_config(
        &cfg.strategy_config,
        cfg.is_cross_margin,
        &cfg.custom_prompt,
        cfg.override_base_prompt,
    )
}

#[derive(Clone, Copy)]
enum ExitRulePromptKind {
    TakeProfit,
    StopLoss,
}

impl ExitRulePromptKind {
    fn config_key(self) -> &'static str {
        match self {
            Self::TakeProfit => "take_profit",
            Self::StopLoss => "stop_loss",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::TakeProfit => "Take-Profit",
            Self::StopLoss => "Stop-Loss",
        }
    }

    fn fixed_rate_prefix(self) -> &'static str {
        match self {
            Self::TakeProfit => "+",
            Self::StopLoss => "",
        }
    }
}

fn render_exit_rule(
    rule: Option<&serde_json::Value>,
    rule_kind: ExitRulePromptKind,
) -> (String, Option<String>) {
    let label = rule_kind.label();
    let Some(rule) = rule else {
        return (format!("{label}: Not configured in the form"), None);
    };
    let mode = rule
        .get("mode")
        .and_then(|value| value.as_str())
        .unwrap_or("unconfigured");
    if mode.eq_ignore_ascii_case("fixed") {
        let Some(rate) = rule.get("pnl_rate").and_then(|value| value.as_f64()) else {
            return (
                format!("{label}: Fixed value is not configured in the form"),
                None,
            );
        };
        return (
            format!(
                "{label}: Fixed {}{:.1}% Unrealized PnL Rate (exchange-hosted protection)",
                rule_kind.fixed_rate_prefix(),
                rate * 100.0
            ),
            None,
        );
    }
    if mode.eq_ignore_ascii_case("custom") {
        let prompt = rule
            .get("custom_prompt")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        let instruction = if prompt.is_empty() {
            format!("{label}: decide exit conditions from market analysis and risk")
        } else {
            prompt.to_string()
        };
        return (
            format!("{label}: Custom AI-driven"),
            Some(format!("{label}: {instruction}")),
        );
    }
    (
        format!("{label}: Mode is not configured in the form"),
        None,
    )
}

/// Core prompt builder that works purely from the strategy config JSON.
/// All callers (live trading, preview, test run, backtest) use this function
/// to ensure identical prompt construction logic.
pub fn build_system_prompt_from_config(
    config: &serde_json::Value,
    is_cross_margin: bool,
    custom_prompt: &str,
    override_base_prompt: bool,
) -> String {
    let sc = config;
    let is_zh = sc
        .get("language")
        .and_then(|v| v.as_str())
        .map(|s| s.eq_ignore_ascii_case("zh"))
        .unwrap_or(false);

    let prompt_variant = sc
        .get("prompt_variant")
        .and_then(|v| v.as_str())
        .unwrap_or("balanced");

    // --- Symbols config (parsed from JSON `symbols` array) ---
    let symbols_str = if let Some(symbols) = sc.get("symbols").and_then(|v| v.as_array()) {
        if symbols.is_empty() {
            "None configured".to_string()
        } else {
            symbols
                .iter()
                .filter_map(|s| {
                    let sym = s.get("symbol").and_then(|v| v.as_str()).unwrap_or("");
                    if sym.is_empty() {
                        return None;
                    }
                    let lev = s.get("leverage").and_then(|v| v.as_i64()).unwrap_or(5);
                    let fixed_cost = s.get("fixed_cost").and_then(|v| v.as_f64());
                    let min_cost = s.get("min_cost").and_then(|v| v.as_f64());
                    let max_cost = s.get("max_cost").and_then(|v| v.as_f64());
                    let cost = if let Some(fixed) = fixed_cost {
                        format!("fixed ${:.0}", fixed)
                    } else {
                        let min = min_cost.map(|v| format!("{:.0}", v)).unwrap_or("-".into());
                        let max = max_cost.map(|v| format!("{:.0}", v)).unwrap_or("-".into());
                        format!("${}-${}", min, max)
                    };
                    Some(format!("  - {} ({}x leverage, cost: {})", sym, lev, cost))
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
    } else {
        "None configured".to_string()
    };

    // --- Risk control ---
    let rc = sc.get("risk_control");
    let max_positions = rc
        .and_then(|r| r.get("max_positions"))
        .and_then(|v| v.as_i64())
        .unwrap_or(3);
    let max_margin_usage = rc
        .and_then(|r| r.get("max_margin_usage"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.9);
    let min_position_size = rc
        .and_then(|r| r.get("min_position_size"))
        .and_then(|v| v.as_f64())
        .unwrap_or(20.0);
    let min_risk_reward = rc
        .and_then(|r| r.get("min_risk_reward_ratio"))
        .and_then(|v| v.as_f64())
        .unwrap_or(1.5);
    let min_confidence = rc
        .and_then(|r| r.get("min_confidence"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.6);

    // --- Margin mode (prefer config value, fall back to is_cross_margin) ---
    let margin_mode = rc
        .and_then(|r| r.get("margin_mode"))
        .and_then(|v| v.as_str())
        .map(|m| if m.eq_ignore_ascii_case("cross") { "Cross" } else { "Isolated" })
        .unwrap_or_else(|| if is_cross_margin { "Cross" } else { "Isolated" });

    // --- TP/SL rules ---
    let tp_sl = sc.get("tp_sl");
    let (take_profit_rule, take_profit_instruction) = render_exit_rule(
        tp_sl.and_then(|value| value.get(ExitRulePromptKind::TakeProfit.config_key())),
        ExitRulePromptKind::TakeProfit,
    );
    let (stop_loss_rule, stop_loss_instruction) = render_exit_rule(
        tp_sl.and_then(|value| value.get(ExitRulePromptKind::StopLoss.config_key())),
        ExitRulePromptKind::StopLoss,
    );
    let tp_sl_section = format!("{}\n{}", take_profit_rule, stop_loss_rule);
    let tp_sl_instruction = [take_profit_instruction, stop_loss_instruction]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n");

    // --- Prompt sections ---
    let role_def = sc
        .pointer("/prompt_sections/role_definition")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let trading_freq = sc
        .pointer("/prompt_sections/trading_frequency")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let entry_standards = sc
        .pointer("/prompt_sections/entry_standards")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let decision_process = sc
        .pointer("/prompt_sections/decision_process")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // --- Indicators ---
    let indicators = sc.get("indicators");
    let primary_tf = indicators
        .and_then(|i| i.pointer("/klines/primary_timeframe"))
        .and_then(|v| v.as_str())
        .unwrap_or("3m");
    let enable_ema = indicators
        .and_then(|i| i.get("enable_ema"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let enable_macd = indicators
        .and_then(|i| i.get("enable_macd"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let enable_rsi = indicators
        .and_then(|i| i.get("enable_rsi"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let enable_atr = indicators
        .and_then(|i| i.get("enable_atr"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let mut indicator_list = Vec::new();
    if enable_ema { indicator_list.push("EMA"); }
    if enable_macd { indicator_list.push("MACD"); }
    if enable_rsi { indicator_list.push("RSI"); }
    if enable_atr { indicator_list.push("ATR"); }
    let indicators_str = if indicator_list.is_empty() {
        "None".to_string()
    } else {
        indicator_list.join(", ")
    };

    let header = if is_zh {
        "你是 QUANTAURA 交易AI，一个专业的加密货币永续合约交易系统。"
    } else {
        "You are QUANTAURA trading AI, a professional crypto perpetual futures trading system."
    };

    let action_desc = if is_zh {
        r#"## 动作定义
你必须从以下三个动作中选择一个：
- **LONG**: 开多仓（如果当前持有空仓，先平空再开多；如果已有多仓，则保持）
- **SHORT**: 开空仓（如果当前持有多仓，先平多再开空；如果已有空仓，则保持）
- **NO ACTION**: 不执行任何操作（观望或保持现有仓位）

所有交易标的均为永续合约（USDT本位），不支持现货和交割合约。"#
    } else {
        r#"## Action Definitions
You must choose exactly one of three actions:
- **LONG**: Open a long position (if currently short, close short first then open long; if already long, hold)
- **SHORT**: Open a short position (if currently long, close long first then open short; if already short, hold)
- **NO ACTION**: Do nothing (observe or maintain current positions)

All trading symbols are perpetual futures contracts (USDT-margined). Spot and delivery contracts are not supported."#
    };

    let response_format = if is_zh {
        r#"## 响应格式
返回纯JSON对象，不要包含markdown：
{"action": "LONG" | "SHORT" | "NO ACTION", "confidence": 0.0-1.0, "reason": "1-2句分析说明"}"#
    } else {
        r#"## Response Format
Respond with a plain JSON object, no markdown:
{"action": "LONG" | "SHORT" | "NO ACTION", "confidence": 0.0-1.0, "reason": "1-2 sentence analysis"}"#
    };

    let custom_prompt_section = if override_base_prompt && !custom_prompt.trim().is_empty() {
        format!("\n## Additional Instructions\n{}", custom_prompt.trim())
    } else {
        String::new()
    };

    let tp_sl_instruction_section = if !tp_sl_instruction.is_empty() {
        format!("\n## Take-Profit / Stop-Loss Guidance\n{}", tp_sl_instruction)
    } else {
        String::new()
    };

    // Build optional prompt sections (skip empty ones to avoid large gaps)
    let role_def_section = if !role_def.is_empty() {
        format!("\n{}\n", role_def)
    } else {
        String::new()
    };
    let trading_freq_section = if !trading_freq.is_empty() {
        format!("\n{}\n", trading_freq)
    } else {
        String::new()
    };
    let entry_standards_section = if !entry_standards.is_empty() {
        format!("\n{}\n", entry_standards)
    } else {
        String::new()
    };
    let decision_process_section = if !decision_process.is_empty() {
        format!("\n{}\n", decision_process)
    } else {
        String::new()
    };

    format!(
        r#"{header}

## Strategy Configuration
- Prompt Style: {prompt_variant}
- Margin Mode: {margin_mode}
- Primary Timeframe: {primary_tf}
- Technical Indicators: {indicators_str}

## Trading Symbols (Perpetual Futures)
{symbols_str}

## Risk Controls
- Max Positions: {max_positions}
- Max Margin Usage: {:.0}%
- Min Position Size: ${:.0}
- Min Risk/Reward Ratio: {:.1}
- Min Confidence Threshold: {:.2}

## Take-Profit / Stop-Loss Rules
{tp_sl_section}
{role_def_section}{trading_freq_section}{entry_standards_section}{decision_process_section}
{action_desc}
{tp_sl_instruction_section}
{response_format}{custom_prompt_section}"#,
        max_margin_usage * 100.0,
        min_position_size,
        min_risk_reward,
        min_confidence,
    )
}

pub fn build_trading_prompt(
    symbol: &str,
    m: &MarketState,
    metrics: &AccountMetrics,
    cfg: &TraderRuntimeConfig,
) -> String {
    let momentum = if m.prev_price.abs() > f64::EPSILON {
        (m.price - m.prev_price) / m.prev_price
    } else {
        0.0
    };

    let leverage = leverage_for_symbol(cfg, symbol);

    format!(
        r#"Analyze {} perpetual futures and decide: LONG, SHORT, or NO ACTION.

Market Data:
- Current Price: {:.2}
- Previous Price: {:.2}
- Price Change: {:.4}%
- Volatility: {:.4}%

Account Status:
- Total Balance: ${:.2}
- Available Balance: ${:.2}
- Used Margin: ${:.2}
- Unrealized PnL: ${:.2}
- Realized PnL: ${:.2}
- Margin Usage: {:.2}%

Position Info:
- Leverage: {}x
- Margin Mode: {}

Respond with JSON: {{"action":"LONG|SHORT|NO ACTION","confidence":0.0-1.0,"reason":"..."}}"#,
        symbol,
        m.price,
        m.prev_price,
        momentum * 100.0,
        m.volatility * 100.0,
        metrics.total_balance,
        metrics.available_balance,
        metrics.used_margin,
        metrics.unrealized_pnl,
        metrics.realized_pnl,
        metrics.margin_used_ratio * 100.0,
        leverage,
        if cfg.is_cross_margin { "Cross" } else { "Isolated" },
    )
}

pub fn generate_fallback_decision(
    symbol: &str,
    m: &MarketState,
    hard_risk_trigger: bool,
    risk_level: &str,
    trigger_source: &str,
    correlation_id: &str,
    prompt: String,
    system_prompt: Option<String>,
) -> DecisionSignal {
    let momentum = if m.prev_price.abs() > f64::EPSILON {
        (m.price - m.prev_price) / m.prev_price
    } else {
        0.0
    };

    if hard_risk_trigger {
        return DecisionSignal {
            symbol: symbol.to_string(),
            action: "NO ACTION".to_string(),
            confidence: 0.95,
            reason: "risk control active: drawdown/margin threshold reached".to_string(),
            timeframe: "3m",
            price: m.price,
            momentum,
            risk_level: risk_level.to_string(),
            trigger_source: trigger_source.to_string(),
            action_taken: "hold-risk-guard".to_string(),
            correlation_id: correlation_id.to_string(),
            prompt,
            system_prompt,
        };
    }

    let threshold = (0.0015 + m.volatility * 0.2).clamp(0.001, 0.01);

    if momentum > threshold {
        DecisionSignal {
            symbol: symbol.to_string(),
            action: "LONG".to_string(),
            confidence: (0.55 + (momentum / threshold).min(1.5) * 0.2).clamp(0.55, 0.9),
            reason: format!("uptrend momentum={:.4}", momentum),
            timeframe: "3m",
            price: m.price,
            momentum,
            risk_level: risk_level.to_string(),
            trigger_source: trigger_source.to_string(),
            action_taken: "open-long".to_string(),
            correlation_id: correlation_id.to_string(),
            prompt,
            system_prompt,
        }
    } else if momentum < -threshold {
        DecisionSignal {
            symbol: symbol.to_string(),
            action: "SHORT".to_string(),
            confidence: (0.55 + ((-momentum) / threshold).min(1.5) * 0.2).clamp(0.55, 0.9),
            reason: format!("downtrend momentum={:.4}", momentum),
            timeframe: "3m",
            price: m.price,
            momentum,
            risk_level: risk_level.to_string(),
            trigger_source: trigger_source.to_string(),
            action_taken: "open-short".to_string(),
            correlation_id: correlation_id.to_string(),
            prompt,
            system_prompt,
        }
    } else {
        DecisionSignal {
            symbol: symbol.to_string(),
            action: "NO ACTION".to_string(),
            confidence: 0.5,
            reason: format!("range momentum={:.4}", momentum),
            timeframe: "3m",
            price: m.price,
            momentum,
            risk_level: risk_level.to_string(),
            trigger_source: trigger_source.to_string(),
            action_taken: "hold-range".to_string(),
            correlation_id: correlation_id.to_string(),
            prompt,
            system_prompt,
        }
    }
}

pub async fn persist_decision(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    d: &DecisionSignal,
    m: &AccountMetrics,
    timing: DecisionTiming,
) -> Result<(), AppError> {
    let payload = json!({
        "price": d.price,
        "momentum": d.momentum,
        "equity": m.total_balance,
        "available_balance": m.available_balance,
        "used_margin": m.used_margin,
        "prompt_hint": cfg.custom_prompt,
        "prompt": d.prompt,
        "system_prompt": d.system_prompt,
        "risk_level": d.risk_level,
        "trigger_source": d.trigger_source,
        "action_taken": d.action_taken,
        "correlation_id": d.correlation_id,
        "cycle_started_at": timing.cycle_started_at,
        "decision_started_at": timing.decision_started_at,
        "completed_at": timing.completed_at
    })
    .to_string();

    state
        .trading_repo
        .insert_decision(InsertTraderDecisionRecord {
            id: Uuid::now_v7().to_string(),
            trader_id: cfg.trader_id.clone(),
            symbol: d.symbol.clone(),
            timeframe: d.timeframe.to_string(),
            decision: d.action.clone(),
            confidence: d.confidence,
            reason: d.reason.clone(),
            payload_json: payload,
            created_at: timing.completed_at,
        })
        .await?;

    Ok(())
}

#[derive(Debug, Clone)]
pub struct TradingDecision {
    pub action: String,
    pub confidence: f64,
    pub reason: String,
}

pub fn parse_ai_response(response: &str) -> TradingDecision {
    let lower = response.to_lowercase();

    // Try JSON parsing first
    if let Some(action) = extract_json_action(response) {
        let reason = extract_json_reason(response).unwrap_or_else(|| response.to_string());
        let confidence = extract_confidence(&lower).unwrap_or(0.7);
        return TradingDecision {
            action,
            confidence,
            reason,
        };
    }

    // Fallback: keyword matching
    let action = if lower.contains("long") && !lower.contains("no action") {
        "LONG".to_string()
    } else if lower.contains("short") && !lower.contains("no action") {
        "SHORT".to_string()
    } else if lower.contains("no action") || lower.contains("hold") || lower.contains("nothing") {
        "NO ACTION".to_string()
    } else {
        "NO ACTION".to_string()
    };

    let reason = extract_json_reason(response).unwrap_or_else(|| response.to_string());

    TradingDecision {
        action,
        confidence: extract_confidence(&lower).unwrap_or(0.7),
        reason,
    }
}

/// Attempt to extract the action from a JSON response.
fn extract_json_action(response: &str) -> Option<String> {
    let trimmed = response.trim();

    // Strip markdown code fences if present
    let json_str = if trimmed.starts_with("```") {
        let inner = trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        inner
    } else {
        trimmed
    };

    let parsed: serde_json::Value = serde_json::from_str(json_str).ok()?;

    if let Some(action) = parsed.get("action").and_then(|v| v.as_str()) {
        let normalized = action.trim().to_uppercase();
        return match normalized.as_str() {
            "LONG" | "BUY" => Some("LONG".to_string()),
            "SHORT" | "SELL" => Some("SHORT".to_string()),
            "NO ACTION" | "NOACTION" | "HOLD" | "NONE" | "WAIT" => Some("NO ACTION".to_string()),
            _ => Some("NO ACTION".to_string()),
        };
    }

    None
}

/// Attempt to extract the "reason" field from a JSON response.
/// Handles both standard JSON and JSON embedded in markdown code fences.
fn extract_json_reason(response: &str) -> Option<String> {
    let trimmed = response.trim();

    // Strip markdown code fences if present (```json ... ``` or ``` ... ```)
    let json_str = if trimmed.starts_with("```") {
        let inner = trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        inner
    } else {
        trimmed
    };

    // Try parsing as a JSON object
    let parsed: serde_json::Value = serde_json::from_str(json_str).ok()?;

    if let Some(reason) = parsed.get("reason").and_then(|v| v.as_str()) {
        return Some(reason.to_string());
    }
    // Also check "reasoning" as some models use that field name
    if let Some(reasoning) = parsed.get("reasoning").and_then(|v| v.as_str()) {
        return Some(reasoning.to_string());
    }

    None
}

fn extract_confidence(text: &str) -> Option<f64> {
    for token in text.split_whitespace() {
        let clean = token.trim_matches(|c: char| !c.is_ascii_digit() && c != '.');
        if let Ok(value) = clean.parse::<f64>() {
            if (0.0..=1.0).contains(&value) {
                return Some(value);
            }
            if (1.0..=100.0).contains(&value) {
                return Some(value / 100.0);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::build_system_prompt_from_config;

    #[test]
    fn system_prompt_renders_take_profit_and_stop_loss_modes_independently() {
        let prompt = build_system_prompt_from_config(
            &serde_json::json!({
                "tp_sl": {
                    "take_profit": {
                        "mode": "fixed",
                        "pnl_rate": 0.2,
                        "custom_prompt": null
                    },
                    "stop_loss": {
                        "mode": "custom",
                        "pnl_rate": null,
                        "custom_prompt": "Close when the market structure breaks."
                    }
                }
            }),
            false,
            "",
            false,
        );

        assert!(prompt.contains("Take-Profit: Fixed +20.0%"));
        assert!(prompt.contains("Stop-Loss: Custom AI-driven"));
        assert!(prompt.contains("Stop-Loss: Close when the market structure breaks."));
        assert!(!prompt.contains("Stop-Loss: Fixed"));
    }
}
