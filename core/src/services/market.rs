use crate::{
    clients::market_data::{
        fetch_aster_klines, fetch_aster_symbols, fetch_binance_klines, fetch_binance_symbols,
        fetch_bitget_klines, fetch_bitget_symbols, fetch_hyperliquid_klines,
        fetch_hyperliquid_symbols, fetch_okx_klines, fetch_okx_symbols, normalize_crypto_symbol,
    },
    contracts::public::{ExchangeSymbolPayload, ExchangeSymbolsPayload, KlinePayload},
    error::{AppError, Result},
};

pub async fn symbols(exchange: Option<String>) -> Result<ExchangeSymbolsPayload> {
    let exchange = exchange
        .unwrap_or_else(|| "hyperliquid".to_string())
        .to_ascii_lowercase();

    if exchange == "hyperliquid" {
        let symbols = fetch_hyperliquid_symbols().await.unwrap_or_else(|_| {
            vec![
                crate::clients::market_data::MarketSymbol {
                    symbol: "BTCUSDT".to_string(),
                    name: "BTCUSDT".to_string(),
                    category: "crypto".to_string(),
                },
                crate::clients::market_data::MarketSymbol {
                    symbol: "ETHUSDT".to_string(),
                    name: "ETHUSDT".to_string(),
                    category: "crypto".to_string(),
                },
                crate::clients::market_data::MarketSymbol {
                    symbol: "SOLUSDT".to_string(),
                    name: "SOLUSDT".to_string(),
                    category: "crypto".to_string(),
                },
            ]
        });
        let symbols = symbols
            .into_iter()
            .map(ExchangeSymbolPayload::from)
            .collect::<Vec<_>>();
        return Ok(ExchangeSymbolsPayload {
            exchange,
            count: symbols.len(),
            symbols,
        });
    }

    if matches!(exchange.as_str(), "binance" | "okx" | "bitget" | "aster") {
        let symbols = fetch_symbols_for_exchange(&exchange)
            .await
            .unwrap_or_else(|_| {
                vec![
                    crate::clients::market_data::MarketSymbol {
                        symbol: "BTCUSDT".to_string(),
                        name: "BTCUSDT".to_string(),
                        category: "crypto".to_string(),
                    },
                    crate::clients::market_data::MarketSymbol {
                        symbol: "ETHUSDT".to_string(),
                        name: "ETHUSDT".to_string(),
                        category: "crypto".to_string(),
                    },
                    crate::clients::market_data::MarketSymbol {
                        symbol: "SOLUSDT".to_string(),
                        name: "SOLUSDT".to_string(),
                        category: "crypto".to_string(),
                    },
                ]
            });
        let symbols: Vec<ExchangeSymbolPayload> = symbols
            .into_iter()
            .map(ExchangeSymbolPayload::from)
            .collect();
        return Ok(ExchangeSymbolsPayload {
            exchange,
            count: symbols.len(),
            symbols,
        });
    }

    Err(AppError::BadRequest(
        "Unsupported exchange for symbol listing".into(),
    ))
}

pub async fn klines(
    symbol: String,
    interval: Option<String>,
    limit: Option<i64>,
    exchange: Option<String>,
) -> Result<Vec<KlinePayload>> {
    let symbol = symbol.trim().to_uppercase();
    if symbol.is_empty() {
        return Err(AppError::BadRequest("symbol parameter is required".into()));
    }

    let interval = interval.unwrap_or_else(|| "5m".to_string());
    let limit = limit.unwrap_or(1000).clamp(1, 1500) as usize;
    let exchange = exchange
        .unwrap_or_else(|| "binance".to_string())
        .to_ascii_lowercase();

    if matches!(
        exchange.as_str(),
        "binance" | "okx" | "bitget" | "aster" | "hyperliquid"
    ) {
        if let Ok(klines) = fetch_klines_for_exchange(&exchange, &symbol, &interval, limit).await {
            return Ok(klines.into_iter().map(KlinePayload::from).collect());
        }
        return Err(AppError::BadGateway(
            "Market data upstream unavailable for requested exchange".into(),
        ));
    }

    Err(AppError::BadRequest(
        "Unsupported exchange for klines endpoint".into(),
    ))
}

async fn fetch_symbols_for_exchange(
    exchange: &str,
) -> Result<Vec<crate::clients::market_data::MarketSymbol>> {
    match exchange {
        "binance" => fetch_binance_symbols().await,
        "okx" => fetch_okx_symbols().await,
        "bitget" => fetch_bitget_symbols().await,
        "aster" => fetch_aster_symbols().await,
        _ => Err(AppError::BadRequest(
            "Unsupported exchange for symbol listing".into(),
        )),
    }
}

async fn fetch_klines_for_exchange(
    exchange: &str,
    symbol: &str,
    interval: &str,
    limit: usize,
) -> Result<Vec<crate::clients::market_data::MarketKline>> {
    match exchange {
        "binance" => fetch_binance_klines(&normalize_crypto_symbol(symbol), interval, limit).await,
        "okx" => fetch_okx_klines(symbol, interval, limit).await,
        "bitget" => fetch_bitget_klines(symbol, interval, limit).await,
        "aster" => fetch_aster_klines(&normalize_crypto_symbol(symbol), interval, limit).await,
        "hyperliquid" => fetch_hyperliquid_klines(symbol, interval, limit).await,
        _ => Err(AppError::BadRequest(
            "Unsupported exchange for klines endpoint".into(),
        )),
    }
}
