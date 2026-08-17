use std::{collections::HashMap, fmt::Write};

use crate::{
    clients::market_data::{
        MarketKline, fetch_aster_klines, fetch_binance_klines, fetch_bitget_klines,
        fetch_hyperliquid_klines, fetch_okx_klines, normalize_crypto_symbol,
    },
    error::AppError,
    services::data_template::{
        DEFAULT_KLINE_COUNT, MAX_KLINE_COUNT, StrategyDataItem, validate_strategy_data_template,
    },
};

use super::models::{SharedState, TraderRuntimeConfig};

#[derive(Debug, Clone)]
pub struct TraderDataContext {
    pub rendered: String,
}

pub async fn load_trader_data_context(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    symbol: &str,
) -> Result<TraderDataContext, AppError> {
    let template =
        validate_strategy_data_template(&cfg.strategy_config).map_err(AppError::InvalidConfig)?;
    let exchange = state
        .exchange_repo
        .find_runtime_config(&cfg.exchange_id)
        .await?
        .ok_or_else(|| {
            AppError::InvalidExchangeConfig("Trader exchange account not found".into())
        })?;

    let mut fetch_cache: HashMap<(String, usize), Vec<MarketKline>> = HashMap::new();
    let mut rendered = format!(
        "## Strategy Market Data\n- Exchange: {}\n- Symbol: {}\n",
        exchange.exchange_type, symbol
    );
    let mut indicator_lines: Vec<String> = Vec::new();

    for item in template
        .items
        .iter()
        .filter(|item| item.item_type == "raw_kline")
    {
        for timeframe in &item.timeframes {
            let key = (timeframe.clone(), item.count);
            let mut candles = if let Some(cached) = fetch_cache.get(&key) {
                cached.clone()
            } else {
                let candles =
                    fetch_klines(&exchange.exchange_type, symbol, timeframe, item.count).await?;
                fetch_cache.insert(key, candles.clone());
                candles
            };

            if item.closed_only {
                let now_ms = chrono::Utc::now().timestamp_millis();
                candles.retain(|candle| candle.close_time <= now_ms);
            }
            if candles.is_empty() {
                return Err(AppError::BadGateway(format!(
                    "No closed {} Kline data available for {}",
                    timeframe, symbol
                )));
            }

            render_raw_klines(&mut rendered, timeframe, &candles);
        }
    }

    for item in template
        .items
        .iter()
        .filter(|item| item.item_type == "indicator")
    {
        for timeframe in &item.timeframes {
            let count = indicator_kline_count(item);
            let key = (timeframe.clone(), count);
            let mut candles = if let Some(cached) = fetch_cache.get(&key) {
                cached.clone()
            } else {
                let candles =
                    fetch_klines(&exchange.exchange_type, symbol, timeframe, count).await?;
                fetch_cache.insert(key, candles.clone());
                candles
            };
            let now_ms = chrono::Utc::now().timestamp_millis();
            candles.retain(|candle| candle.close_time <= now_ms);
            if candles.is_empty() {
                return Err(AppError::BadGateway(format!(
                    "No closed {} Kline data available for {}",
                    timeframe, symbol
                )));
            }

            let (label, value) = calculate_indicator(item, &candles)?;
            indicator_lines.push(format!("- {} @ {} = {}", label, timeframe, value));
        }
    }

    if !indicator_lines.is_empty() {
        let _ = writeln!(rendered, "### Indicators");
        for line in indicator_lines {
            let _ = writeln!(rendered, "{line}");
        }
    }

    Ok(TraderDataContext { rendered })
}

fn indicator_kline_count(item: &StrategyDataItem) -> usize {
    let period = item
        .params
        .get("period")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_else(|| default_period(&item.indicator)) as usize;
    let minimum = match item.indicator.as_str() {
        "macd" => 34,
        "rsi" | "atr" => period.saturating_add(1),
        _ => period,
    };
    DEFAULT_KLINE_COUNT.max(minimum).min(MAX_KLINE_COUNT)
}

async fn fetch_klines(
    exchange_type: &str,
    symbol: &str,
    timeframe: &str,
    count: usize,
) -> Result<Vec<MarketKline>, AppError> {
    match exchange_type.trim().to_ascii_lowercase().as_str() {
        "binance" => fetch_binance_klines(&normalize_crypto_symbol(symbol), timeframe, count).await,
        "aster" => fetch_aster_klines(&normalize_crypto_symbol(symbol), timeframe, count).await,
        "okx" => fetch_okx_klines(symbol, timeframe, count).await,
        "bitget" => fetch_bitget_klines(symbol, timeframe, count).await,
        "hyperliquid" => fetch_hyperliquid_klines(symbol, timeframe, count).await,
        _ => Err(AppError::InvalidExchangeConfig(
            "Trader exchange does not support Kline data".into(),
        )),
    }
}

/// Format a kline open time (epoch millis) as HH:MM UTC — the current date
/// is stated once in the trading prompt header, so candle rows stay compact.
fn format_kline_time(open_time_ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(open_time_ms)
        .map(|dt| dt.format("%H:%M").to_string())
        .unwrap_or_default()
}

