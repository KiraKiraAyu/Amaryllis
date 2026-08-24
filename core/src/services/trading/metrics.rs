use super::service::*;

pub async fn runtime_events(
    app: &SharedState,
    trader_id: Option<String>,
    window_hours: Option<i64>,
    limit: Option<i64>,
    offset: Option<i64>,
    event_type: Option<String>,
    risk_level: Option<String>,
    correlation_id: Option<String>,
) -> AppResult<RuntimeEventsPayload> {
    let trader_id = match resolve_trader_id(app, trader_id).await {
        Ok(v) => v,
        Err(e) => return Err(e),
    };

    let window_hours = window_hours.unwrap_or(24).clamp(1, 24 * 365);
    let from_ts = now_ts() - window_hours * 3600;
    let limit = limit.unwrap_or(100).clamp(1, 1000);
    let offset = offset.unwrap_or(0).max(0);
    let event_type = event_type.unwrap_or_default().trim().to_string();
    let risk_level = risk_level.unwrap_or_default().trim().to_ascii_lowercase();
    let correlation_id = correlation_id.unwrap_or_default().trim().to_string();

    match app
        .trading_repo
        .runtime_events(
            &trader_id,
            from_ts,
            &event_type,
            &risk_level,
            &correlation_id,
            limit,
            offset,
        )
        .await
    {
        Ok((total_count, items)) => {
            let items: Vec<RuntimeEventPayload> =
                items.into_iter().map(runtime_event_payload).collect();
            Ok(RuntimeEventsPayload {
                trader_id,
                window_hours,
                from_ts,
                limit,
                offset,
                filters: RuntimeEventsFilterPayload {
                    event_type,
                    risk_level,
                    correlation_id,
                },
                total: total_count,
                items,
            })
        }
        Err(_) => Err(app_error(
            AppErrorKind::Internal,
            "Failed to load runtime events",
        )),
    }
}

pub async fn runtime_event_types(
    app: &SharedState,
    trader_id: Option<String>,
    window_hours: Option<i64>,
) -> AppResult<RuntimeEventTypesPayload> {
    let trader_id = match resolve_trader_id(app, trader_id).await {
        Ok(v) => v,
        Err(e) => return Err(e),
    };

    let window_hours = window_hours.unwrap_or(24).clamp(1, 24 * 365);
    let from_ts = now_ts() - window_hours * 3600;

    match app
        .trading_repo
        .runtime_events_since(&trader_id, from_ts)
        .await
    {
        Ok(items) => {
            let mut counts_by_type: HashMap<String, i64> = HashMap::new();
            for row in items {
                *counts_by_type.entry(row.event_type).or_insert(0) += 1;
            }

            let mut items: Vec<RuntimeEventTypePayload> = canonical_runtime_event_types()
                .iter()
                .map(|x| {
                    let count = counts_by_type.remove(x.event_type).unwrap_or(0);
                    RuntimeEventTypePayload {
                        event_type: x.event_type.to_string(),
                        count,
                        description: x.description.to_string(),
                        canonical: true,
                    }
                })
                .collect();

            let mut extra_items: Vec<RuntimeEventTypePayload> = counts_by_type
                .into_iter()
                .filter_map(|(event_type, count)| {
                    if event_type.trim().is_empty() {
                        None
                    } else {
                        Some(RuntimeEventTypePayload {
                            event_type,
                            count,
                            description: "non-canonical runtime event type".to_string(),
                            canonical: false,
                        })
                    }
                })
                .collect();

            extra_items.sort_by(|a, b| {
                b.count
                    .cmp(&a.count)
                    .then_with(|| a.event_type.cmp(&b.event_type))
            });

            items.extend(extra_items);

            Ok(RuntimeEventTypesPayload {
                trader_id,
                window_hours,
                from_ts,
                items,
            })
        }
        Err(_) => Err(app_error(
            AppErrorKind::Internal,
            "Failed to load runtime event types",
        )),
    }
}

