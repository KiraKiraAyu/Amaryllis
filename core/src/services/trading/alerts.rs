use super::service::*;
use crate::clients::outbound_http::{OutboundRequestLog, body_preview, send_text};
use reqwest::Method;

pub async fn runtime_alerts(
    app: &SharedState,
    trader_id: Option<String>,
    window_hours: Option<i64>,
    open_market_fallback_rate_max_pct: Option<f64>,
    replace_throttle_rate_max_pct: Option<f64>,
    stale_reconcile_terminal_rate_max_pct: Option<f64>,
    persist_min_interval_secs: Option<i64>,
) -> AppResult<RuntimeAlertsPayload> {
    let trader_id = match resolve_trader_id(app, trader_id).await {
        Ok(v) => v,
        Err(e) => return Err(e),
    };

    let window_hours = window_hours.unwrap_or(24).clamp(1, 24 * 365);
    let now = now_ts();
    let from_ts = now - window_hours * 3600;
    let persist_min_interval_secs = persist_min_interval_secs
        .unwrap_or(300)
        .clamp(30, 24 * 3600);

    let open_market_fallback_rate_max_pct = open_market_fallback_rate_max_pct
        .unwrap_or(15.0)
        .clamp(0.0, 100.0);
    let replace_throttle_rate_max_pct = replace_throttle_rate_max_pct
        .unwrap_or(20.0)
        .clamp(0.0, 100.0);
    let stale_reconcile_terminal_rate_max_pct = stale_reconcile_terminal_rate_max_pct
        .unwrap_or(20.0)
        .clamp(0.0, 100.0);

    let replace_succeeded = count_runtime_events(
        app,
        &trader_id,
        Some(EVENT_CANCEL_REPLACE_SUCCEEDED),
        None,
        from_ts,
    )
    .await;
    let replace_throttled = count_runtime_events(
        app,
        &trader_id,
        Some(EVENT_CANCEL_REPLACE_THROTTLED),
        None,
        from_ts,
    )
    .await;
    let open_market_fallback = count_runtime_events(
        app,
        &trader_id,
        Some(EVENT_LIVE_OPEN_USED_MARKET_FALLBACK),
        None,
        from_ts,
    )
    .await;
    let open_submitted = count_runtime_events(
        app,
        &trader_id,
        Some(EVENT_LIVE_ORDER_SUBMITTED),
        Some("submit-open"),
        from_ts,
    )
    .await;
    let stale_reconcile_terminal = count_runtime_events(
        app,
        &trader_id,
        Some(EVENT_STALE_INTENT_RECONCILE_TERMINAL),
        None,
        from_ts,
    )
    .await;
    let stale_reconcile_pending = count_runtime_events(
        app,
        &trader_id,
        Some(EVENT_STALE_INTENT_RECONCILE_PENDING),
        None,
        from_ts,
    )
    .await;

    let pct = |part: i64, total: i64| -> f64 {
        if total > 0 {
            (part as f64 * 100.0) / total as f64
        } else {
            0.0
        }
    };

    let replace_attempted = replace_succeeded + replace_throttled;
    let stale_reconcile_total = stale_reconcile_terminal + stale_reconcile_pending;

    let replace_throttle_rate = pct(replace_throttled, replace_attempted);
    let open_market_fallback_rate = pct(open_market_fallback, open_submitted);
    let stale_reconcile_terminal_rate = pct(stale_reconcile_terminal, stale_reconcile_total);

    let open_market_fallback_breached =
        open_market_fallback_rate > open_market_fallback_rate_max_pct;
    let replace_throttle_breached = replace_throttle_rate > replace_throttle_rate_max_pct;
    let stale_reconcile_terminal_breached =
        stale_reconcile_terminal_rate > stale_reconcile_terminal_rate_max_pct;

    let any_breached = open_market_fallback_breached
        || replace_throttle_breached
        || stale_reconcile_terminal_breached;

    let thresholds_pct = runtime_alert_thresholds_payload(
        open_market_fallback_rate_max_pct,
        replace_throttle_rate_max_pct,
        stale_reconcile_terminal_rate_max_pct,
    );
    let rates_pct = runtime_alert_rates_payload(
        open_market_fallback_rate,
        replace_throttle_rate,
        stale_reconcile_terminal_rate,
    );
    let alerts = runtime_alert_items_payload(&thresholds_pct, &rates_pct);

    let controls_row = app
        .trading_repo
        .runtime_alert_controls(&trader_id)
        .await
        .ok()
        .flatten();

    let mut controls_record =
        controls_row.unwrap_or_else(|| empty_controls_record(trader_id.clone()));
    if controls_record.is_muted
        && controls_record.muted_until > 0
        && controls_record.muted_until <= now
    {
        let _ = app
            .trading_repo
            .unmute_expired_runtime_alerts(&trader_id, now)
            .await;
        controls_record.is_muted = false;
        controls_record.muted_until = 0;
        controls_record.mute_reason.clear();
        controls_record.updated_at = now;
    }

    let is_muted = controls_record.is_muted;
    let muted_until = controls_record.muted_until;
    let mute_reason = controls_record.mute_reason.clone();
    let acked_at = controls_record.acked_at;
    let acked_by = controls_record.acked_by.clone();
    let ack_note = controls_record.ack_note.clone();
    let controls_updated_at = controls_record.updated_at;
    let controls_created_at = controls_record.created_at;

    let notification_suppressed = is_muted;
    let severity = if any_breached { "warning" } else { "ok" };

    let recent_same_count = app
        .trading_repo
        .recent_runtime_alert_history_count(
            &trader_id,
            any_breached,
            severity,
            now - persist_min_interval_secs,
        )
        .await
        .unwrap_or(0);

    let mut alert_history_id = String::new();
    if recent_same_count == 0 && !notification_suppressed {
        let new_alert_history_id = Uuid::now_v7().to_string();
        let inserted = app
            .trading_repo
            .insert_runtime_alert_history(InsertRuntimeAlertHistoryRecord {
                id: new_alert_history_id.clone(),
                trader_id: trader_id.clone(),
                window_hours,
                thresholds_json: serde_json::to_string(&thresholds_pct)
                    .unwrap_or_else(|_| "{}".to_string()),
                rates_json: serde_json::to_string(&rates_pct).unwrap_or_else(|_| "{}".to_string()),
                alerts_json: serde_json::to_string(&alerts).unwrap_or_else(|_| "[]".to_string()),
                breached: any_breached,
                severity: severity.to_string(),
                created_at: now,
            })
            .await;

        if inserted.is_ok() {
            alert_history_id = new_alert_history_id;
        }
    }

    let webhook_payload = json!({
        "event": "runtime_alert_evaluated",
        "trader_id": trader_id.clone(),
        "window_hours": window_hours,
        "from_ts": from_ts,
        "persist_min_interval_secs": persist_min_interval_secs,
        "thresholds_pct": thresholds_pct.clone(),
        "rates_pct": rates_pct.clone(),
        "alerts": alerts.clone(),
        "alert_state": {
            "breached": any_breached,
            "severity": severity
        },
        "created_at": now
    });

    let notification = if notification_suppressed {
        RuntimeAlertNotificationPayload {
            channel: "webhook".to_string(),
            suppressed: true,
            reason: "muted".to_string(),
            attempts: 0,
            max_attempts: app.config.runtime_alerts.max_retries as i64,
            success: false,
            status: 0,
            error: String::new(),
        }
    } else {
        notify_runtime_alert_webhook_best_effort(
            app,
            &trader_id,
            &alert_history_id,
            &webhook_payload,
        )
        .await
    };

    let controls = runtime_alert_controls_payload(
        trader_id.clone(),
        notification_suppressed,
        muted_until,
        mute_reason,
        acked_at,
        acked_by,
        ack_note,
        controls_updated_at,
        controls_created_at,
    );

    Ok(RuntimeAlertsPayload {
        trader_id,
        window_hours,
        from_ts,
        persist_min_interval_secs,
        thresholds_pct,
        rates_pct,
        totals: runtime_alert_totals_payload(
            replace_succeeded,
            replace_throttled,
            open_market_fallback,
            open_submitted,
            stale_reconcile_terminal,
            stale_reconcile_pending,
        ),
        alerts,
        alert_state: RuntimeAlertStatePayload {
            breached: any_breached,
            severity: severity.to_string(),
            muted: notification_suppressed,
            acked_at,
        },
        controls,
        alert_history_id,
        notification,
    })
}

