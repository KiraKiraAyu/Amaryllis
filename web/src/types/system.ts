export interface HealthResponse {
  status: string
  time_ms: number
}

export interface SystemConfigResponse {
  btc_eth_leverage: number
  altcoin_leverage: number
  runtime_alert_webhook_enabled: boolean
  runtime_alert_webhook_auth_header_set: boolean
  runtime_alert_webhook_timeout_secs: number
  runtime_alert_webhook_max_retries: number
  runtime_alert_webhook_retry_backoff_ms: number
  runtime_alert_webhook_signing_enabled: boolean
  runtime_alert_webhook_signing_header_set: boolean
  runtime_alert_webhook_signing_timestamp_header_set: boolean
  runtime_alert_webhook_signing_max_age_secs: number
}