pub async fn runtime_metrics(
    app: &SharedState,
    trader_id: Option<String>,
    window_hours: Option<i64>,
) -> AppResult<RuntimeMetricsPayload> {
    let trader_id = match resolve_trader_id(app, trader_id).await {
        Ok(v) => v,
        Err(e) => return Err(e),
    };

    let window_hours = window_hours.unwrap_or(24).clamp(1, 24 * 365);
    let from_ts = now_ts() - window_hours * 3600;

    let total_runtime_events = count_runtime_events(app, &trader_id, None, None, from_ts).await;
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
    let replace_market_fallback = count_runtime_events(
        app,
        &trader_id,
        Some(EVENT_CANCEL_REPLACE_USED_MARKET_FALLBACK),
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
    let medium_risk_open_skips = count_runtime_events(
        app,
        &trader_id,
        Some(EVENT_LIVE_OPEN_SKIPPED_MEDIUM_RISK),
        None,
        from_ts,
    )
    .await;
    let live_risk_snapshots = count_runtime_events(
        app,
        &trader_id,
        Some(EVENT_LIVE_RISK_SNAPSHOT),
        None,
        from_ts,
    )
    .await;

    let replace_attempted = replace_succeeded + replace_throttled;
    let stale_reconcile_total = stale_reconcile_terminal + stale_reconcile_pending;
    let pct = |part: i64, total: i64| -> f64 {
        if total > 0 {
            (part as f64 * 100.0) / total as f64
        } else {
            0.0
        }
    };

    let mut risk_counts: HashMap<String, i64> = HashMap::new();
    for row in app
        .trading_repo
        .runtime_events_since(&trader_id, from_ts)
        .await
        .unwrap_or_default()
    {
        let key = if row.risk_level.trim().is_empty() {
            "unknown".to_string()
        } else {
            row.risk_level
        };
        *risk_counts.entry(key).or_insert(0) += 1;
    }
    let mut risk_level_distribution: Vec<RiskLevelCountPayload> = risk_counts
        .into_iter()
        .map(|(risk_level, count)| RiskLevelCountPayload { risk_level, count })
        .collect();
    risk_level_distribution.sort_by_key(|b| std::cmp::Reverse(b.count));

    Ok(RuntimeMetricsPayload {
        trader_id,
        window_hours,
        from_ts,
        totals: runtime_metric_totals_payload(
            total_runtime_events,
            replace_succeeded,
            replace_throttled,
            replace_market_fallback,
            open_market_fallback,
            open_submitted,
            stale_reconcile_terminal,
            stale_reconcile_pending,
            medium_risk_open_skips,
            live_risk_snapshots,
        ),
        rates_pct: runtime_metric_rates_payload(
            pct(replace_throttled, replace_attempted),
            pct(replace_market_fallback, replace_succeeded),
            pct(open_market_fallback, open_submitted),
            pct(stale_reconcile_terminal, stale_reconcile_total),
        ),
        risk_level_distribution,
    })
}

pub async fn runtime_metrics_series(
    app: &SharedState,
    trader_id: Option<String>,
    window_hours: Option<i64>,
    bucket_minutes: Option<i64>,
) -> AppResult<RuntimeMetricsSeriesPayload> {
    let trader_id = match resolve_trader_id(app, trader_id).await {
        Ok(v) => v,
        Err(e) => return Err(e),
    };

    let window_hours = window_hours.unwrap_or(24).clamp(1, 24 * 365);
    let from_ts = now_ts() - window_hours * 3600;
    let bucket_minutes = bucket_minutes.unwrap_or(60).clamp(1, 24 * 60);
    let bucket_secs = bucket_minutes * 60;
    let pct = |part: i64, total: i64| -> f64 {
        if total > 0 {
            (part as f64 * 100.0) / total as f64
        } else {
            0.0
        }
    };

    match app
        .trading_repo
        .runtime_events_since(&trader_id, from_ts)
        .await
    {
        Ok(items) => {
            #[derive(Default)]
            struct BucketStats {
                runtime_events: i64,
                replace_succeeded: i64,
                replace_throttled: i64,
                replace_market_fallback: i64,
                open_market_fallback: i64,
                open_submitted: i64,
                stale_reconcile_terminal: i64,
                stale_reconcile_pending: i64,
                medium_risk_open_skips: i64,
                live_risk_snapshots: i64,
            }

            let mut buckets: HashMap<i64, BucketStats> = HashMap::new();
            for row in items {
                let bucket_ts = (row.created_at / bucket_secs) * bucket_secs;
                let stats = buckets.entry(bucket_ts).or_default();
                stats.runtime_events += 1;
                match row.event_type.as_str() {
                    EVENT_CANCEL_REPLACE_SUCCEEDED => stats.replace_succeeded += 1,
                    EVENT_CANCEL_REPLACE_THROTTLED => stats.replace_throttled += 1,
                    EVENT_CANCEL_REPLACE_USED_MARKET_FALLBACK => stats.replace_market_fallback += 1,
                    EVENT_LIVE_OPEN_USED_MARKET_FALLBACK => stats.open_market_fallback += 1,
                    EVENT_STALE_INTENT_RECONCILE_TERMINAL => stats.stale_reconcile_terminal += 1,
                    EVENT_STALE_INTENT_RECONCILE_PENDING => stats.stale_reconcile_pending += 1,
                    EVENT_LIVE_OPEN_SKIPPED_MEDIUM_RISK => stats.medium_risk_open_skips += 1,
                    EVENT_LIVE_RISK_SNAPSHOT => stats.live_risk_snapshots += 1,
                    EVENT_LIVE_ORDER_SUBMITTED if row.action_taken == "submit-open" => {
                        stats.open_submitted += 1
                    }
                    _ => {}
                }
            }

            let mut bucket_pairs: Vec<(i64, BucketStats)> = buckets.into_iter().collect();
            bucket_pairs.sort_by_key(|(bucket_ts, _)| *bucket_ts);

            let items: Vec<RuntimeMetricsSeriesBucketPayload> = bucket_pairs
                .into_iter()
                .map(|(bucket_ts, stats)| {
                    let replace_attempted = stats.replace_succeeded + stats.replace_throttled;
                    let stale_reconcile_total =
                        stats.stale_reconcile_terminal + stats.stale_reconcile_pending;
                    RuntimeMetricsSeriesBucketPayload {
                        bucket_from_ts: bucket_ts,
                        bucket_to_ts: bucket_ts + bucket_secs,
                        totals: runtime_metric_totals_payload(
                            stats.runtime_events,
                            stats.replace_succeeded,
                            stats.replace_throttled,
                            stats.replace_market_fallback,
                            stats.open_market_fallback,
                            stats.open_submitted,
                            stats.stale_reconcile_terminal,
                            stats.stale_reconcile_pending,
                            stats.medium_risk_open_skips,
                            stats.live_risk_snapshots,
                        ),
                        rates_pct: runtime_metric_rates_payload(
                            pct(stats.replace_throttled, replace_attempted),
                            pct(stats.replace_market_fallback, stats.replace_succeeded),
                            pct(stats.open_market_fallback, stats.open_submitted),
                            pct(stats.stale_reconcile_terminal, stale_reconcile_total),
                        ),
                    }
                })
                .collect();

            Ok(RuntimeMetricsSeriesPayload {
                trader_id,
                window_hours,
                from_ts,
                bucket_minutes,
                bucket_secs,
                items,
            })
        }
        Err(_) => Err(app_error(
            AppErrorKind::Internal,
            "Failed to load runtime metrics series",
        )),
    }
}

fn runtime_event_payload(row: RuntimeEventRecord) -> RuntimeEventPayload {
    let payload = serde_json::from_str::<Value>(&row.payload_json)
        .unwrap_or_else(|_| Value::Object(serde_json::Map::new()));
    RuntimeEventPayload {
        id: row.id,
        event_type: row.event_type,
        symbol: row.symbol,
        side: row.side,
        risk_level: row.risk_level,
        trigger_source: row.trigger_source,
        action_taken: row.action_taken,
        correlation_id: row.correlation_id,
        payload,
        created_at: row.created_at,
    }
}
