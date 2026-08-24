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
            timeframe: "3m",
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
                timeframe: "5m",
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
            reason: String::new(),
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
            reason: String::new(),
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

    // Try JSON extraction first, fall back to keyword matching
    let action = extract_json_action(response).unwrap_or_else(|| {
        if lower.contains("long") && !lower.contains("no action") {
            "LONG".to_string()
        } else if lower.contains("short") && !lower.contains("no action") {
            "SHORT".to_string()
        } else {
            "NO ACTION".to_string()
        }
    });

    let reason = extract_json_reason(response).unwrap_or_else(|| response.to_string());
    let confidence = extract_confidence(&lower).unwrap_or(0.7);

    TradingDecision {
        action,
        confidence,
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
