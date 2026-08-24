use super::*;

#[derive(Debug, Clone)]
pub(super) struct HyperliquidAsset {
    pub(super) index: usize,
    pub(super) coin: String,
    pub(super) sz_decimals: i32,
    pub(super) max_leverage: i64,
}

#[derive(Debug, Serialize)]
pub(super) struct HyperliquidExchangeRequest<'a, T> {
    pub(super) action: &'a T,
    pub(super) nonce: i64,
    pub(super) signature: HyperliquidSignature,
    #[serde(rename = "vaultAddress")]
    pub(super) vault_address: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub(super) enum HyperliquidAction {
    #[serde(rename = "order")]
    Order {
        orders: Vec<HyperliquidOrderAction>,
        grouping: String,
    },
    #[serde(rename = "cancel")]
    Cancel {
        cancels: Vec<HyperliquidCancelAction>,
    },
    #[serde(rename = "updateLeverage")]
    UpdateLeverage {
        asset: usize,
        #[serde(rename = "isCross")]
        is_cross: bool,
        leverage: u32,
    },
}

#[derive(Debug, Serialize)]
pub(super) struct HyperliquidOrderAction {
    pub(super) a: usize,
    pub(super) b: bool,
    pub(super) p: String,
    pub(super) s: String,
    pub(super) r: bool,
    pub(super) t: HyperliquidOrderWireType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) c: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct HyperliquidOrderWireType {
    pub(super) limit: HyperliquidLimitType,
}

#[derive(Debug, Serialize)]
pub(super) struct HyperliquidLimitType {
    pub(super) tif: String,
}

#[derive(Debug, Serialize)]
pub(super) struct HyperliquidCancelAction {
    pub(super) a: usize,
    pub(super) o: i64,
}

#[derive(Debug, Serialize)]
pub(super) struct HyperliquidSignature {
    pub(super) r: String,
    pub(super) s: String,
    pub(super) v: i64,
}

#[derive(Debug, Deserialize)]
pub(super) struct HyperliquidError {
    #[serde(default)]
    pub(super) error: String,
}

pub(super) fn parse_json_response(resp: OutboundResponse) -> Result<Value, AppError> {
    let status = resp.status;
    let body = resp.body;
    if !status.is_success() {
        return Err(exchange_api_error(status, body));
    }
    if let Ok(error) = serde_json::from_str::<HyperliquidError>(&body)
        && !error.error.trim().is_empty()
    {
        return Err(exchange_api_error(status, error.error));
    }
    serde_json::from_str::<Value>(&body).map_err(AppError::ExchangeJson)
}

pub(super) fn exchange_api_error(status: StatusCode, message: String) -> AppError {
    AppError::ExchangeApi {
        status: status.as_u16(),
        code: i64::from(status.as_u16()),
        message,
    }
}

pub(super) fn ensure_hyperliquid_status_ok(response: &Value) -> Result<(), AppError> {
    if response.get("status").and_then(Value::as_str) != Some("ok") {
        return Err(AppError::ExchangeApi {
            status: 200,
            code: 0,
            message: response.to_string(),
        });
    }

    if let Some(statuses) = response
        .pointer("/response/data/statuses")
        .and_then(Value::as_array)
    {
        for status in statuses {
            if let Some(error) = status.get("error").and_then(Value::as_str) {
                return Err(AppError::ExchangeApi {
                    status: 200,
                    code: 0,
                    message: error.to_string(),
                });
            }
        }
    }

    Ok(())
}

