use super::data_context::load_trader_data_context;
use super::prompt_assemble::{build_system_prompt, build_trading_prompt};
use super::prompt_data::TimelineEntryView;
use super::service::*;
use crate::repositories::trading::records::history::InsertTraderDecisionRecord;

/// Calculate price momentum from current and previous prices.
fn momentum(m: &MarketState) -> f64 {
    if m.prev_price.abs() > f64::EPSILON {
        (m.price - m.prev_price) / m.prev_price
    } else {
        0.0
    }
}

/// Format an epoch-seconds timestamp as a compact timeline time (`MM-DD HH:MM`).
fn format_timeline_time(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.format("%m-%d %H:%M").to_string())
        .unwrap_or_default()
}

/// Load the trader's recent history as a chronological timeline (oldest first):
/// past AI analyses from the decisions table plus closed trades with their
/// realized results. Non-AI decision rows (CLOSE/SYSTEM) are already covered
/// by trade entries and are skipped to avoid duplication.
async fn load_recent_timeline(state: &SharedState, trader_id: &str) -> Vec<TimelineEntryView> {
    const HISTORY_LIMIT: i64 = 10;

    let decisions = state
        .trading_repo
        .decisions(trader_id, None, HISTORY_LIMIT, 0)
        .await
        .unwrap_or_default();
    let trades = state
        .trading_repo
        .trades(trader_id, HISTORY_LIMIT, 0)
        .await
        .unwrap_or_default();

    let mut entries: Vec<(i64, TimelineEntryView)> = Vec::new();
    for d in decisions {
        if !matches!(d.decision.as_str(), "LONG" | "SHORT" | "NO ACTION") {
            continue;
        }
        entries.push((
            d.created_at,
            TimelineEntryView::Analysis {
                time: format_timeline_time(d.created_at),
                symbol: d.symbol,
                action: d.decision,
                confidence: d.confidence,
                reason: d.reason,
            },
        ));
    }
    for t in trades {
        entries.push((
            t.closed_at,
            TimelineEntryView::Trade {
                time: format_timeline_time(t.closed_at),
                symbol: t.symbol,
                side: t.side,
                entry_price: t.entry_price,
                exit_price: t.exit_price,
                quantity: t.quantity,
                realized_pnl: t.realized_pnl,
                roi_pct: t.roi_pct,
            },
        ));
    }

    entries.sort_by_key(|(ts, _)| *ts);
    entries.into_iter().map(|(_, entry)| entry).collect()
}