fn render_raw_klines(
    rendered: &mut String,
    timeframe: &str,
    candles: &[MarketKline],
) {
    let _ = writeln!(
        rendered,
        "### {} ({}, {} closed candles)\ntime,open,high,low,close,volume",
        "Kline",
        timeframe,
        candles.len()
    );
    for candle in candles {
        let _ = writeln!(
            rendered,
            "{},{:.8},{:.8},{:.8},{:.8},{:.8}",
            format_kline_time(candle.open_time),
            candle.open,
            candle.high,
            candle.low,
            candle.close,
            candle.volume
        );
    }
}

fn calculate_indicator(
    item: &StrategyDataItem,
    candles: &[MarketKline],
) -> Result<(String, String), AppError> {
    let closes: Vec<f64> = candles.iter().map(|candle| candle.close).collect();
    let period = item
        .params
        .get("period")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_else(|| default_period(&item.indicator)) as usize;
    let name = indicator_name(&item.indicator);

    match item.indicator.as_str() {
        "ema" => Ok((
            format!("{name}({period})"),
            format!("{:.8}", ema(&closes, period)?),
        )),
        "rsi" => Ok((
            format!("{name}({period})"),
            format!("{:.4}", rsi(&closes, period)?),
        )),
        "atr" => Ok((
            format!("{name}({period})"),
            format!("{:.8}", atr(candles, period)?),
        )),
        "bollinger" => {
            let (middle, upper, lower) = bollinger(&closes, period)?;
            Ok((
                format!("{name}({period})"),
                format!("middle={middle:.8}, upper={upper:.8}, lower={lower:.8}"),
            ))
        }
        "macd" => {
            let (macd, signal, histogram) = macd(&closes)?;
            Ok((
                format!("{name}(12,26,9)"),
                format!("line={macd:.8}, signal={signal:.8}, histogram={histogram:.8}"),
            ))
        }
        _ => Err(AppError::InvalidConfig(format!(
            "Unsupported indicator '{}'",
            item.indicator
        ))),
    }
}

fn indicator_name(indicator: &str) -> &str {
    match indicator {
        "ema" => "EMA",
        "macd" => "MACD",
        "rsi" => "RSI",
        "atr" => "ATR",
        "bollinger" => "Bollinger",
        _ => "Indicator",
    }
}

fn default_period(indicator: &str) -> u64 {
    match indicator {
        "bollinger" => 20,
        _ => 14,
    }
}

fn ema(values: &[f64], period: usize) -> Result<f64, AppError> {
    ema_series(values, period)?
        .last()
        .copied()
        .ok_or_else(|| AppError::InvalidConfig("Insufficient Kline data for EMA".into()))
}

fn ema_series(values: &[f64], period: usize) -> Result<Vec<f64>, AppError> {
    if period == 0 || values.len() < period {
        return Err(AppError::InvalidConfig(
            "Insufficient Kline data for indicator period".into(),
        ));
    }
    let mut current = values[..period].iter().sum::<f64>() / period as f64;
    let multiplier = 2.0 / (period as f64 + 1.0);
    let mut out = vec![current];
    for value in &values[period..] {
        current = (value * multiplier) + (current * (1.0 - multiplier));
        out.push(current);
    }
    Ok(out)
}

fn rsi(values: &[f64], period: usize) -> Result<f64, AppError> {
    if period == 0 || values.len() <= period {
        return Err(AppError::InvalidConfig(
            "Insufficient Kline data for RSI".into(),
        ));
    }
    let changes: Vec<f64> = values.windows(2).map(|pair| pair[1] - pair[0]).collect();
    let (mut avg_gain, mut avg_loss) = changes[..period].iter().fold((0.0, 0.0), |acc, change| {
        if *change >= 0.0 {
            (acc.0 + change, acc.1)
        } else {
            (acc.0, acc.1 + change.abs())
        }
    });
    avg_gain /= period as f64;
    avg_loss /= period as f64;
    for change in &changes[period..] {
        let gain = change.max(0.0);
        let loss = (-change).max(0.0);
        avg_gain = ((avg_gain * (period as f64 - 1.0)) + gain) / period as f64;
        avg_loss = ((avg_loss * (period as f64 - 1.0)) + loss) / period as f64;
    }
    if avg_loss <= f64::EPSILON {
        return Ok(100.0);
    }
    Ok(100.0 - (100.0 / (1.0 + avg_gain / avg_loss)))
}

fn atr(candles: &[MarketKline], period: usize) -> Result<f64, AppError> {
    if period == 0 || candles.len() <= period {
        return Err(AppError::InvalidConfig(
            "Insufficient Kline data for ATR".into(),
        ));
    }
    let ranges: Vec<f64> = candles
        .windows(2)
        .map(|pair| {
            let current = &pair[1];
            let previous_close = pair[0].close;
            (current.high - current.low)
                .max((current.high - previous_close).abs())
                .max((current.low - previous_close).abs())
        })
        .collect();
    let mut current = ranges[..period].iter().sum::<f64>() / period as f64;
    for range in &ranges[period..] {
        current = ((current * (period as f64 - 1.0)) + range) / period as f64;
    }
    Ok(current)
}