pub async fn runtime_alert_history(
    app: &SharedState,
    trader_id: Option<String>,
    window_hours: Option<i64>,
    limit: Option<i64>,
    offset: Option<i64>,
    breached_only: Option<bool>,
    severity: Option<String>,
) -> AppResult<RuntimeAlertHistoryPayload> {
    let trader_id = match resolve_trader_id(app, trader_id).await {
        Ok(v) => v,
        Err(e) => return Err(e),
    };

    let window_hours = window_hours.unwrap_or(24 * 7).clamp(1, 24 * 365);
    let from_ts = now_ts() - window_hours * 3600;
    let limit = limit.unwrap_or(100).clamp(1, 1000);
    let offset = offset.unwrap_or(0).max(0);
    let breached_filter = match breached_only {
        Some(true) => 1_i64,
        Some(false) => 0_i64,
        None => -1_i64,
    };
    let severity_filter = severity.unwrap_or_default().trim().to_ascii_lowercase();

    let breached = match breached_filter {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    };
    let result = app
        .trading_repo
        .runtime_alert_history(
            &trader_id,
            from_ts,
            breached,
            &severity_filter,
            limit,
            offset,
        )
        .await;

    match result {
        Ok((total_count, items)) => {
            let items: Vec<RuntimeAlertHistoryItemPayload> =
                items.into_iter().map(alert_history_payload).collect();

            Ok(RuntimeAlertHistoryPayload {
                trader_id,
                window_hours,
                from_ts,
                limit,
                offset,
                filters: RuntimeAlertHistoryFiltersPayload {
                    breached_only,
                    severity: severity_filter,
                },
                total: total_count,
                items,
            })
        }
        Err(_) => Err(app_error(
            AppErrorKind::Internal,
            "Failed to load runtime alert history",
        )),
    }
}