pub async fn generate_ai_decision(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    symbol: &str,
    market: &HashMap<String, MarketState>,
    metrics: &AccountMetrics,
    open_positions: &[PositionView],
    now_ts: i64,
    hard_risk_trigger: bool,
    risk_level: &str,
    trigger_source: &str,
    correlation_id: &str,
    backtest_mode: bool,
) -> DecisionSignal {
    let timeframe = format!("{}m", cfg.scan_interval_minutes.max(1));

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
        if !backtest_mode {
            emit_runtime_event_best_effort(
                state,
                cfg,
                EVENT_RISK_GUARD_ACTIVE,
                symbol,
                "",
                risk_level,
                trigger_source,
                "Risk guard active: drawdown/margin threshold reached, holding position",
                correlation_id,
                json!({ "symbol": symbol, "risk_level": risk_level }),
                now_ts,
            )
            .await;
        }
        return DecisionSignal {
            symbol: symbol.to_string(),
            action: "NO ACTION".to_string(),
            confidence: 0.95,
            reason: String::new(),
            timeframe: timeframe.clone(),
            price: m.price,
            momentum: momentum(&m),
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
            if !backtest_mode {
                emit_runtime_event_best_effort(
                    state,
                    cfg,
                    EVENT_MARKET_DATA_UNAVAILABLE,
                    symbol,
                    "",
                    risk_level,
                    trigger_source,
                    "No market data available for symbol, holding position",
                    correlation_id,
                    json!({ "symbol": symbol }),
                    now_ts,
                )
                .await;
            }
            return DecisionSignal {
                symbol: symbol.to_string(),
                action: "NO ACTION".to_string(),
                confidence: 0.5,
                reason: String::new(),
                timeframe: timeframe.clone(),
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

    let momentum = momentum(&m);

    let data_context = match load_trader_data_context(state, cfg, symbol).await {
        Ok(context) => context,
        Err(err) => {
            warn!(
                "[AI_PROMPT] skipped - strategy data unavailable trader={} symbol={} err={}",
                cfg.trader_id, symbol, err
            );
            if !backtest_mode {
                emit_runtime_event_best_effort(
                    state,
                    cfg,
                    EVENT_MARKET_DATA_UNAVAILABLE,
                    symbol,
                    "",
                    risk_level,
                    "strategy_data_unavailable",
                    &format!("Strategy market data unavailable: {err}"),
                    correlation_id,
                    json!({ "symbol": symbol, "error": err.to_string() }),
                    now_ts,
                )
                .await;
            }
            return DecisionSignal {
                symbol: symbol.to_string(),
                action: "NO ACTION".to_string(),
                confidence: 0.5,
                reason: String::new(),
                timeframe,
                price: m.price,
                momentum,
                risk_level: risk_level.to_string(),
                trigger_source: "strategy_data_unavailable".to_string(),
                action_taken: "hold-data-unavailable".to_string(),
                correlation_id: correlation_id.to_string(),
                prompt: String::new(),
                system_prompt: None,
            };
        }
    };

    let timeline = load_recent_timeline(state, &cfg.trader_id).await;

    let prompt = build_trading_prompt(
        symbol,
        &m,
        metrics,
        open_positions,
        &timeline,
        now_ts,
        &data_context.rendered,
    );
    let system_prompt_owned = if cfg.override_base_prompt && !cfg.custom_prompt.trim().is_empty() {
        Some(cfg.custom_prompt.clone())
    } else {
        Some(build_system_prompt(cfg))
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
        if !backtest_mode {
            emit_runtime_event_best_effort(
                state,
                cfg,
                EVENT_AI_FALLBACK,
                symbol,
                "",
                risk_level,
                trigger_source,
                "AI not configured — decision derived from momentum heuristic",
                correlation_id,
                json!({ "symbol": symbol, "momentum": momentum }),
                now_ts,
            )
            .await;
        }
        return generate_fallback_decision(
            symbol,
            &m,
            risk_level,
            trigger_source,
            correlation_id,
            &timeframe,
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
    let (chunk_tx, mut chunk_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

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
                timeframe,
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
            if !backtest_mode {
                emit_runtime_event_best_effort(
                    state,
                    cfg,
                    EVENT_AI_FALLBACK,
                    symbol,
                    "",
                    risk_level,
                    trigger_source,
                    &format!("AI request failed ({e}) — decision derived from momentum heuristic"),
                    correlation_id,
                    json!({ "symbol": symbol, "momentum": momentum, "error": e.to_string() }),
                    now_ts,
                )
                .await;
            }
            generate_fallback_decision(
                symbol,
                &m,
                risk_level,
                trigger_source,
                correlation_id,
                &timeframe,
                prompt,
                system_prompt_owned,
            )
        }
    }
}

pub fn generate_fallback_decision(
    symbol: &str,
    m: &MarketState,
    risk_level: &str,
    trigger_source: &str,
    correlation_id: &str,
    timeframe: &str,
    prompt: String,
    system_prompt: Option<String>,
) -> DecisionSignal {
    let momentum = momentum(m);

    let threshold = (0.0015 + m.volatility * 0.2).clamp(0.001, 0.01);

    if momentum > threshold {
        DecisionSignal {
            symbol: symbol.to_string(),
            action: "LONG".to_string(),
            confidence: (0.55 + (momentum / threshold).min(1.5) * 0.2).clamp(0.55, 0.9),
            reason: String::new(),
            timeframe: timeframe.to_string(),
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
            reason: String::new(),
            timeframe: timeframe.to_string(),
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
            reason: String::new(),
            timeframe: timeframe.to_string(),
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
            timeframe: d.timeframe.clone(),
            decision: d.action.clone(),
            confidence: d.confidence,
            reason: d.reason.clone(),
            payload_json: payload,
            created_at: timing.completed_at,
        })
        .await?;

    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct TradingDecision {
    pub action: String,
    pub confidence: f64,
    pub reason: String,
}

pub fn parse_ai_response(response: &str) -> TradingDecision {
    if let Some(decision) = try_parse_json_decision(response) {
        return decision;
    }

    parse_unstructured_decision(response)
}

fn strip_markdown_fence(text: &str) -> &str {
    let trimmed = text.trim();
    if trimmed.starts_with("```") {
        trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```JSON")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
    } else {
        trimmed
    }
}

fn try_parse_json_decision(response: &str) -> Option<TradingDecision> {
    let candidate = strip_markdown_fence(response);

    // Direct JSON parse
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(candidate)
        && let Some(decision) = extract_decision_from_json_val(&val)
    {
        return Some(decision);
    }

    // Embedded JSON block { ... } inside text
    if let Some(start) = candidate.find('{')
        && let Some(end) = candidate.rfind('}')
        && end > start
    {
        let slice = &candidate[start..=end];
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(slice)
            && let Some(decision) = extract_decision_from_json_val(&val)
        {
            return Some(decision);
        }
    }

    None
}

fn extract_decision_from_json_val(val: &serde_json::Value) -> Option<TradingDecision> {
    let action_raw = val
        .get("action")
        .or_else(|| val.get("decision"))
        .or_else(|| val.get("signal"))
        .and_then(serde_json::Value::as_str)?;

    let action = normalize_action(action_raw);

    let confidence = val
        .get("confidence")
        .or_else(|| val.get("confidence_score"))
        .and_then(|v| {
            if let Some(f) = v.as_f64() {
                Some(normalize_confidence(f))
            } else if let Some(s) = v.as_str() {
                s.trim_end_matches('%')
                    .trim()
                    .parse::<f64>()
                    .ok()
                    .map(normalize_confidence)
            } else {
                None
            }
        })
        .unwrap_or(0.7);

    let reason = val
        .get("reason")
        .or_else(|| val.get("reasoning"))
        .or_else(|| val.get("explanation"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_string();

    Some(TradingDecision {
        action,
        confidence,
        reason,
    })
}

fn normalize_action(raw: &str) -> String {
    match raw.trim().to_ascii_uppercase().as_str() {
        "LONG" | "BUY" | "OPEN_LONG" => "LONG".to_string(),
        "SHORT" | "SELL" | "OPEN_SHORT" => "SHORT".to_string(),
        _ => "NO ACTION".to_string(),
    }
}

fn normalize_confidence(value: f64) -> f64 {
    if (0.0..=1.0).contains(&value) {
        value
    } else if (1.0..=100.0).contains(&value) {
        value / 100.0
    } else {
        0.7
    }
}

fn parse_unstructured_decision(response: &str) -> TradingDecision {
    let lower = response.to_lowercase();
    let action = if (lower.contains("long") || lower.contains("buy"))
        && !lower.contains("no action")
        && !lower.contains("hold")
    {
        "LONG".to_string()
    } else if (lower.contains("short") || lower.contains("sell"))
        && !lower.contains("no action")
        && !lower.contains("hold")
    {
        "SHORT".to_string()
    } else {
        "NO ACTION".to_string()
    };

    let confidence = extract_confidence_from_text(&lower).unwrap_or(0.7);

    TradingDecision {
        action,
        confidence,
        reason: response.trim().to_string(),
    }
}

fn extract_confidence_from_text(text: &str) -> Option<f64> {
    if let Some(pos) = text.find("confidence") {
        let snippet = &text[pos..];
        for token in snippet.split_whitespace().take(6) {
            let clean = token.trim_matches(|c: char| !c.is_ascii_digit() && c != '.');
            if let Ok(val) = clean.parse::<f64>() {
                return Some(normalize_confidence(val));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_json_decision() {
        let text = r#"{"action": "LONG", "confidence": 0.85, "reason": "Strong breakout on high volume"}"#;
        let d = parse_ai_response(text);
        assert_eq!(d.action, "LONG");
        assert!((d.confidence - 0.85).abs() < f64::EPSILON);
        assert_eq!(d.reason, "Strong breakout on high volume");
    }

    #[test]
    fn parses_markdown_fenced_json_with_percent_confidence() {
        let text = "```json\n{\n  \"action\": \"SHORT\",\n  \"confidence\": \"90%\",\n  \"reasoning\": \"RSI overbought divergence\"\n}\n```";
        let d = parse_ai_response(text);
        assert_eq!(d.action, "SHORT");
        assert!((d.confidence - 0.90).abs() < f64::EPSILON);
        assert_eq!(d.reason, "RSI overbought divergence");
    }

    #[test]
    fn parses_embedded_json_surrounded_by_text_and_numbers() {
        let text = "Based on market conditions at BTC price $65432 and GPT-4 analysis:\n```json\n{\"action\": \"BUY\", \"confidence\": 0.88, \"reason\": \"EMA crossover\"}\n```\nTrade safely in 2026.";
        let d = parse_ai_response(text);
        assert_eq!(d.action, "LONG");
        assert!((d.confidence - 0.88).abs() < f64::EPSILON);
        assert_eq!(d.reason, "EMA crossover");
    }

    #[test]
    fn parses_unstructured_text_fallback() {
        let text = "I recommend to open LONG position with confidence 0.75 due to bullish trend.";
        let d = parse_ai_response(text);
        assert_eq!(d.action, "LONG");
        assert!((d.confidence - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn parses_hold_decision_correctly() {
        let text = r#"{"action": "HOLD", "confidence": 0.6, "reason": "Consolidation phase"}"#;
        let d = parse_ai_response(text);
        assert_eq!(d.action, "NO ACTION");
    }

    #[test]
    fn momentum_and_fallback_decisions() {
        let m_bullish = MarketState {
            price: 105.0,
            prev_price: 100.0,
            volatility: 0.01,
        };
        assert!(momentum(&m_bullish) > 0.0);
        let fallback = generate_fallback_decision(
            "BTCUSDT",
            &m_bullish,
            "low",
            "fallback",
            "test_corr",
            "5m",
            "prompt text".to_string(),
            None,
        );
        assert_eq!(fallback.action, "LONG");
        assert_eq!(fallback.timeframe, "5m");
    }
}
