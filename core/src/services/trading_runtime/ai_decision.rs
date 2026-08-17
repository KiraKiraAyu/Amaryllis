use super::data_context::load_trader_data_context;
use super::prompt_assemble::{build_system_prompt, build_trading_prompt};
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
        return DecisionSignal {
            symbol: symbol.to_string(),
            action: "NO ACTION".to_string(),
            confidence: 0.95,
            reason: "risk control active: drawdown/margin threshold reached".to_string(),
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

    let momentum = momentum(&m);

    let data_context = match load_trader_data_context(state, cfg, symbol).await {
        Ok(context) => context,
        Err(err) => {
            warn!(
                "[AI_PROMPT] skipped - strategy data unavailable trader={} symbol={} err={}",
                cfg.trader_id, symbol, err
            );
            return DecisionSignal {
                symbol: symbol.to_string(),
                action: "NO ACTION".to_string(),
                confidence: 0.5,
                reason: format!("strategy market data unavailable: {err}"),
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

    let prompt = build_trading_prompt(
        symbol,
        &m,
        metrics,
        open_positions,
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