pub async fn runtime_alert_deliveries(
    app: &SharedState,
    trader_id: Option<String>,
    window_hours: Option<i64>,
    limit: Option<i64>,
    offset: Option<i64>,
    success: Option<bool>,
    destination: Option<String>,
) -> AppResult<RuntimeAlertDeliveriesPayload> {
    let trader_id = match resolve_trader_id(app, trader_id).await {
        Ok(v) => v,
        Err(e) => return Err(e),
    };

    let window_hours = window_hours.unwrap_or(24 * 7).clamp(1, 24 * 365);
    let from_ts = now_ts() - window_hours * 3600;
    let limit = limit.unwrap_or(100).clamp(1, 1000);
    let offset = offset.unwrap_or(0).max(0);
    let success_filter = match success {
        Some(true) => 1_i64,
        Some(false) => 0_i64,
        None => -1_i64,
    };
    let destination_filter = destination.unwrap_or_default().trim().to_ascii_lowercase();

    let success = match success_filter {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    };
    let result = app
        .trading_repo
        .runtime_alert_deliveries(
            &trader_id,
            from_ts,
            success,
            &destination_filter,
            limit,
            offset,
        )
        .await;

    match result {
        Ok((total_count, items)) => {
            let items: Vec<RuntimeAlertDeliveryLogPayload> =
                items.into_iter().map(delivery_payload).collect();

            Ok(RuntimeAlertDeliveriesPayload {
                trader_id,
                window_hours,
                from_ts,
                limit,
                offset,
                filters: RuntimeAlertDeliveriesFiltersPayload {
                    success,
                    destination: destination_filter,
                },
                total: total_count,
                items,
            })
        }
        Err(_) => Err(app_error(
            AppErrorKind::Internal,
            "Failed to load runtime alert deliveries",
        )),
    }
}