pub(super) fn sign_l1_action(
    private_key: &str,
    action: &impl Serialize,
    nonce: i64,
    vault_address: Option<&str>,
    source: &str,
) -> Result<HyperliquidSignature, AppError> {
    let connection_id = action_hash(action, nonce, vault_address)?;
    let digest = eip712_agent_digest(source, &connection_id);
    let private_key_bytes = hex::decode(private_key.trim().trim_start_matches("0x"))
        .map_err(|e| AppError::ExchangeCrypto(e.to_string()))?;
    let signing_key = SigningKey::from_slice(&private_key_bytes)
        .map_err(|e| AppError::ExchangeCrypto(e.to_string()))?;
    let (signature, recovery_id) = signing_key
        .sign_prehash_recoverable(&digest)
        .map_err(|e| AppError::ExchangeCrypto(e.to_string()))?;
    let signature_bytes = signature.to_bytes();
    let r = &signature_bytes[..32];
    let s = &signature_bytes[32..];

    Ok(HyperliquidSignature {
        r: format!("0x{}", hex::encode(r)),
        s: format!("0x{}", hex::encode(s)),
        v: i64::from(recovery_id.to_byte()) + 27,
    })
}

pub(super) fn action_hash(
    action: &impl Serialize,
    nonce: i64,
    vault_address: Option<&str>,
) -> Result<[u8; 32], AppError> {
    let mut bytes =
        rmp_serde::to_vec_named(action).map_err(|e| AppError::ExchangeCrypto(e.to_string()))?;
    bytes.extend_from_slice(&(nonce as u64).to_be_bytes());
    match vault_address {
        Some(address) => {
            bytes.push(1);
            bytes.extend_from_slice(&address_to_bytes(address)?);
        }
        None => bytes.push(0),
    }
    Ok(keccak256(&bytes))
}

pub(super) fn eip712_agent_digest(source: &str, connection_id: &[u8; 32]) -> [u8; 32] {
    let domain_separator = eip712_domain_separator();
    let agent_typehash = keccak256(b"Agent(string source,bytes32 connectionId)");
    let source_hash = keccak256(source.as_bytes());
    let mut agent_bytes = Vec::with_capacity(96);
    agent_bytes.extend_from_slice(&agent_typehash);
    agent_bytes.extend_from_slice(&source_hash);
    agent_bytes.extend_from_slice(connection_id);
    let agent_hash = keccak256(&agent_bytes);

    let mut digest_bytes = Vec::with_capacity(66);
    digest_bytes.extend_from_slice(b"\x19\x01");
    digest_bytes.extend_from_slice(&domain_separator);
    digest_bytes.extend_from_slice(&agent_hash);
    keccak256(&digest_bytes)
}

pub(super) fn eip712_domain_separator() -> [u8; 32] {
    let domain_typehash = keccak256(
        b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
    );
    let name_hash = keccak256(b"Exchange");
    let version_hash = keccak256(b"1");
    let mut chain_id = [0_u8; 32];
    chain_id[30] = 0x05;
    chain_id[31] = 0x39;
    let verifying_contract = [0_u8; 32];

    let mut bytes = Vec::with_capacity(160);
    bytes.extend_from_slice(&domain_typehash);
    bytes.extend_from_slice(&name_hash);
    bytes.extend_from_slice(&version_hash);
    bytes.extend_from_slice(&chain_id);
    bytes.extend_from_slice(&verifying_contract);
    keccak256(&bytes)
}

pub(super) fn address_to_bytes(address: &str) -> Result<[u8; 20], AppError> {
    let decoded = hex::decode(address.trim().trim_start_matches("0x"))
        .map_err(|e| AppError::ExchangeCrypto(e.to_string()))?;
    if decoded.len() != 20 {
        return Err(AppError::ExchangeCrypto(
            "ethereum address must be 20 bytes".to_string(),
        ));
    }
    let mut out = [0_u8; 20];
    out.copy_from_slice(&decoded);
    Ok(out)
}

pub(super) fn keccak256(input: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(input);
    hasher.finalize().into()
}

pub(super) fn hyperliquid_coin(symbol: &str) -> String {
    let mut coin = symbol.trim().to_uppercase();
    if let Some(stripped) = coin.strip_suffix("USDT") {
        coin = stripped.to_string();
    } else if let Some(stripped) = coin.strip_suffix("USDC") {
        coin = stripped.to_string();
    }
    coin
}

pub(super) fn internal_symbol(coin: &str) -> String {
    format!("{}USDT", coin.trim().to_uppercase())
}

