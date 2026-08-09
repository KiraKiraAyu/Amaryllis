use super::*;

#[derive(Debug, Deserialize)]
pub(super) struct OkxEnvelope<T> {
    pub(super) code: String,
    #[serde(default)]
    pub(super) msg: String,
    pub(super) data: T,
}

#[derive(Debug, Deserialize)]
pub(super) struct OkxTimePayload {}

#[derive(Debug, Deserialize)]
pub(super) struct OkxTicker {
    pub(super) last: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OkxInstrument {
    pub(super) inst_id: String,
    #[serde(default)]
    pub(super) base_ccy: String,
    #[serde(default)]
    pub(super) quote_ccy: String,
    pub(super) min_sz: String,
    pub(super) lot_sz: String,
    pub(super) tick_sz: String,
    #[serde(default)]
    pub(super) ct_val: String,
    #[serde(default)]
    pub(super) max_lmt_sz: String,
    #[serde(default)]
    pub(super) max_mkt_sz: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OkxAccountConfig {
    #[serde(default)]
    pub(super) pos_mode: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OkxPositionMode {
    Net,
    LongShort,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OkxOrderAck {
    #[serde(default)]
    pub(super) ord_id: String,
    #[serde(default)]
    pub(super) cl_ord_id: String,
    #[serde(default)]
    pub(super) inst_id: String,
    #[serde(default)]
    pub(super) s_code: Option<String>,
    #[serde(default)]
    pub(super) s_msg: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct OkxBalanceRoot {
    #[serde(default)]
    pub(super) details: Vec<OkxBalanceDetail>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OkxBalanceDetail {
    pub(super) ccy: String,
    #[serde(default)]
    pub(super) eq: String,
    #[serde(default)]
    pub(super) cash_bal: String,
    #[serde(default)]
    pub(super) avail_eq: String,
    #[serde(default)]
    pub(super) avail_bal: String,
    #[serde(default)]
    pub(super) upl: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OkxPositionRow {
    pub(super) inst_id: String,
    pub(super) pos_side: String,
    pub(super) pos: String,
    pub(super) avg_px: String,
    pub(super) mark_px: String,
    pub(super) upl: String,
    pub(super) lever: String,
    #[serde(default)]
    pub(super) liq_px: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OkxOrderRow {
    pub(super) ord_id: String,
    #[serde(default)]
    pub(super) cl_ord_id: String,
    pub(super) inst_id: String,
    pub(super) side: String,
    #[serde(default)]
    pub(super) pos_side: String,
    #[serde(default)]
    pub(super) reduce_only: String,
    pub(super) ord_type: String,
    #[serde(default)]
    pub(super) px: String,
    pub(super) sz: String,
    #[serde(default)]
    pub(super) acc_fill_sz: String,
    pub(super) state: String,
    #[serde(default)]
    pub(super) u_time: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OkxFillRow {
    pub(super) trade_id: String,
    pub(super) ord_id: String,
    pub(super) inst_id: String,
    pub(super) side: String,
    pub(super) fill_px: String,
    pub(super) fill_sz: String,
    pub(super) fee: String,
    pub(super) fee_ccy: String,
    #[serde(default)]
    pub(super) fill_pnl: String,
    pub(super) ts: String,
}

pub(super) fn parse_okx_response<T: for<'de> Deserialize<'de>>(
    resp: OutboundResponse,
) -> Result<T, AppError> {
    let status = resp.status;
    let body = resp.body;
    if !status.is_success() {
        return Err(exchange_api_error(status, 0, body));
    }

    let envelope: OkxEnvelope<T> = serde_json::from_str(&body).map_err(AppError::ExchangeJson)?;
    if envelope.code != "0" {
        return Err(exchange_api_error(
            status,
            envelope.code.parse::<i64>().unwrap_or(0),
            envelope.msg,
        ));
    }
    Ok(envelope.data)
}

pub(super) fn exchange_api_error(status: StatusCode, code: i64, message: String) -> AppError {
    AppError::ExchangeApi {
        status: status.as_u16(),
        code: if code == 0 {
            i64::from(status.as_u16())
        } else {
            code
        },
        message,
    }
}

pub(super) fn ensure_okx_item_success(
    code: Option<&str>,
    message: Option<&str>,
) -> Result<(), AppError> {
    if matches!(code, None | Some("") | Some("0")) {
        return Ok(());
    }
    Err(AppError::ExchangeApi {
        status: 200,
        code: code.and_then(|v| v.parse::<i64>().ok()).unwrap_or(0),
        message: message.unwrap_or("okx item error").to_string(),
    })
}

pub(super) fn sign_okx(
    secret: &str,
    timestamp: &str,
    method: &str,
    request_path: &str,
    body: &str,
) -> Result<String, AppError> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| AppError::ExchangeCrypto(e.to_string()))?;
    mac.update(format!("{timestamp}{method}{request_path}{body}").as_bytes());
    Ok(general_purpose::STANDARD.encode(mac.finalize().into_bytes()))
}

pub(super) fn request_path(path: &str, params: Vec<(&str, String)>) -> String {
    let query = build_query(params);
    if query.is_empty() {
        path.to_string()
    } else {
        format!("{path}?{query}")
    }
}

pub(super) fn build_query(params: Vec<(&str, String)>) -> String {
    params
        .into_iter()
        .map(|(k, v)| format!("{}={}", encode_component(k), encode_component(&v)))
        .collect::<Vec<_>>()
        .join("&")
}

pub(super) fn encode_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(b))
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{:02X}", b));
            }
        }
    }
    out
}

pub(super) fn okx_inst_id(symbol: &str) -> String {
    let upper = symbol.trim().to_uppercase();
    if upper.contains("-") {
        return upper;
    }
    upper
        .strip_suffix("USDT")
        .map(|base| format!("{base}-USDT-SWAP"))
        .unwrap_or(upper)
}

pub(super) fn internal_symbol(inst_id: &str) -> String {
    let upper = inst_id.trim().to_uppercase();
    upper
        .strip_suffix("-SWAP")
        .unwrap_or(&upper)
        .replace('-', "")
}

pub(super) fn okx_side(side: ExchangeSide) -> &'static str {
    match side {
        ExchangeSide::Buy => "buy",
        ExchangeSide::Sell => "sell",
    }
}

pub(super) fn okx_order_type(
    order_type: ExchangeOrderType,
    tif: Option<TimeInForce>,
) -> &'static str {
    match order_type {
        ExchangeOrderType::Market => "market",
        ExchangeOrderType::Limit => match tif.unwrap_or(TimeInForce::Gtc) {
            TimeInForce::Gtc => "limit",
            TimeInForce::Ioc => "ioc",
            TimeInForce::Fok => "fok",
        },
    }
}