pub async fn runtime_alert_controls(
    app: &SharedState,
    trader_id: Option<String>,
) -> AppResult<RuntimeAlertControlsPayload> {
    let trader_id = match resolve_trader_id(app, trader_id).await {
        Ok(v) => v,
        Err(e) => return Err(e),
    };

    match app.trading_repo.runtime_alert_controls(&trader_id).await {
        Ok(Some(record)) => Ok(controls_payload(record)),
        Ok(None) => Ok(empty_runtime_alert_controls_payload(trader_id)),
        Err(_) => Err(app_error(
            AppErrorKind::Internal,
            "Failed to load runtime alert controls",
        )),
    }
}

pub async fn mute_runtime_alerts(
    app: &SharedState,
    trader_id: Option<String>,
    mute_minutes: Option<i64>,
    mute_until: Option<i64>,
    reason: Option<String>,
) -> AppResult<RuntimeAlertMutePayload> {
    let trader_id = match resolve_trader_id(app, trader_id).await {
        Ok(v) => v,
        Err(e) => return Err(e),
    };

    let now = now_ts();
    let mute_until = mute_until
        .filter(|v| *v > now)
        .unwrap_or_else(|| now + mute_minutes.unwrap_or(60).clamp(1, 24 * 365 * 24 * 60) * 60);
    let reason = reason.unwrap_or_default().trim().to_string();

    let result = app
        .trading_repo
        .set_runtime_alert_mute(&trader_id, Some(mute_until), reason.clone(), now)
        .await;

    match result {
        Ok(_) => Ok(RuntimeAlertMutePayload {
            message: "Runtime alerts muted",
            trader_id,
            is_muted: true,
            muted_until: mute_until,
            mute_reason: reason,
        }),
        Err(_) => Err(app_error(
            AppErrorKind::Internal,
            "Failed to mute runtime alerts",
        )),
    }
}

pub async fn unmute_runtime_alerts(
    app: &SharedState,
    trader_id: Option<String>,
) -> AppResult<RuntimeAlertMutePayload> {
    let trader_id = match resolve_trader_id(app, trader_id).await {
        Ok(v) => v,
        Err(e) => return Err(e),
    };

    let now = now_ts();
    let result = app
        .trading_repo
        .set_runtime_alert_mute(&trader_id, None, String::new(), now)
        .await;

    match result {
        Ok(_) => Ok(RuntimeAlertMutePayload {
            message: "Runtime alerts unmuted",
            trader_id,
            is_muted: false,
            muted_until: 0,
            mute_reason: String::new(),
        }),
        Err(_) => Err(app_error(
            AppErrorKind::Internal,
            "Failed to unmute runtime alerts",
        )),
    }
}

pub async fn ack_runtime_alerts(
    app: &SharedState,
    trader_id: Option<String>,
    note: Option<String>,
) -> AppResult<RuntimeAlertAckPayload> {
    let trader_id = match resolve_trader_id(app, trader_id).await {
        Ok(v) => v,
        Err(e) => return Err(e),
    };

    let now = now_ts();
    let note = note.unwrap_or_default().trim().to_string();

    let result = app
        .trading_repo
        .ack_runtime_alerts(&trader_id, note.clone(), now)
        .await;

    match result {
        Ok(_) => Ok(RuntimeAlertAckPayload {
            message: "Runtime alerts acknowledged",
            trader_id,
            acked_at: now,
            acked_by: String::new(),
            ack_note: note,
        }),
        Err(_) => Err(app_error(
            AppErrorKind::Internal,
            "Failed to acknowledge runtime alerts",
        )),
    }
}