pub(super) fn hyperliquid_cloid(value: Option<String>) -> String {
    if let Some(value) = value {
        let trimmed = value.trim();
        if is_valid_hyperliquid_cloid(trimmed) {
            return trimmed.to_string();
        }
    }
    format!("0x{}", Uuid::now_v7().simple())
}

pub(super) fn is_valid_hyperliquid_cloid(value: &str) -> bool {
    value.len() == 34
        && value.starts_with("0x")
        && value
            .as_bytes()
            .iter()
            .skip(2)
            .all(|b| b.is_ascii_hexdigit())
}

pub(super) fn normalize_address(address: &str) -> String {
    format!(
        "0x{}",
        address.trim().trim_start_matches("0x").to_ascii_lowercase()
    )
}

pub(super) fn exchange_side_label(side: ExchangeSide) -> &'static str {
    match side {
        ExchangeSide::Buy => "BUY",
        ExchangeSide::Sell => "SELL",
    }
}

pub(super) fn order_type_label(order_type: ExchangeOrderType) -> &'static str {
    match order_type {
        ExchangeOrderType::Market => "MARKET",
        ExchangeOrderType::Limit => "LIMIT",
    }
}

pub(super) fn position_side_label(side: PositionSide) -> String {
    match side {
        PositionSide::Both => "BOTH",
        PositionSide::Long => "LONG",
        PositionSide::Short => "SHORT",
    }
    .to_string()
}

pub(super) fn hyperliquid_open_order(row: &Value) -> ExchangeOpenOrder {
    let side = row.get("side").and_then(Value::as_str).unwrap_or("");
    let reduce_only = row
        .get("reduceOnly")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let remaining_qty = row
        .get("sz")
        .and_then(Value::as_str)
        .map(parse_f64)
        .unwrap_or(0.0);
    let original_qty = row
        .get("origSz")
        .and_then(Value::as_str)
        .map(parse_f64)
        .unwrap_or(remaining_qty);
    ExchangeOpenOrder {
        order_id: row
            .get("oid")
            .and_then(Value::as_i64)
            .map(|v| v.to_string())
            .unwrap_or_default(),
        client_order_id: row
            .get("cloid")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        symbol: row
            .get("coin")
            .and_then(Value::as_str)
            .map(internal_symbol)
            .unwrap_or_default(),
        side: hyperliquid_side_label(side).to_string(),
        position_side: hyperliquid_position_side_label(side, reduce_only).to_string(),
        reduce_only,
        order_type: row
            .get("orderType")
            .and_then(Value::as_str)
            .unwrap_or("Limit")
            .to_ascii_uppercase(),
        status: "open".to_string(),
        price: row
            .get("limitPx")
            .and_then(Value::as_str)
            .map(parse_f64)
            .unwrap_or(0.0),
        orig_qty: original_qty,
        executed_qty: (original_qty - remaining_qty).max(0.0),
        update_time: row.get("timestamp").and_then(Value::as_i64).unwrap_or(0),
    }
}

pub(super) fn hyperliquid_order_status_detail(
    row: &Value,
    fallback_order_id: &str,
) -> ExchangeOrderDetail {
    let status = row.get("status").and_then(Value::as_str).unwrap_or("");

    if status == "order" {
        let order_wrapper = row.get("order").unwrap_or(row);
        let order = order_wrapper.get("order").unwrap_or(order_wrapper);
        let mut detail = hyperliquid_order_detail(order);
        detail.status = order_wrapper
            .get("status")
            .and_then(Value::as_str)
            .map(hyperliquid_status_label)
            .unwrap_or("open")
            .to_string();
        if detail.update_time == 0 {
            detail.update_time = order_wrapper
                .get("statusTimestamp")
                .and_then(Value::as_i64)
                .unwrap_or(0);
        }
        if detail.order_id.is_empty() {
            detail.order_id = fallback_order_id.to_string();
        }
        return detail;
    }

    ExchangeOrderDetail {
        order_id: fallback_order_id.to_string(),
        client_order_id: String::new(),
        symbol: String::new(),
        side: String::new(),
        position_side: String::new(),
        reduce_only: false,
        order_type: String::new(),
        status: hyperliquid_status_label(status).to_string(),
        price: 0.0,
        orig_qty: 0.0,
        executed_qty: 0.0,
        update_time: 0,
    }
}

