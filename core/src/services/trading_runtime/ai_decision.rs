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
            action: "HOLD",
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
                action: "HOLD",
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
    let custom_prompt = if cfg.override_base_prompt {
        Some(cfg.custom_prompt.as_str())
    } else {
        None
    };
    let system_prompt_owned = custom_prompt.map(|s| s.to_string());

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

    match state
        .llm_service
        .chat_with_config(
            cfg.ai_provider_type.clone(),
            cfg.ai_api_key.clone(),
            cfg.ai_model_name.clone(),
            cfg.ai_base_url.clone(),
            vec![user_message],
            custom_prompt,
        )
        .await
    {
        Ok(response) => {
            let decision = parse_ai_response(&response);
            let action_str = match decision.action.to_uppercase().as_str() {
                "BUY" => "BUY",
                "SELL" => "SELL",
                _ => "HOLD",
            };
            DecisionSignal {
                symbol: symbol.to_string(),
                action: action_str,
                confidence: decision.confidence,
                reason: decision.reason,
                timeframe: "3m",
                price: m.price,
                momentum,
                risk_level: risk_level.to_string(),
                trigger_source: trigger_source.to_string(),
                action_taken: format!("ai-{}-{}", cfg.ai_model_id, decision.action.to_lowercase()),
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

    format!(
        r#"Analyze the following trading opportunity for {}:

Current Price: {:.2}
Previous Price: {:.2}
Price Change: {:.4}%
Volatility: {:.4}%

Account Status:
- Total Balance: ${:.2}
- Available Balance: ${:.2}
- Used Margin: ${:.2}
- Unrealized PnL: ${:.2}
- Realized PnL: ${:.2}
- Margin Usage: {:.2}%

Position Limits:
- Max leverage: {}x
- Risk per position: 6% of account

Respond with a JSON object in this exact format:
{{
    "action": "BUY" or "SELL" or "HOLD",
    "confidence": 0.0-1.0,
    "reason": "Your analysis in 1-2 sentences"
}}"#,
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
        cfg.btc_eth_leverage.max(cfg.altcoin_leverage)
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
            action: "HOLD",
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
            action: "BUY",
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
            action: "SELL",
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
            action: "HOLD",
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
    ts: i64,
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
        "correlation_id": d.correlation_id
    })
    .to_string();

    state
        .trading_repo
        .insert_decision(InsertTraderDecisionRecord {
            id: Uuid::now_v7().to_string(),
            trader_id: cfg.trader_id.clone(),
            symbol: d.symbol.clone(),
            timeframe: d.timeframe.to_string(),
            decision: d.action.to_string(),
            confidence: d.confidence,
            reason: d.reason.clone(),
            payload_json: payload,
            created_at: ts,
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
    let action = if lower.contains("buy") || lower.contains("long") {
        "BUY"
    } else if lower.contains("sell") || lower.contains("short") {
        "SELL"
    } else {
        "HOLD"
    };

    // Try to parse the response as JSON and extract the reason field.
    // LLMs often wrap their output in a JSON object like:
    //   {"action":"HOLD","confidence":0.5,"reason":"..."}
    // If JSON parsing succeeds, use the extracted reason; otherwise fall back
    // to the raw response text (without truncation).
    let reason = extract_json_reason(response).unwrap_or_else(|| response.to_string());

    TradingDecision {
        action: action.to_string(),
        confidence: extract_confidence(&lower).unwrap_or(0.7),
        reason,
    }
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
