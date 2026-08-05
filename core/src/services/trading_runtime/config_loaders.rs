use super::service::*;

pub async fn load_trader_runtime_config(
    state: &SharedState,
    trader_id: &str,
) -> Result<Option<TraderRuntimeConfig>, AppError> {
    let Some(row) = state.trading_repo.get_trader(trader_id).await? else {
        return Ok(None);
    };

    let resolved_model = match state
        .llm_service
        .resolve_for_user(Some(&row.ai_model_id))
        .await
    {
        Ok(model) => model,
        Err(AppError::BadRequest(_)) => return Ok(None),
        Err(err) => return Err(err),
    };

    // Load strategy configuration to extract symbols and leverage settings
    let mut symbols_config = Vec::new();
    let mut strategy_config: serde_json::Value = serde_json::json!({});
    if !row.strategy_id.trim().is_empty() {
        use crate::entity::strategies;
        use sea_orm::EntityTrait;

        if let Ok(Some(strategy)) = strategies::Entity::find_by_id(row.strategy_id.clone())
            .one(state.trading_repo.db())
            .await
        {
            if let Ok(cfg_val) = serde_json::from_str::<serde_json::Value>(&strategy.config) {
                strategy_config = cfg_val.clone();
                // Parse per-symbol configuration
                if let Some(symbols_arr) = cfg_val.get("symbols").and_then(|v| v.as_array()) {
                    for item in symbols_arr {
                        if let Some(symbol) = item.get("symbol").and_then(|v| v.as_str()) {
                            let leverage = item.get("leverage").and_then(|v| v.as_i64()).unwrap_or(5);
                            let min_cost = item.get("min_cost").and_then(|v| v.as_f64());
                            let max_cost = item.get("max_cost").and_then(|v| v.as_f64());
                            let fixed_cost = item.get("fixed_cost").and_then(|v| v.as_f64());
                            symbols_config.push(SymbolConfig {
                                symbol: symbol.to_uppercase(),
                                leverage,
                                min_cost,
                                max_cost,
                                fixed_cost,
                            });
                        }
                    }
                }
            }
        }
    }

    // Dynamic trading_symbols comma-separated string derived from symbols_config
    let trading_symbols = symbols_config
        .iter()
        .map(|s| s.symbol.clone())
        .collect::<Vec<_>>()
        .join(",");

    Ok(Some(TraderRuntimeConfig {
        trader_id: row.id,
        name: row.name,
        ai_model_id: resolved_model.id,
        ai_model_name: resolved_model.model_id,
        ai_provider_type: resolved_model.provider_type,
        ai_api_key: resolved_model.api_key,
        ai_base_url: resolved_model.base_url,
        exchange_id: row.exchange_id,
        scan_interval_minutes: row.scan_interval_minutes,
        initial_balance: row.initial_balance,
        is_cross_margin: row.is_cross_margin != 0,
        trading_symbols,
        custom_prompt: row.custom_prompt,
        override_base_prompt: row.override_base_prompt != 0,
        system_prompt_template: row.system_prompt_template,
        symbols_config,
        strategy_config,
    }))
}

pub async fn set_trader_running(
    state: &SharedState,
    trader_id: &str,
    running: bool,
) -> Result<(), AppError> {
    state
        .trading_repo
        .set_trader_running(trader_id, running, now_i64())
        .await?;
    Ok(())
}

pub async fn load_runtime_execution_context(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
) -> Result<
    (
        RuntimeExecutionContext,
        Option<Box<dyn LiveExchangeAdapter>>,
    ),
    AppError,
> {
    let Some(row) = state
        .exchange_repo
        .find_runtime_config(&cfg.exchange_id)
        .await?
    else {
        return Ok((
            RuntimeExecutionContext {
                mode: RuntimeExecutionMode::Simulated,
            },
            None,
        ));
    };

    if !row.enabled {
        return Ok((
            RuntimeExecutionContext {
                mode: RuntimeExecutionMode::Simulated,
            },
            None,
        ));
    }

    let api_key = row.api_key;
    let secret_key = row.secret_key;
    let passphrase_raw = row.passphrase;
    let wallet_addr = row.hyperliquid_wallet_addr;
    let testnet = row.testnet;

    if exchange_credentials_missing(
        &row.exchange_type,
        &api_key,
        &secret_key,
        &passphrase_raw,
        &wallet_addr,
    ) {
        return Err(AppError::InvalidExchangeConfig(format!(
            "enabled exchange credentials are incomplete for trader={}",
            cfg.trader_id
        )));
    }

    let credentials = ExchangeCredentials {
        api_key,
        secret_key,
        passphrase: if passphrase_raw.trim().is_empty() {
            None
        } else {
            Some(passphrase_raw)
        },
        wallet_addr: if wallet_addr.trim().is_empty() {
            None
        } else {
            Some(wallet_addr)
        },
        testnet,
    };

    let adapter = create_exchange_adapter(&row.exchange_type, credentials)?;
    if let Err(err) = adapter.ping().await {
        return Err(AppError::BadGateway(format!(
            "enabled exchange ping failed for trader={}: {}",
            cfg.trader_id, err
        )));
    }

    let symbols = parse_symbols(&cfg.trading_symbols);
    preflight_live_symbols(adapter.as_ref(), cfg, &symbols).await?;

    Ok((
        RuntimeExecutionContext {
            mode: RuntimeExecutionMode::LiveExchange,
        },
        Some(adapter),
    ))
}