fn bollinger(values: &[f64], period: usize) -> Result<(f64, f64, f64), AppError> {
    if period == 0 || values.len() < period {
        return Err(AppError::InvalidConfig(
            "Insufficient Kline data for Bollinger Bands".into(),
        ));
    }
    let slice = &values[values.len() - period..];
    let middle = slice.iter().sum::<f64>() / period as f64;
    let variance = slice
        .iter()
        .map(|value| (value - middle).powi(2))
        .sum::<f64>()
        / period as f64;
    let deviation = variance.sqrt();
    Ok((middle, middle + 2.0 * deviation, middle - 2.0 * deviation))
}

fn macd(values: &[f64]) -> Result<(f64, f64, f64), AppError> {
    let fast = ema_series(values, 12)?;
    let slow = ema_series(values, 26)?;
    let aligned_fast = &fast[fast.len().saturating_sub(slow.len())..];
    let line: Vec<f64> = aligned_fast
        .iter()
        .zip(&slow)
        .map(|(fast_value, slow_value)| fast_value - slow_value)
        .collect();
    let signal = ema_series(&line, 9)?;
    let line_value = *line
        .last()
        .ok_or_else(|| AppError::InvalidConfig("Insufficient Kline data for MACD".into()))?;
    let signal_value = *signal
        .last()
        .ok_or_else(|| AppError::InvalidConfig("Insufficient Kline data for MACD".into()))?;
    Ok((line_value, signal_value, line_value - signal_value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn candle(close: f64) -> MarketKline {
        MarketKline {
            open_time: 1,
            open: close - 0.5,
            high: close + 1.0,
            low: close - 1.0,
            close,
            volume: 10.0,
            quote_volume: 100.0,
            close_time: 2,
        }
    }

    #[test]
    fn indicator_calculations_use_recent_ohlcv() {
        let values: Vec<f64> = (1..=30).map(f64::from).collect();
        let ema_value = ema(&values, 5).expect("ema");
        assert!(ema_value > 26.0 && ema_value < 30.0);
        assert_eq!(rsi(&values, 14).expect("rsi"), 100.0);

        let bands = bollinger(&values, 5).expect("bollinger");
        assert_eq!(bands.0, 28.0);
        assert!(bands.1 > bands.0 && bands.2 < bands.0);
    }

    #[test]
    fn indicator_calculations_reject_insufficient_lookback() {
        let values = vec![1.0, 2.0, 3.0];
        assert!(ema(&values, 5).is_err());
        assert!(rsi(&values, 3).is_err());
        assert!(macd(&values).is_err());
    }

    #[test]
    fn raw_kline_render_contains_headers_and_readable_time() {
        let mut rendered = String::new();
        let candles = vec![
            MarketKline {
                open_time: 1,
                open: 9.5,
                high: 11.0,
                low: 9.0,
                close: 10.0,
                volume: 10.0,
                quote_volume: 100.0,
                close_time: 2,
            },
            MarketKline {
                open_time: 1_000 * 60 * 30,
                open: 10.5,
                high: 12.0,
                low: 10.0,
                close: 11.0,
                volume: 20.0,
                quote_volume: 200.0,
                close_time: 2_000 * 60 * 30,
            },
        ];
        render_raw_klines(&mut rendered, "5m", &candles);
        assert!(rendered.contains("Kline (5m, 2 closed candles)"));
        assert!(rendered.contains("00:00,9.50000000,11.00000000,9.00000000,10.00000000"));
        assert!(rendered.contains("00:30,10.50000000,12.00000000,10.00000000,11.00000000"));
    }

    #[test]
    fn kline_time_excludes_date() {
        // 2026-08-17 14:35:00 UTC in epoch millis
        let ts = 1_786_977_300_000;
        let formatted = format_kline_time(ts);
        assert_eq!(formatted, "14:35");
    }

    #[test]
    fn indicator_result_pairs_label_with_value() {
        let values: Vec<f64> = (1..=30).map(f64::from).collect();
        let candles: Vec<MarketKline> = values.iter().map(|v| candle(*v)).collect();
        let item = StrategyDataItem {
            id: "rsi".to_string(),
            item_type: "indicator".to_string(),
            timeframes: vec!["5m".to_string()],
            count: 0,
            closed_only: true,
            indicator: "rsi".to_string(),
            params: json!({ "period": 14 }),
            output_mode: "latest".to_string(),
        };

        let (label, value) = calculate_indicator(&item, &candles).expect("rsi");
        assert_eq!(label, "RSI(14)");
        assert_eq!(value, "100.0000");
    }

    #[test]
    fn indicators_fetch_their_own_sufficient_kline_history() {
        let item = StrategyDataItem {
            id: "rsi".to_string(),
            item_type: "indicator".to_string(),
            timeframes: vec!["5m".to_string()],
            count: 0,
            closed_only: true,
            indicator: "rsi".to_string(),
            params: json!({ "period": 120 }),
            output_mode: "latest".to_string(),
        };

        assert_eq!(indicator_kline_count(&item), 121);
    }
}