pub(super) fn hyperliquid_order_detail(row: &Value) -> ExchangeOrderDetail {
    let open = hyperliquid_open_order(row);
    ExchangeOrderDetail {
        order_id: open.order_id,
        client_order_id: open.client_order_id,
        symbol: open.symbol,
        side: open.side,
        position_side: open.position_side,
        reduce_only: open.reduce_only,
        order_type: open.order_type,
        status: row
            .get("status")
            .and_then(Value::as_str)
            .map(hyperliquid_status_label)
            .unwrap_or(&open.status)
            .to_string(),
        price: open.price,
        orig_qty: open.orig_qty,
        executed_qty: open.executed_qty,
        update_time: open.update_time,
    }
}

pub(super) fn hyperliquid_fill(row: &Value) -> ExchangeTradeFill {
    ExchangeTradeFill {
        trade_id: row
            .get("tid")
            .and_then(Value::as_i64)
            .map(|v| v.to_string())
            .unwrap_or_default(),
        order_id: row
            .get("oid")
            .and_then(Value::as_i64)
            .map(|v| v.to_string())
            .unwrap_or_default(),
        symbol: row
            .get("coin")
            .and_then(Value::as_str)
            .map(internal_symbol)
            .unwrap_or_default(),
        side: hyperliquid_side_label(row.get("side").and_then(Value::as_str).unwrap_or(""))
            .to_string(),
        price: row
            .get("px")
            .and_then(Value::as_str)
            .map(parse_f64)
            .unwrap_or(0.0),
        quantity: row
            .get("sz")
            .and_then(Value::as_str)
            .map(parse_f64)
            .unwrap_or(0.0),
        fee: row
            .get("fee")
            .and_then(Value::as_str)
            .map(parse_f64)
            .unwrap_or(0.0),
        fee_asset: row
            .get("feeToken")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        realized_pnl: row
            .get("closedPnl")
            .and_then(Value::as_str)
            .map(parse_f64)
            .unwrap_or(0.0),
        executed_at: row.get("time").and_then(Value::as_i64).unwrap_or(0),
    }
}

pub(super) fn hyperliquid_side_label(side: &str) -> &'static str {
    match side {
        "A" => "SELL",
        _ => "BUY",
    }
}

pub(super) fn hyperliquid_position_side_label(side: &str, reduce_only: bool) -> &'static str {
    match (side, reduce_only) {
        ("A", false) | ("B", true) => "SHORT",
        _ => "LONG",
    }
}

pub(super) fn hyperliquid_status_label(status: &str) -> &'static str {
    match status.trim().to_ascii_lowercase().as_str() {
        "filled" => "filled",
        "canceled"
        | "cancelled"
        | "margincanceled"
        | "vaultwithdrawalcanceled"
        | "openinterestcapcanceled"
        | "selftradecanceled"
        | "reduceonlycanceled"
        | "siblingfilledcanceled"
        | "delistedcanceled"
        | "liquidatedcanceled"
        | "scheduledcancel" => "canceled",
        "rejected" => "rejected",
        "unknownoid" => "expired",
        _ => "open",
    }
}

pub(super) fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub(super) fn format_decimal(v: f64) -> String {
    let mut s = format!("{:.10}", v);
    while s.contains('.') && s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    if s.is_empty() { "0".to_string() } else { s }
}

pub(super) fn format_hyperliquid_size(value: f64, sz_decimals: i32) -> String {
    format_decimal(round_to_decimals(value.max(0.0), sz_decimals.max(0)))
}

pub(super) fn format_hyperliquid_price(value: f64, sz_decimals: i32) -> String {
    let max_decimals = (6 - sz_decimals).max(0);
    let rounded = round_to_decimals(
        round_to_significant_figures(value.max(0.0), 5),
        max_decimals,
    );
    format_decimal(rounded)
}