pub fn exchange_credentials_missing(
    exchange_type: &str,
    api_key: &str,
    secret_key: &str,
    passphrase: &str,
    wallet_addr: &str,
) -> bool {
    match exchange_type.trim().to_ascii_lowercase().as_str() {
        "hyperliquid" | "aster" => wallet_addr.trim().is_empty() || secret_key.trim().is_empty(),
        "okx" | "bitget" => {
            api_key.trim().is_empty()
                || secret_key.trim().is_empty()
                || passphrase.trim().is_empty()
        }
        _ => api_key.trim().is_empty() || secret_key.trim().is_empty(),
    }
}

pub fn parse_symbols(raw: &str) -> Vec<String> {
    let mut out = raw
        .split(|c: char| c == ',' || c.is_whitespace() || c == ';' || c == '|')
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    if out.is_empty() {
        out = vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()];
    }

    out.sort();
    out.dedup();
    out
}

pub fn leverage_for_symbol(cfg: &TraderRuntimeConfig, symbol: &str) -> i64 {
    cfg.symbols_config
        .iter()
        .find(|s| s.symbol.to_uppercase() == symbol.to_uppercase())
        .map(|sc| sc.leverage.clamp(1, 200))
        .unwrap_or(5)
}

pub async fn preflight_live_symbols(
    adapter: &dyn LiveExchangeAdapter,
    cfg: &TraderRuntimeConfig,
    symbols: &[String],
) -> Result<(), AppError> {
    let margin_mode = margin_mode_for_config(cfg);

    for symbol in symbols {
        let symbol = symbol.trim().to_uppercase();
        if symbol.is_empty() {
            continue;
        }

        let constraints = adapter.get_symbol_constraints(&symbol).await?;
        validate_symbol_constraints(adapter.exchange_type(), &symbol, &constraints)?;
        adapter
            .ensure_symbol_settings(&symbol, leverage_for_symbol(cfg, &symbol), margin_mode)
            .await?;
    }

    Ok(())
}

pub fn margin_mode_for_config(cfg: &TraderRuntimeConfig) -> ExchangeMarginMode {
    if cfg.is_cross_margin {
        ExchangeMarginMode::Cross
    } else {
        ExchangeMarginMode::Isolated
    }
}

pub fn validate_symbol_constraints(
    exchange_type: &str,
    requested_symbol: &str,
    constraints: &ExchangeSymbolConstraints,
) -> Result<(), AppError> {
    let valid = !constraints.symbol.trim().is_empty()
        && !constraints.base_asset.trim().is_empty()
        && !constraints.quote_asset.trim().is_empty()
        && constraints.min_qty.is_finite()
        && constraints.min_qty > 0.0
        && constraints.step_size.is_finite()
        && constraints.step_size > 0.0
        && constraints.tick_size.is_finite()
        && constraints.tick_size > 0.0
        && constraints.min_notional.is_finite()
        && constraints.min_notional > 0.0
        && constraints.max_qty.is_finite()
        && constraints.max_qty >= 0.0;

    if valid {
        return Ok(());
    }

    Err(AppError::InvalidExchangeConfig(format!(
        "invalid symbol constraints exchange={} requested_symbol={} symbol={} min_qty={} max_qty={} step_size={} min_notional={} tick_size={}",
        exchange_type,
        requested_symbol,
        constraints.symbol,
        constraints.min_qty,
        constraints.max_qty,
        constraints.step_size,
        constraints.min_notional,
        constraints.tick_size,
    )))
}

pub fn liquidation_price(side: &str, entry: f64, leverage: i64) -> f64 {
    let lv = leverage.max(1) as f64;
    if side == "LONG" {
        (entry * (1.0 - (0.9 / lv))).max(0.0001)
    } else {
        (entry * (1.0 + (0.9 / lv))).max(0.0001)
    }
}

pub fn default_price_for_symbol(symbol: &str) -> f64 {
    match symbol {
        s if s.contains("BTC") => 65_000.0,
        s if s.contains("ETH") => 3_200.0,
        s if s.contains("SOL") => 160.0,
        s if s.contains("BNB") => 560.0,
        s if s.contains("XRP") => 0.65,
        s if s.contains("DOGE") => 0.16,
        _ => 50.0,
    }
}

pub fn deterministic_noise(seed: u64) -> f64 {
    let mut x = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51afd7ed558ccd);
    x ^= x >> 33;
    let frac = (x as f64 / u64::MAX as f64).clamp(0.0, 1.0);
    (frac * 2.0) - 1.0
}

pub fn hash_str(s: &str) -> u64 {
    let mut h: u64 = 1469598103934665603;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

pub fn parse_f64(v: &str) -> f64 {
    v.parse::<f64>().unwrap_or(0.0)
}

pub fn now_i64() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn now_u64() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exchange_credentials_missing_matches_exchange_requirements() {
        assert!(exchange_credentials_missing("okx", "key", "secret", "", ""));
        assert!(!exchange_credentials_missing(
            "bitget",
            "key",
            "secret",
            "passphrase",
            ""
        ));
        assert!(exchange_credentials_missing(
            "hyperliquid",
            "",
            "private-key",
            "",
            ""
        ));
        assert!(!exchange_credentials_missing(
            "hyperliquid",
            "",
            "private-key",
            "",
            "0xabc"
        ));
        assert!(exchange_credentials_missing(
            "binance", "", "secret", "", ""
        ));
        assert!(exchange_credentials_missing(
            "aster", "", "private-key", "", ""
        ));
        assert!(!exchange_credentials_missing(
            "aster", "", "private-key", "", "0xabc"
        ));
    }
}