pub async fn notify_runtime_alert_webhook_best_effort(
    app: &SharedState,
    trader_id: &str,
    alert_history_id: &str,
    payload: &Value,
) -> RuntimeAlertNotificationPayload {
    let runtime_alerts = &app.config.runtime_alerts;
    let webhook_url = runtime_alerts.url.trim().to_string();
    if webhook_url.is_empty() {
        return RuntimeAlertNotificationPayload {
            channel: "webhook".to_string(),
            suppressed: true,
            reason: "webhook_not_configured".to_string(),
            attempts: 0,
            max_attempts: 0,
            success: false,
            status: 0,
            error: String::new(),
        };
    }

    let timeout_secs = runtime_alerts.timeout_secs.max(1);
    let max_retries = runtime_alerts.max_retries.max(1);
    let base_backoff_ms = runtime_alerts.retry_backoff_ms.max(1);
    let auth_header = runtime_alerts.auth_header.trim().to_string();
    let signing_secret = runtime_alerts.signing_secret.trim().to_string();
    let signing_header = runtime_alerts.signing_header.trim().to_string();
    let signing_timestamp_header = runtime_alerts.signing_timestamp_header.trim().to_string();

    let payload_bytes = match serde_json::to_vec(payload) {
        Ok(v) => v,
        Err(err) => {
            warn!("runtime alert webhook payload encode failed: {}", err);
            return RuntimeAlertNotificationPayload {
                channel: "webhook".to_string(),
                suppressed: false,
                reason: "payload_encode_failed".to_string(),
                attempts: 0,
                max_attempts: max_retries as i64,
                success: false,
                status: 0,
                error: err.to_string(),
            };
        }
    };

    let payload_text =
        String::from_utf8(payload_bytes.clone()).unwrap_or_else(|_| "{}".to_string());
    let signed_timestamp = now_ts().to_string();
    let signature = if signing_secret.is_empty() {
        None
    } else {
        match HmacSha256::new_from_slice(signing_secret.as_bytes()) {
            Ok(mut mac) => {
                let mut to_sign = signed_timestamp.as_bytes().to_vec();
                to_sign.push(b'.');
                to_sign.extend_from_slice(&payload_bytes);
                mac.update(&to_sign);
                Some(hex::encode(mac.finalize().into_bytes()))
            }
            Err(err) => {
                warn!("runtime alert webhook signature build failed: {}", err);
                return RuntimeAlertNotificationPayload {
                    channel: "webhook".to_string(),
                    suppressed: false,
                    reason: "signature_build_failed".to_string(),
                    attempts: 0,
                    max_attempts: max_retries as i64,
                    success: false,
                    status: 0,
                    error: err.to_string(),
                };
            }
        }
    };

    let request_headers_json = json!({
        "authorization_set": !auth_header.is_empty(),
        "signing_header": if signing_header.is_empty() { "" } else { &signing_header },
        "signing_timestamp_header": if signing_timestamp_header.is_empty() { "" } else { &signing_timestamp_header }
    });

    let client = match Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
    {
        Ok(c) => c,
        Err(err) => {
            warn!("runtime alert webhook client build failed: {}", err);
            return RuntimeAlertNotificationPayload {
                channel: "webhook".to_string(),
                suppressed: false,
                reason: "client_build_failed".to_string(),
                attempts: 0,
                max_attempts: max_retries as i64,
                success: false,
                status: 0,
                error: err.to_string(),
            };
        }
    };

    let mut final_success = false;
    let mut final_status = 0_i64;
    let mut final_error = String::new();
    let mut performed_attempts = 0_i64;

    for attempt in 1..=max_retries {
        performed_attempts = attempt as i64;
        let attempt_started = std::time::Instant::now();

        let mut req = client
            .post(&webhook_url)
            .header("Content-Type", "application/json")
            .body(payload_bytes.clone());

        if !auth_header.is_empty() {
            req = req.header("Authorization", auth_header.clone());
        }

        if let Some(sig) = &signature {
            if !signing_header.is_empty() {
                req = req.header(signing_header.clone(), sig.clone());
            }
            if !signing_timestamp_header.is_empty() {
                req = req.header(signing_timestamp_header.clone(), signed_timestamp.clone());
            }
        }

        let send_result = send_text(
            req,
            OutboundRequestLog::new("runtime_alert.webhook", Method::POST, &webhook_url)
                .body(body_preview(&payload_bytes)),
        )
        .await;
        let latency_ms = attempt_started.elapsed().as_millis() as i64;

        let (success, status_code, response_body, error_message) = match send_result {
            Ok(resp) => {
                let status = i64::from(resp.status.as_u16());
                let ok = resp.status.is_success();
                let body = resp.body;
                if !ok {
                    warn!(
                        "runtime alert webhook non-success status={} attempt={}/{}",
                        status, attempt, max_retries
                    );
                }
                (ok, status, body, String::new())
            }
            Err(err) => {
                warn!(
                    "runtime alert webhook send failed attempt={}/{} err={}",
                    attempt, max_retries, err
                );
                (false, 0_i64, String::new(), err.to_string())
            }
        };

        let _ = app
            .trading_repo
            .insert_runtime_alert_delivery(InsertRuntimeAlertDeliveryRecord {
                id: Uuid::now_v7().to_string(),
                trader_id: trader_id.to_string(),
                alert_history_id: alert_history_id.to_string(),
                destination: "webhook".to_string(),
                endpoint: webhook_url.clone(),
                request_headers_json: serde_json::to_string(&request_headers_json)
                    .unwrap_or_else(|_| "{}".to_string()),
                request_body_json: payload_text.clone(),
                response_status: status_code,
                response_body: response_body.clone(),
                attempt: attempt as i64,
                max_attempts: max_retries as i64,
                success,
                error_message: error_message.clone(),
                latency_ms: latency_ms as i64,
                created_at: now_ts(),
            })
            .await;

        if success {
            final_success = true;
            final_status = status_code;
            final_error.clear();
            break;
        } else {
            final_success = false;
            final_status = status_code;
            final_error = error_message;
        }

        if attempt < max_retries {
            let exp = ((attempt - 1) as u32).min(16);
            let delay_ms = base_backoff_ms.saturating_mul(1_u64 << exp).min(60_000);
            sleep(Duration::from_millis(delay_ms)).await;
        }
    }

    RuntimeAlertNotificationPayload {
        channel: "webhook".to_string(),
        suppressed: false,
        reason: String::new(),
        attempts: performed_attempts,
        max_attempts: max_retries as i64,
        success: final_success,
        status: final_status,
        error: final_error,
    }
}

