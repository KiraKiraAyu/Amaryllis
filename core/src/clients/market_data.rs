use chrono::{TimeZone, Utc};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    clients::outbound_http::{http_client, send_text, OutboundRequestLog},
    error::AppError,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MarketKline {
    #[serde(rename = "openTime")]
    pub open_time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    #[serde(rename = "quoteVolume")]
    pub quote_volume: f64,
    #[serde(rename = "closeTime")]
    pub close_time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MarketSymbol {
    pub symbol: String,
    pub name: String,
    pub category: String,
}

pub(crate) fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub(crate) fn ts_to_rfc3339(ts: i64) -> String {
    Utc.timestamp_opt(ts, 0)
        .single()
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| "1970-01-01T00:00:00+00:00".to_string())
}

pub(crate) fn parse_json_value(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|_| json!({}))
}

pub(crate) fn normalize_crypto_symbol(symbol: &str) -> String {
    let upper = symbol.trim().to_uppercase();
    if upper.ends_with("USDT") {
        upper
    } else {
        format!("{}USDT", upper)
    }
}

pub(crate) async fn fetch_binance_klines(
    symbol: &str,
    interval: &str,
    limit: usize,
) -> Result<Vec<MarketKline>, AppError> {
    fetch_binance_compatible_klines(
        "https://fapi.binance.com",
        "market.binance.klines",
        "Binance",
        symbol,
        interval,
        limit,
    )
    .await
}

pub(crate) async fn fetch_aster_klines(
    symbol: &str,
    interval: &str,
    limit: usize,
) -> Result<Vec<MarketKline>, AppError> {
    fetch_binance_compatible_klines(
        "https://fapi.asterdex.com",
        "market.aster.klines",
        "Aster",
        symbol,
        interval,
        limit,
    )
    .await
}

async fn fetch_binance_compatible_klines(
    base_url: &str,
    log_name: &'static str,
    exchange_name: &'static str,
    symbol: &str,
    interval: &str,
    limit: usize,
) -> Result<Vec<MarketKline>, AppError> {
    let url = format!(
        "{}/fapi/v1/klines?symbol={}&interval={}&limit={}",
        base_url,
        symbol,
        interval,
        limit.min(1500)
    );
    let response = send_text(
        http_client().get(&url),
        OutboundRequestLog::new(log_name, Method::GET, &url),
    )
    .await?;
    if !response.status.is_success() {
        return Err(AppError::BadGateway(format!(
            "{exchange_name} klines request failed with status {}",
            response.status
        )));
    }
    let rows = serde_json::from_str::<Vec<Vec<Value>>>(&response.body)?;

    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        if r.len() < 7 {
            continue;
        }
        let open_time = r.first().and_then(Value::as_i64).unwrap_or(0);
        let open = r
            .get(1)
            .and_then(Value::as_str)
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0);
        let high = r
            .get(2)
            .and_then(Value::as_str)
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0);
        let low = r
            .get(3)
            .and_then(Value::as_str)
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0);
        let close = r
            .get(4)
            .and_then(Value::as_str)
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0);
        let volume = r
            .get(5)
            .and_then(Value::as_str)
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0);
        let close_time = r.get(6).and_then(Value::as_i64).unwrap_or(open_time);
        let quote_volume = r
            .get(7)
            .and_then(Value::as_str)
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(volume * close);

        out.push(MarketKline {
            open_time,
            open,
            high,
            low,
            close,
            volume,
            quote_volume,
            close_time,
        });
    }

    Ok(out)
}

pub(crate) async fn fetch_binance_symbols() -> Result<Vec<MarketSymbol>, AppError> {
    fetch_binance_compatible_symbols(
        "https://fapi.binance.com",
        "market.binance.symbols",
        "Binance",
    )
    .await
}

pub(crate) async fn fetch_aster_symbols() -> Result<Vec<MarketSymbol>, AppError> {
    fetch_binance_compatible_symbols("https://fapi.asterdex.com", "market.aster.symbols", "Aster")
        .await
}

async fn fetch_binance_compatible_symbols(
    base_url: &str,
    log_name: &'static str,
    exchange_name: &'static str,
) -> Result<Vec<MarketSymbol>, AppError> {
    let url = format!("{base_url}/fapi/v1/exchangeInfo");
    let response = send_text(
        http_client().get(&url),
        OutboundRequestLog::new(log_name, Method::GET, &url),
    )
    .await?;
    if !response.status.is_success() {
        return Err(AppError::BadGateway(format!(
            "{exchange_name} symbols request failed with status {}",
            response.status
        )));
    }
    let payload = serde_json::from_str::<Value>(&response.body)?;
    let symbols = payload
        .get("symbols")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut out = Vec::new();
    for s in symbols {
        let status = s.get("status").and_then(Value::as_str).unwrap_or_default();
        let symbol = s.get("symbol").and_then(Value::as_str).unwrap_or_default();
        let quote = s
            .get("quoteAsset")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if status != "TRADING" || quote != "USDT" || symbol.is_empty() {
            continue;
        }
        out.push(MarketSymbol {
            symbol: symbol.to_string(),
            name: symbol.to_string(),
            category: "crypto".to_string(),
        });
        if out.len() >= 400 {
            break;
        }
    }

    Ok(out)
}

