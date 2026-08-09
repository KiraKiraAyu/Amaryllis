pub mod binance;
pub mod bitget;
pub mod exchanges;
pub mod hyperliquid;
pub mod llm_chat;
pub mod market_data;
pub mod okx;
pub mod aster;
pub mod outbound_http;

/// Maximum length allowed by Binance-compatible APIs for `newClientOrderId`.
/// Regex: `^[\.A-Z\:/a-z0-9_-]{1,36}$`
pub const MAX_CLIENT_ORDER_ID_LEN: usize = 36;

/// Generate a client order ID that fits within `MAX_CLIENT_ORDER_ID_LEN`.
///
/// The UUID v7 `.simple()` output is 32 hex chars. When combined with the
/// prefix it may exceed the 36-char limit. This function truncates the UUID
/// to fit, preserving the timestamp-sorted prefix of the UUID for traceability.
pub fn short_client_order_id(prefix: &str) -> String {
    let uuid = uuid::Uuid::now_v7().simple().to_string();
    let max_uuid_len = MAX_CLIENT_ORDER_ID_LEN.saturating_sub(prefix.len());
    let uuid_part = if uuid.len() > max_uuid_len {
        &uuid[..max_uuid_len]
    } else {
        &uuid
    };
    format!("{}{}", prefix, uuid_part)
}