fn empty_controls_record(trader_id: String) -> RuntimeAlertControlsRecord {
    RuntimeAlertControlsRecord {
        trader_id,
        is_muted: false,
        muted_until: 0,
        mute_reason: String::new(),
        acked_at: 0,
        acked_by: String::new(),
        ack_note: String::new(),
        updated_at: 0,
        created_at: 0,
    }
}

fn controls_payload(record: RuntimeAlertControlsRecord) -> RuntimeAlertControlsPayload {
    runtime_alert_controls_payload(
        record.trader_id,
        record.is_muted,
        record.muted_until,
        record.mute_reason,
        record.acked_at,
        record.acked_by,
        record.ack_note,
        record.updated_at,
        record.created_at,
    )
}

fn alert_history_payload(record: RuntimeAlertHistoryRecord) -> RuntimeAlertHistoryItemPayload {
    let thresholds_pct =
        serde_json::from_str::<RuntimeAlertThresholdsPayload>(&record.thresholds_json)
            .unwrap_or_else(|_| runtime_alert_thresholds_payload(0.0, 0.0, 0.0));
    let rates_pct = serde_json::from_str::<RuntimeAlertRatesPayload>(&record.rates_json)
        .unwrap_or_else(|_| runtime_alert_rates_payload(0.0, 0.0, 0.0));
    let alerts = serde_json::from_str::<Vec<RuntimeAlertItemPayload>>(&record.alerts_json)
        .unwrap_or_default();

    RuntimeAlertHistoryItemPayload {
        id: record.id,
        window_hours: record.window_hours,
        thresholds_pct,
        rates_pct,
        alerts,
        breached: record.breached,
        severity: record.severity,
        created_at: record.created_at,
    }
}

fn delivery_payload(record: RuntimeAlertDeliveryRecord) -> RuntimeAlertDeliveryLogPayload {
    RuntimeAlertDeliveryLogPayload {
        id: record.id,
        alert_history_id: record.alert_history_id,
        destination: record.destination,
        endpoint: record.endpoint,
        response_status: record.response_status,
        response_body: record.response_body,
        attempt: record.attempt,
        max_attempts: record.max_attempts,
        success: record.success,
        error_message: record.error_message,
        latency_ms: record.latency_ms,
        created_at: record.created_at,
    }
}