pub(crate) async fn fetch_okx_symbols() -> Result<Vec<MarketSymbol>, AppError> {
    let url = "https://www.okx.com/api/v5/public/instruments?instType=SWAP";
    let response = send_text(
        http_client().get(url),
        OutboundRequestLog::new("market.okx.symbols", Method::GET, url),
    )
    .await?;
    if !response.status.is_success() {
        return Err(AppError::BadGateway(format!(
            "OKX symbols request failed with status {}",
            response.status
        )));
    }

    let payload = serde_json::from_str::<Value>(&response.body)?;
    ensure_okx_public_success(&payload)?;
    let rows = payload
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut out = Vec::new();
    for row in rows {
        let state = row.get("state").and_then(Value::as_str).unwrap_or_default();
        let quote = row
            .get("quoteCcy")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let inst_id = row
            .get("instId")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if state != "live" || quote != "USDT" || !inst_id.ends_with("-USDT-SWAP") {
            continue;
        }
        let symbol = okx_internal_symbol(inst_id);
        out.push(MarketSymbol {
            name: symbol.clone(),
            symbol,
            category: "crypto".to_string(),
        });
        if out.len() >= 400 {
            break;
        }
    }

    Ok(out)
}

pub(crate) async fn fetch_okx_klines(
    symbol: &str,
    interval: &str,
    limit: usize,
) -> Result<Vec<MarketKline>, AppError> {
    let inst_id = okx_inst_id(symbol);
    let okx_interval = okx_interval(interval);
    let url = format!(
        "https://www.okx.com/api/v5/market/candles?instId={}&bar={}&limit={}",
        inst_id,
        okx_interval,
        limit.min(300)
    );
    let response = send_text(
        http_client().get(&url),
        OutboundRequestLog::new("market.okx.klines", Method::GET, &url),
    )
    .await?;
    if !response.status.is_success() {
        return Err(AppError::BadGateway(format!(
            "OKX klines request failed with status {}",
            response.status
        )));
    }

    let payload = serde_json::from_str::<Value>(&response.body)?;
    ensure_okx_public_success(&payload)?;
    let mut rows = payload
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| parse_okx_kline_row(&row, &okx_interval))
        .collect::<Vec<_>>();
    rows.reverse();
    Ok(rows)
}

pub(crate) async fn fetch_bitget_symbols() -> Result<Vec<MarketSymbol>, AppError> {
    let url = "https://api.bitget.com/api/v2/mix/market/contracts?productType=USDT-FUTURES";
    let response = send_text(
        http_client().get(url),
        OutboundRequestLog::new("market.bitget.symbols", Method::GET, url),
    )
    .await?;
    if !response.status.is_success() {
        return Err(AppError::BadGateway(format!(
            "Bitget symbols request failed with status {}",
            response.status
        )));
    }

    let payload = serde_json::from_str::<Value>(&response.body)?;
    ensure_bitget_public_success(&payload)?;
    let rows = payload
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut out = Vec::new();
    for row in rows {
        let symbol = row
            .get("symbol")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_uppercase();
        let quote = row
            .get("quoteCoin")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if symbol.is_empty() || quote != "USDT" {
            continue;
        }
        out.push(MarketSymbol {
            name: symbol.clone(),
            symbol,
            category: "crypto".to_string(),
        });
        if out.len() >= 400 {
            break;
        }
    }

    Ok(out)
}

pub(crate) async fn fetch_bitget_klines(
    symbol: &str,
    interval: &str,
    limit: usize,
) -> Result<Vec<MarketKline>, AppError> {
    let bitget_interval = bitget_interval(interval);
    let url = format!(
        "https://api.bitget.com/api/v2/mix/market/candles?symbol={}&productType=USDT-FUTURES&granularity={}&limit={}",
        normalize_crypto_symbol(symbol),
        bitget_interval,
        limit.min(1000)
    );
    let response = send_text(
        http_client().get(&url),
        OutboundRequestLog::new("market.bitget.klines", Method::GET, &url),
    )
    .await?;
    if !response.status.is_success() {
        return Err(AppError::BadGateway(format!(
            "Bitget klines request failed with status {}",
            response.status
        )));
    }

    let payload = serde_json::from_str::<Value>(&response.body)?;
    ensure_bitget_public_success(&payload)?;
    let rows = payload
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| parse_bitget_kline_row(row, &bitget_interval))
        .collect::<Vec<_>>();
    Ok(rows)
}