pub(super) fn okx_pos_side(side: PositionSide) -> &'static str {
    match side {
        PositionSide::Both => "net",
        PositionSide::Long => "long",
        PositionSide::Short => "short",
    }
}

pub(super) fn okx_td_mode(mode: ExchangeMarginMode) -> &'static str {
    match mode {
        ExchangeMarginMode::Cross => "cross",
        ExchangeMarginMode::Isolated => "isolated",
    }
}

pub(super) fn okx_position_mode(pos_mode: &str) -> OkxPositionMode {
    match pos_mode.trim().to_ascii_lowercase().as_str() {
        "long_short_mode" => OkxPositionMode::LongShort,
        _ => OkxPositionMode::Net,
    }
}

pub(super) fn inferred_position_side_for_order(
    side: ExchangeSide,
    reduce_only: bool,
) -> PositionSide {
    match (side, reduce_only) {
        (ExchangeSide::Sell, false) | (ExchangeSide::Buy, true) => PositionSide::Short,
        _ => PositionSide::Long,
    }
}

pub(super) fn okx_position_side_label(pos_side: &str, quantity: f64) -> String {
    match pos_side.trim().to_ascii_lowercase().as_str() {
        "long" => "LONG".to_string(),
        "short" => "SHORT".to_string(),
        _ if quantity < 0.0 => "SHORT".to_string(),
        _ => "LONG".to_string(),
    }
}

