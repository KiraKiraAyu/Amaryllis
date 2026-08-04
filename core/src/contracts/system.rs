use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub time_ms: u128,
}

#[derive(Debug, Serialize)]
pub struct SettingsResponse {
    pub timezone: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    pub timezone: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SystemConfigResponse {
    pub btc_eth_leverage: u32,
    pub altcoin_leverage: u32,
    pub runtime_alert_webhook_enabled: bool,
    pub runtime_alert_webhook_auth_header_set: bool,
    pub runtime_alert_webhook_timeout_secs: u64,
    pub runtime_alert_webhook_max_retries: u64,
    pub runtime_alert_webhook_retry_backoff_ms: u64,
    pub runtime_alert_webhook_signing_enabled: bool,
    pub runtime_alert_webhook_signing_header_set: bool,
    pub runtime_alert_webhook_signing_timestamp_header_set: bool,
    pub runtime_alert_webhook_signing_max_age_secs: u64,
}