pub(crate) async fn fetch_hyperliquid_symbols() -> Result<Vec<MarketSymbol>, AppError> {
    let payload = hyperliquid_info(json!({ "type": "meta" }), "market.hyperliquid.symbols").await?;
    let rows = payload
        .get("universe")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut out = Vec::new();
    for row in rows {
        let coin = row
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_uppercase();
        if coin.is_empty() {
            continue;
        }
        let symbol = normalize_hyperliquid_symbol(&coin);
        out.push(MarketSymbol {
            name: symbol.clone(),
            symbol,
            category: "crypto".to_string(),
        });
        if out.len() >= 400 {
            break;
        }
    }

    Ok(out)
}

pub(crate) async fn fetch_hyperliquid_klines(
    symbol: &str,
    interval: &str,
    limit: usize,
) -> Result<Vec<MarketKline>, AppError> {
    let end_time = now_millis();
    let interval_ms = interval_to_millis(interval);
    let limit = limit.clamp(1, 500);
    let start_time = end_time.saturating_sub(interval_ms.saturating_mul(limit as i64 + 1));
    let coin = hyperliquid_coin(symbol);
    let payload = hyperliquid_info(
        json!({
            "type": "candleSnapshot",
            "req": {
                "coin": coin,
                "interval": interval,
                "startTime": start_time,
                "endTime": end_time
            }
        }),
        "market.hyperliquid.klines",
    )
    .await?;
    Ok(payload
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(parse_hyperliquid_kline_row)
        .collect())
}

async fn hyperliquid_info(body: Value, log_name: &'static str) -> Result<Value, AppError> {
    let url = "https://api.hyperliquid.xyz/info";
    let response = send_text(
        http_client().post(url).json(&body),
        OutboundRequestLog::new(log_name, Method::POST, url),
    )
    .await?;
    if !response.status.is_success() {
        return Err(AppError::BadGateway(format!(
            "Hyperliquid info request failed with status {}",
            response.status
        )));
    }
    let payload = serde_json::from_str::<Value>(&response.body)?;
    if let Some(error) = payload.get("error").and_then(Value::as_str) {
        return Err(AppError::BadGateway(format!(
            "Hyperliquid market response error: {error}"
        )));
    }
    Ok(payload)
}

fn ensure_okx_public_success(payload: &Value) -> Result<(), AppError> {
    if payload.get("code").and_then(Value::as_str) == Some("0") {
        return Ok(());
    }
    Err(AppError::BadGateway(format!(
        "OKX market response error: {}",
        payload
            .get("msg")
            .and_then(Value::as_str)
            .unwrap_or("unknown error")
    )))
}

fn ensure_bitget_public_success(payload: &Value) -> Result<(), AppError> {
    if payload.get("code").and_then(Value::as_str) == Some("00000") {
        return Ok(());
    }
    Err(AppError::BadGateway(format!(
        "Bitget market response error: {}",
        payload
            .get("msg")
            .and_then(Value::as_str)
            .unwrap_or("unknown error")
    )))
}

fn parse_okx_kline_row(row: &Value, interval: &str) -> Option<MarketKline> {
    let cells = row.as_array()?;
    let open_time = parse_cell_i64(cells.first())?;
    let open = parse_cell_f64(cells.get(1))?;
    let high = parse_cell_f64(cells.get(2))?;
    let low = parse_cell_f64(cells.get(3))?;
    let close = parse_cell_f64(cells.get(4))?;
    let volume = parse_cell_f64(cells.get(5)).unwrap_or(0.0);
    let quote_volume = parse_cell_f64(cells.get(7)).unwrap_or(volume * close);
    Some(MarketKline {
        open_time,
        open,
        high,
        low,
        close,
        volume,
        quote_volume,
        close_time: open_time + interval_to_millis(interval) - 1,
    })
}

fn parse_bitget_kline_row(row: Value, interval: &str) -> Option<MarketKline> {
    let cells = row.as_array()?;
    let open_time = parse_cell_i64(cells.first())?;
    let open = parse_cell_f64(cells.get(1))?;
    let high = parse_cell_f64(cells.get(2))?;
    let low = parse_cell_f64(cells.get(3))?;
    let close = parse_cell_f64(cells.get(4))?;
    let volume = parse_cell_f64(cells.get(5)).unwrap_or(0.0);
    let quote_volume = parse_cell_f64(cells.get(6)).unwrap_or(volume * close);
    Some(MarketKline {
        open_time,
        open,
        high,
        low,
        close,
        volume,
        quote_volume,
        close_time: open_time + interval_to_millis(interval) - 1,
    })
}