pub(super) fn okx_order_position_side_label(
    pos_side: &str,
    side: &str,
    reduce_only: bool,
) -> String {
    match pos_side.trim().to_ascii_lowercase().as_str() {
        "long" => "LONG".to_string(),
        "short" => "SHORT".to_string(),
        _ => match (side.trim().to_ascii_lowercase().as_str(), reduce_only) {
            ("sell", false) | ("buy", true) => "SHORT".to_string(),
            _ => "LONG".to_string(),
        },
    }
}

pub(super) fn okx_contract_size(inst: &OkxInstrument) -> f64 {
    positive_or_default(parse_f64(&inst.ct_val), 1.0)
}

pub(super) fn okx_order_size_contracts(
    quantity: f64,
    inst: &OkxInstrument,
) -> Result<f64, AppError> {
    let contract_size = okx_contract_size(inst);
    let raw_contracts = quantity / contract_size;
    let min_contracts = parse_f64(&inst.min_sz);
    let lot_size = positive_or_default(parse_f64(&inst.lot_sz), 1.0);
    if raw_contracts + f64::EPSILON < min_contracts {
        return Err(AppError::InvalidExchangeConfig(format!(
            "okx order quantity below min size: quantity={} min_qty={}",
            quantity,
            min_contracts * contract_size
        )));
    }

    let rounded_contracts = floor_to_step(raw_contracts, lot_size);
    if rounded_contracts + f64::EPSILON < min_contracts {
        return Err(AppError::InvalidExchangeConfig(format!(
            "okx order quantity below min size after lot rounding: quantity={} min_qty={}",
            quantity,
            min_contracts * contract_size
        )));
    }

    Ok(rounded_contracts)
}

pub(super) fn okx_max_qty(inst: &OkxInstrument) -> f64 {
    min_positive([parse_f64(&inst.max_lmt_sz), parse_f64(&inst.max_mkt_sz)]).unwrap_or(0.0)
}

pub(super) fn okx_instrument_from_map<'a>(
    instruments: &'a HashMap<String, OkxInstrument>,
    inst_id: &str,
) -> Result<&'a OkxInstrument, AppError> {
    instruments.get(inst_id).ok_or_else(|| {
        AppError::InvalidExchangeConfig(format!("okx instrument not found: {inst_id}"))
    })
}

pub(super) fn positive_or_default(value: f64, fallback: f64) -> f64 {
    if value > 0.0 { value } else { fallback }
}

pub(super) fn min_positive(values: impl IntoIterator<Item = f64>) -> Option<f64> {
    values
        .into_iter()
        .filter(|value| *value > 0.0)
        .min_by(|a, b| a.total_cmp(b))
}

pub(super) fn first_present(values: impl IntoIterator<Item = Option<f64>>) -> Option<f64> {
    values.into_iter().flatten().next()
}

pub(super) fn floor_to_step(value: f64, step: f64) -> f64 {
    if step <= 0.0 {
        return value;
    }
    (value / step).floor() * step
}