pub(super) fn hyperliquid_price_tick(price: f64, sz_decimals: i32) -> f64 {
    let decimal_tick = 10_f64.powi(-(6 - sz_decimals).max(0));
    if price <= 0.0 {
        return decimal_tick;
    }
    let significant_tick = 10_f64.powf(price.abs().log10().floor() - 4.0);
    significant_tick.max(decimal_tick)
}

pub(super) fn round_to_decimals(value: f64, decimals: i32) -> f64 {
    if decimals <= 0 {
        return value.round();
    }
    let factor = 10_f64.powi(decimals);
    (value * factor).round() / factor
}

pub(super) fn round_to_significant_figures(value: f64, sig_figs: i32) -> f64 {
    if value == 0.0 {
        return 0.0;
    }
    let exponent = value.abs().log10().floor();
    let scale = 10_f64.powf((sig_figs as f64 - 1.0) - exponent);
    (value * scale).round() / scale
}

pub(super) fn parse_f64(v: &str) -> f64 {
    v.parse::<f64>().unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hyperliquid_coin_normalizes_internal_symbols() {
        assert_eq!(hyperliquid_coin("BTCUSDT"), "BTC");
        assert_eq!(hyperliquid_coin("ETHUSDC"), "ETH");
        assert_eq!(internal_symbol("BTC"), "BTCUSDT");
    }

    #[test]
    fn hyperliquid_action_hash_is_stable_shape() {
        let hash = action_hash(
            &HyperliquidAction::Cancel {
                cancels: vec![HyperliquidCancelAction { a: 0, o: 1 }],
            },
            1,
            None,
        )
        .expect("hash");

        assert_eq!(
            hex::encode(hash),
            "da5917dce1db000bb4b11641e06875146264750f67d6196ee1bd75991d84062a"
        );
    }

    #[test]
    fn hyperliquid_generates_valid_cloid_when_runtime_id_is_not_valid() {
        let cloid = hyperliquid_cloid(Some("nfx_lmt_not_valid".to_string()));

        assert!(is_valid_hyperliquid_cloid(&cloid));
    }

    #[test]
    fn hyperliquid_order_status_parses_nested_order_payload() {
        let detail = hyperliquid_order_status_detail(
            &json!({
                "status": "order",
                "order": {
                    "order": {
                        "coin": "BTC",
                        "side": "B",
                        "limitPx": "65000",
                        "sz": "0.01",
                        "origSz": "0.02",
                        "oid": 123,
                        "timestamp": 1710000000000_i64,
                        "reduceOnly": false,
                        "orderType": "Limit"
                    },
                    "status": "filled",
                    "statusTimestamp": 1710000001000_i64
                }
            }),
            "123",
        );

        assert_eq!(detail.order_id, "123");
        assert_eq!(detail.symbol, "BTCUSDT");
        assert_eq!(detail.side, "BUY");
        assert_eq!(detail.position_side, "LONG");
        assert_eq!(detail.status, "filled");
        assert_eq!(detail.orig_qty, 0.02);
        assert_eq!(detail.executed_qty, 0.01);
    }

    #[test]
    fn hyperliquid_formats_price_with_exchange_precision_rules() {
        assert_eq!(format_hyperliquid_price(65012.345, 5), "65012");
        assert_eq!(format_hyperliquid_size(0.123456, 4), "0.1235");
        assert_eq!(hyperliquid_price_tick(65012.345, 5), 1.0);
    }

    #[test]
    fn hyperliquid_exchange_response_surfaces_nested_order_errors() {
        let err = ensure_hyperliquid_status_ok(&json!({
            "status": "ok",
            "response": {
                "type": "order",
                "data": {
                    "statuses": [{ "error": "Insufficient margin" }]
                }
            }
        }))
        .expect_err("nested error should fail");

        assert!(err.to_string().contains("Insufficient margin"));
    }
}