fn parse_hyperliquid_kline_row(row: Value) -> Option<MarketKline> {
    let open_time = row.get("t").and_then(Value::as_i64)?;
    let close_time = row.get("T").and_then(Value::as_i64).unwrap_or(open_time);
    let open = row
        .get("o")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<f64>().ok())?;
    let high = row
        .get("h")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<f64>().ok())?;
    let low = row
        .get("l")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<f64>().ok())?;
    let close = row
        .get("c")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<f64>().ok())?;
    let volume = row
        .get("v")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.0);
    Some(MarketKline {
        open_time,
        open,
        high,
        low,
        close,
        volume,
        quote_volume: volume * close,
        close_time,
    })
}

fn parse_cell_f64(cell: Option<&Value>) -> Option<f64> {
    let cell = cell?;
    cell.as_str()
        .and_then(|value| value.parse::<f64>().ok())
        .or_else(|| cell.as_f64())
}

fn parse_cell_i64(cell: Option<&Value>) -> Option<i64> {
    let cell = cell?;
    cell.as_str()
        .and_then(|value| value.parse::<i64>().ok())
        .or_else(|| cell.as_i64())
}

fn okx_inst_id(symbol: &str) -> String {
    let upper = symbol.trim().to_uppercase();
    if upper.contains("-") {
        return upper;
    }
    upper
        .strip_suffix("USDT")
        .map(|base| format!("{base}-USDT-SWAP"))
        .unwrap_or(upper)
}

fn okx_internal_symbol(inst_id: &str) -> String {
    let upper = inst_id.trim().to_uppercase();
    upper
        .strip_suffix("-SWAP")
        .unwrap_or(&upper)
        .replace('-', "")
}

fn okx_interval(interval: &str) -> String {
    uppercase_large_interval(interval)
}

fn bitget_interval(interval: &str) -> String {
    uppercase_large_interval(interval)
}

fn uppercase_large_interval(interval: &str) -> String {
    let trimmed = interval.trim();
    if let Some(raw) = trimmed.strip_suffix('h') {
        return format!("{raw}H");
    }
    if let Some(raw) = trimmed.strip_suffix('d') {
        return format!("{raw}D");
    }
    if let Some(raw) = trimmed.strip_suffix('w') {
        return format!("{raw}W");
    }
    trimmed.to_string()
}

fn hyperliquid_coin(symbol: &str) -> String {
    let mut coin = symbol.trim().to_uppercase();
    if let Some(stripped) = coin.strip_suffix("USDT") {
        coin = stripped.to_string();
    } else if let Some(stripped) = coin.strip_suffix("USDC") {
        coin = stripped.to_string();
    }
    coin
}

fn normalize_hyperliquid_symbol(coin: &str) -> String {
    format!("{}USDT", coin.trim().to_uppercase())
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn interval_to_millis(interval: &str) -> i64 {
    let trimmed = interval.trim();
    if let Some(raw) = trimmed.strip_suffix('m') {
        return raw.parse::<i64>().unwrap_or(1).max(1) * 60_000;
    }
    if let Some(raw) = trimmed.strip_suffix('H') {
        return raw.parse::<i64>().unwrap_or(1).max(1) * 60 * 60_000;
    }
    if let Some(raw) = trimmed.strip_suffix('h') {
        return raw.parse::<i64>().unwrap_or(1).max(1) * 60 * 60_000;
    }
    if let Some(raw) = trimmed.strip_suffix('D') {
        return raw.parse::<i64>().unwrap_or(1).max(1) * 24 * 60 * 60_000;
    }
    if let Some(raw) = trimmed.strip_suffix('d') {
        return raw.parse::<i64>().unwrap_or(1).max(1) * 24 * 60 * 60_000;
    }
    if let Some(raw) = trimmed.strip_suffix('W') {
        return raw.parse::<i64>().unwrap_or(1).max(1) * 7 * 24 * 60 * 60_000;
    }
    if let Some(raw) = trimmed.strip_suffix('w') {
        return raw.parse::<i64>().unwrap_or(1).max(1) * 7 * 24 * 60 * 60_000;
    }
    60_000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exchange_intervals_normalize_hour_and_day_case() {
        assert_eq!(okx_interval("1h"), "1H");
        assert_eq!(okx_interval("1d"), "1D");
        assert_eq!(bitget_interval("4h"), "4H");
        assert_eq!(okx_interval("1w"), "1W");
        assert_eq!(interval_to_millis("1d"), 86_400_000);
        assert_eq!(interval_to_millis("1w"), 604_800_000);
    }

    #[test]
    fn hyperliquid_symbols_use_internal_usdt_suffix() {
        assert_eq!(hyperliquid_coin("BTCUSDT"), "BTC");
        assert_eq!(normalize_hyperliquid_symbol("BTC"), "BTCUSDT");
    }
}