pub(super) fn position_side_label(side: PositionSide) -> String {
    match side {
        PositionSide::Both => "BOTH",
        PositionSide::Long => "LONG",
        PositionSide::Short => "SHORT",
    }
    .to_string()
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

pub(super) fn okx_open_order(row: OkxOrderRow, ct_val: f64) -> ExchangeOpenOrder {
    let reduce_only = row.reduce_only == "true";
    ExchangeOpenOrder {
        order_id: row.ord_id,
        client_order_id: row.cl_ord_id,
        symbol: internal_symbol(&row.inst_id),
        side: row.side.to_ascii_uppercase(),
        position_side: okx_order_position_side_label(&row.pos_side, &row.side, reduce_only),
        reduce_only,
        order_type: row.ord_type.to_ascii_uppercase(),
        status: row.state,
        price: parse_f64(&row.px),
        orig_qty: parse_f64(&row.sz) * ct_val,
        executed_qty: parse_f64(&row.acc_fill_sz) * ct_val,
        update_time: row.u_time.parse::<i64>().unwrap_or(0),
    }
}

pub(super) fn okx_order_detail(row: OkxOrderRow, ct_val: f64) -> ExchangeOrderDetail {
    let reduce_only = row.reduce_only == "true";
    ExchangeOrderDetail {
        order_id: row.ord_id,
        client_order_id: row.cl_ord_id,
        symbol: internal_symbol(&row.inst_id),
        side: row.side.to_ascii_uppercase(),
        position_side: okx_order_position_side_label(&row.pos_side, &row.side, reduce_only),
        reduce_only,
        order_type: row.ord_type.to_ascii_uppercase(),
        status: row.state,
        price: parse_f64(&row.px),
        orig_qty: parse_f64(&row.sz) * ct_val,
        executed_qty: parse_f64(&row.acc_fill_sz) * ct_val,
        update_time: row.u_time.parse::<i64>().unwrap_or(0),
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

pub(super) fn parse_f64(v: &str) -> f64 {
    v.parse::<f64>().unwrap_or(0.0)
}

pub(super) fn parse_optional_f64(v: &str) -> Option<f64> {
    let trimmed = v.trim();
    if trimmed.is_empty() {
        None
    } else {
        trimmed.parse::<f64>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn okx_inst_id_normalizes_usdt_symbols() {
        assert_eq!(okx_inst_id("BTCUSDT"), "BTC-USDT-SWAP");
        assert_eq!(internal_symbol("BTC-USDT-SWAP"), "BTCUSDT");
    }

    #[test]
    fn okx_signing_matches_known_hmac_shape() {
        let signature = sign_okx(
            "secret",
            "2020-12-08T09:08:57.715Z",
            "GET",
            "/api/v5/account/balance?ccy=BTC",
            "",
        )
        .expect("sign");

        assert!(!signature.is_empty());
    }

    #[test]
    fn okx_contract_size_preserves_fractional_contract_value() {
        let inst = OkxInstrument {
            inst_id: "BTC-USDT-SWAP".to_string(),
            base_ccy: "BTC".to_string(),
            quote_ccy: "USDT".to_string(),
            min_sz: "1".to_string(),
            lot_sz: "1".to_string(),
            tick_sz: "0.1".to_string(),
            ct_val: "0.01".to_string(),
            max_lmt_sz: "1000".to_string(),
            max_mkt_sz: "500".to_string(),
        };

        assert_eq!(okx_contract_size(&inst), 0.01);
    }

    #[test]
    fn okx_order_size_rejects_below_min_instead_of_inflating() {
        let inst = OkxInstrument {
            inst_id: "BTC-USDT-SWAP".to_string(),
            base_ccy: "BTC".to_string(),
            quote_ccy: "USDT".to_string(),
            min_sz: "1".to_string(),
            lot_sz: "1".to_string(),
            tick_sz: "0.1".to_string(),
            ct_val: "0.01".to_string(),
            max_lmt_sz: "1000".to_string(),
            max_mkt_sz: "500".to_string(),
        };

        assert!(okx_order_size_contracts(0.001, &inst).is_err());
        assert_eq!(okx_order_size_contracts(0.012, &inst).expect("size"), 1.0);
    }

    #[test]
    fn okx_max_qty_uses_conservative_positive_limit() {
        let inst = OkxInstrument {
            inst_id: "BTC-USDT-SWAP".to_string(),
            base_ccy: "BTC".to_string(),
            quote_ccy: "USDT".to_string(),
            min_sz: "1".to_string(),
            lot_sz: "1".to_string(),
            tick_sz: "0.1".to_string(),
            ct_val: "0.01".to_string(),
            max_lmt_sz: "1000".to_string(),
            max_mkt_sz: "500".to_string(),
        };

        assert_eq!(okx_max_qty(&inst), 500.0);
    }

    #[test]
    fn okx_order_position_side_uses_side_when_pos_side_is_net() {
        assert_eq!(okx_order_position_side_label("net", "sell", false), "SHORT");
        assert_eq!(okx_order_position_side_label("net", "buy", true), "SHORT");
        assert_eq!(okx_order_position_side_label("long", "sell", true), "LONG");
    }
}
