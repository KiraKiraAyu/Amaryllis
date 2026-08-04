use axum::{Json, extract::State};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    contracts::system::{HealthResponse, SettingsResponse, SystemConfigResponse, UpdateSettingsRequest},
    error::Result,
    http::response::ApiResponse,
    repositories::AppSettingsRepo,
    state,
};

pub async fn health() -> Result<Json<ApiResponse<HealthResponse>>> {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    let payload = HealthResponse {
        status: "ok",
        time_ms: now_ms,
    };

    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn config(
    State(shared_state): State<state::AppState>,
) -> Result<Json<ApiResponse<SystemConfigResponse>>> {
    let runtime_alerts = &shared_state.config.runtime_alerts;

    let payload = SystemConfigResponse {
        btc_eth_leverage: env_u32("BTC_ETH_LEVERAGE", 10),
        altcoin_leverage: env_u32("ALTCOIN_LEVERAGE", 5),

        runtime_alert_webhook_enabled: runtime_alerts.enabled(),
        runtime_alert_webhook_auth_header_set: runtime_alerts.auth_header_set(),
        runtime_alert_webhook_timeout_secs: runtime_alerts.timeout_secs,
        runtime_alert_webhook_max_retries: runtime_alerts.max_retries,
        runtime_alert_webhook_retry_backoff_ms: runtime_alerts.retry_backoff_ms,
        runtime_alert_webhook_signing_enabled: runtime_alerts.signing_enabled(),
        runtime_alert_webhook_signing_header_set: runtime_alerts.signing_header_set(),
        runtime_alert_webhook_signing_timestamp_header_set: runtime_alerts
            .signing_timestamp_header_set(),
        runtime_alert_webhook_signing_max_age_secs: runtime_alerts.signing_max_age_secs,
    };

    Ok(Json(ApiResponse::success(Some(payload), None)))
}

fn env_u32(key: &str, default: u32) -> u32 {
    env::var(key)
        .ok()
        .and_then(|v| v.trim().parse::<u32>().ok())
        .unwrap_or(default)
}

const TIMEZONE_KEY: &str = "timezone";

pub async fn get_settings(
    State(shared_state): State<state::AppState>,
) -> Result<Json<ApiResponse<SettingsResponse>>> {
    let repo = AppSettingsRepo::new(shared_state.db.clone());
    let timezone = repo.get(TIMEZONE_KEY).await.unwrap_or(None);

    let payload = SettingsResponse { timezone };

    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn update_settings(
    State(shared_state): State<state::AppState>,
    Json(body): Json<UpdateSettingsRequest>,
) -> Result<Json<ApiResponse<SettingsResponse>>> {
    let repo = AppSettingsRepo::new(shared_state.db.clone());

    if let Some(ref tz) = body.timezone {
        let tz = tz.trim();
        if tz.is_empty() {
            repo.delete(TIMEZONE_KEY).await?;
        } else {
            repo.set(TIMEZONE_KEY, tz).await?;
        }
    }

    let timezone = repo.get(TIMEZONE_KEY).await.unwrap_or(None);

    let payload = SettingsResponse { timezone };

    Ok(Json(ApiResponse::success(Some(payload), None)))
}
