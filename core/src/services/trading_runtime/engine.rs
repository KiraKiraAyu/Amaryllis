use super::service::*;

pub async fn run_trader_loop(
    engine: TradingRuntimeService,
    cfg: TraderRuntimeConfig,
    mut stop_rx: watch::Receiver<bool>,
) -> Result<(), AppError> {
    let symbols = parse_symbols(&cfg.trading_symbols);
    if symbols.is_empty() {
        return Err(AppError::InvalidConfig(
            "trading_symbols is empty after parsing".to_string(),
        ));
    }

    let mut market = seed_market(&cfg, &symbols).await?;
    let mut interval = time::interval(Duration::from_secs(
        (cfg.scan_interval_minutes.max(1) as u64) * 60,
    ));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let (exec_ctx, live_adapter) =
        load_runtime_execution_context(&engine.inner.state, &cfg).await?;
    let live_mode = match exec_ctx.mode {
        RuntimeExecutionMode::Simulated => "simulated",
        RuntimeExecutionMode::LiveExchange => live_adapter
            .as_deref()
            .map(|adapter| adapter.exchange_type())
            .unwrap_or("live-exchange"),
    };
    info!(
        "runtime execution mode initialized trader={} mode={}",
        cfg.trader_id, live_mode
    );

    let mut consecutive_live_failures: u32 = 0;
    let live_circuit_breaker_limit: u32 = 5;

    let mut user_stream_rx: Option<mpsc::Receiver<ExchangeUserStreamEvent>> = None;
    let mut user_stream_session: Option<ExchangeUserStreamSession> = None;
    let mut user_stream_keepalive = time::interval(Duration::from_secs(30 * 60));
    user_stream_keepalive.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let user_stream_reconnect_backoff = Duration::from_secs(2);

    if exec_ctx.mode == RuntimeExecutionMode::LiveExchange {
        if let Some(adapter) = live_adapter.as_deref() {
            match init_exchange_user_stream(adapter).await {
                Ok(session) => {
                    let reader_rx =
                        spawn_exchange_user_stream_reader(session.clone(), stop_rx.clone());
                    user_stream_rx = Some(reader_rx);
                    user_stream_session = Some(session);
                    info!(
                        "exchange user stream initialized trader={} exchange={}",
                        cfg.trader_id,
                        adapter.exchange_type()
                    );
                }
                Err(err) => {
                    warn!(
                        "exchange user stream init skipped trader={} exchange={} err={}",
                        cfg.trader_id,
                        adapter.exchange_type(),
                        err
                    );
                    user_stream_rx = None;
                    user_stream_session = None;
                }
            }
        }
    }

    // immediate first cycle
    process_cycle(
        &engine.inner.state,
        &cfg,
        &symbols,
        &mut market,
        &exec_ctx,
        live_adapter.as_deref(),
    )
    .await?;

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let result = process_cycle(
                    &engine.inner.state,
                    &cfg,
                    &symbols,
                    &mut market,
                    &exec_ctx,
                    live_adapter.as_deref(),
                )
                .await;
                match result {
                    Ok(_) => {
                        if consecutive_live_failures > 0 {
                            info!(
                                "live cycle recovered trader={} failures_before_recover={}",
                                cfg.trader_id, consecutive_live_failures
                            );
                        }
                        consecutive_live_failures = 0;
                        let _ = engine.inner.state.set_runtime_engine_running(&cfg.trader_id, true, None);
                    }
                    Err(err) => {
                        if exec_ctx.mode == RuntimeExecutionMode::LiveExchange {
                            consecutive_live_failures = consecutive_live_failures.saturating_add(1);
                            let failure_msg = format!(
                                "live exchange failure {}/{}: {}",
                                consecutive_live_failures, live_circuit_breaker_limit, err
                            );
                            let _ = engine.inner.state.set_runtime_engine_running(
                                &cfg.trader_id,
                                true,
                                Some(failure_msg.clone()),
                            );
                            error!(
                                "cycle failed trader={} live_failure_count={} err={}",
                                cfg.trader_id, consecutive_live_failures, err
                            );

                            if consecutive_live_failures >= live_circuit_breaker_limit {
                                let breaker_msg = format!(
                                    "live circuit breaker opened after {} consecutive failures",
                                    consecutive_live_failures
                                );
                                let _ = engine.inner.state.set_runtime_engine_running(
                                    &cfg.trader_id,
                                    false,
                                    Some(breaker_msg.clone()),
                                );
                                warn!(
                                    "stopping live loop by circuit breaker trader={} reason={}",
                                    cfg.trader_id, breaker_msg
                                );
                                break;
                            }
                        } else {
                            let _ = engine.inner.state.set_runtime_engine_running(
                                &cfg.trader_id,
                                true,
                                Some(err.to_string()),
                            );
                            error!("cycle failed trader={} err={}", cfg.trader_id, err);
                        }
                    }
                }
            }
            _ = user_stream_keepalive.tick() => {
                if exec_ctx.mode == RuntimeExecutionMode::LiveExchange {
                    if let (Some(adapter), Some(session)) = (live_adapter.as_deref(), user_stream_session.as_ref()) {
                        if let Err(err) = adapter.keepalive_user_stream_session(session).await {
                            warn!(
                                "exchange user stream keepalive failed trader={} err={}",
                                cfg.trader_id, err
                            );

                            if let Some(old_session) = user_stream_session.take() {
                                let _ = adapter.close_user_stream_session(&old_session).await;
                            }

                            time::sleep(user_stream_reconnect_backoff).await;
                            match init_exchange_user_stream(adapter).await {
                                Ok(session) => {
                                    user_stream_rx = Some(spawn_exchange_user_stream_reader(session.clone(), stop_rx.clone()));
                                    user_stream_session = Some(session);
                                    info!("exchange user stream reconnected after keepalive failure trader={}", cfg.trader_id);
                                }
                                Err(reconnect_err) => {
                                    warn!(
                                        "exchange user stream reconnect failed after keepalive error trader={} err={}",
                                        cfg.trader_id, reconnect_err
                                    );
                                    user_stream_rx = None;
                                    user_stream_session = None;
                                }
                            }
                        }
                    }
                }
            }
            event = recv_user_stream_event(&mut user_stream_rx) => {
                if let Some(event) = event {
                    let should_reconnect = matches!(event, ExchangeUserStreamEvent::ListenKeyExpired { .. });

                    let now = now_i64();
                    if let Err(err) = handle_exchange_user_stream_event(&engine.inner.state, &cfg, event, now).await {
                        warn!(
                            "exchange user stream event handling failed trader={} err={}",
                            cfg.trader_id, err
                        );
                    }

                    if should_reconnect && exec_ctx.mode == RuntimeExecutionMode::LiveExchange {
                        if let Some(adapter) = live_adapter.as_deref() {
                            if let Some(old_session) = user_stream_session.take() {
                                let _ = adapter.close_user_stream_session(&old_session).await;
                            }

                            time::sleep(user_stream_reconnect_backoff).await;
                            match init_exchange_user_stream(adapter).await {
                                Ok(session) => {
                                    user_stream_rx = Some(spawn_exchange_user_stream_reader(session.clone(), stop_rx.clone()));
                                    user_stream_session = Some(session);
                                    info!("exchange user stream reconnected after listen key expiration trader={}", cfg.trader_id);
                                }
                                Err(reconnect_err) => {
                                    warn!(
                                        "exchange user stream reconnect failed after listen key expiration trader={} err={}",
                                        cfg.trader_id, reconnect_err
                                    );
                                    user_stream_rx = None;
                                    user_stream_session = None;
                                }
                            }
                        }
                    }
                } else if exec_ctx.mode == RuntimeExecutionMode::LiveExchange {
                    if let Some(adapter) = live_adapter.as_deref() {
                        warn!(
                            "exchange user stream disconnected trader={}, attempting reconnect",
                            cfg.trader_id
                        );

                        if let Some(old_session) = user_stream_session.take() {
                            let _ = adapter.close_user_stream_session(&old_session).await;
                        }

                        time::sleep(user_stream_reconnect_backoff).await;
                        match init_exchange_user_stream(adapter).await {
                            Ok(session) => {
                                user_stream_rx = Some(spawn_exchange_user_stream_reader(session.clone(), stop_rx.clone()));
                                user_stream_session = Some(session);
                                info!("exchange user stream reconnected after disconnect trader={}", cfg.trader_id);
                            }
                            Err(reconnect_err) => {
                                warn!(
                                    "exchange user stream reconnect failed after disconnect trader={} err={}",
                                    cfg.trader_id, reconnect_err
                                );
                                user_stream_rx = None;
                                user_stream_session = None;
                            }
                        }
                    }
                }
            }
            changed = stop_rx.changed() => {
                match changed {
                    Ok(_) => {
                        if *stop_rx.borrow() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        }
    }

    if let (Some(adapter), Some(session)) = (live_adapter.as_deref(), user_stream_session.as_ref())
    {
        if let Err(err) = adapter.close_user_stream_session(session).await {
            warn!(
                "exchange user stream close failed trader={} err={}",
                cfg.trader_id, err
            );
        }
    }

    set_trader_running(&engine.inner.state, &cfg.trader_id, false).await?;
    let _ = engine
        .inner
        .state
        .set_runtime_engine_running(&cfg.trader_id, false, None);

    info!("runtime engine loop exited trader={}", cfg.trader_id);
    Ok(())
}

async fn init_exchange_user_stream(
    adapter: &dyn LiveExchangeAdapter,
) -> Result<ExchangeUserStreamSession, AppError> {
    adapter.user_stream_session().await
}

pub async fn process_cycle(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    symbols: &[String],
    market: &mut HashMap<String, MarketState>,
    exec_ctx: &RuntimeExecutionContext,
    live_adapter: Option<&dyn LiveExchangeAdapter>,
) -> Result<(), AppError> {
    let now = now_i64();

    // 1) advance synthetic market baseline
    advance_market(cfg, now as u64, symbols, market);

    // 2) if live mode, pre-sync account/positions and overlay prices from exchange
    if exec_ctx.mode == RuntimeExecutionMode::LiveExchange {
        if let Some(adapter) = live_adapter {
            sync_live_positions_and_balances(state, cfg, adapter, now).await?;
            refresh_market_from_exchange(symbols, market, adapter).await;
        } else {
            warn!(
                "runtime marked live but adapter missing, fallback to simulated path trader={}",
                cfg.trader_id
            );
        }
    }

    // 3) mark-to-market open positions
    let mut open_positions = load_open_positions(state, &cfg.trader_id).await?;
    mark_to_market_positions(state, cfg, &mut open_positions, market, now).await?;

    // 4) account metrics
    let metrics = compute_account_metrics(state, cfg).await?;

    // 5) risk guard
    let drawdown_pct = if cfg.initial_balance > 0.0 {
        ((cfg.initial_balance - metrics.total_balance) / cfg.initial_balance) * 100.0
    } else {
        0.0
    };

    let hard_risk_trigger = drawdown_pct >= 35.0 || metrics.margin_used_ratio > 0.9;

    // 6) generate decisions
    let live_risk_decision = evaluate_live_risk(&state.config, cfg, &metrics, hard_risk_trigger);
    let live_risk_level = live_risk_decision.level.as_str();
    let cycle_correlation_id = format!(
        "cycle:{}:{}:{}",
        cfg.trader_id,
        now,
        Uuid::now_v7().simple()
    );

    let mut decisions = Vec::with_capacity(symbols.len());
    for sym in symbols {
        let trigger_source = if hard_risk_trigger {
            "hard_risk_guard"
        } else {
            "ai_model"
        };

        let signal = generate_ai_decision(
            state,
            cfg,
            sym,
            market,
            hard_risk_trigger,
            live_risk_level,
            trigger_source,
            &cycle_correlation_id,
            &metrics,
            now,
        )
        .await;

        persist_decision(state, cfg, &signal, &metrics, now).await?;
        decisions.push(signal);
    }

    // 7) execute decisions (live / simulated)
    match (exec_ctx.mode, live_adapter) {
        (RuntimeExecutionMode::LiveExchange, Some(adapter)) => {
            execute_decisions_live(
                state,
                cfg,
                &decisions,
                &open_positions,
                &metrics,
                market,
                adapter,
                now,
                hard_risk_trigger,
                &live_risk_decision,
                &cycle_correlation_id,
            )
            .await?;
        }
        _ => {
            if hard_risk_trigger {
                close_worst_positions(state, cfg, &open_positions, market, now).await?;
            } else {
                execute_decisions(
                    state,
                    cfg,
                    &decisions,
                    &open_positions,
                    &metrics,
                    market,
                    now,
                )
                .await?;
            }
        }
    }

    // 8) refresh account snapshot after execution
    let refreshed = compute_account_metrics(state, cfg).await?;
    insert_account_snapshot(state, cfg, &refreshed, now).await?;

    // Push equity snapshot to realtime clients
    state
        .realtime_hub
        .publish(crate::realtime::RealtimeEvent::EquitySnapshot {
            trader_id: cfg.trader_id.clone(),
            equity: refreshed.total_balance,
            available_cash: refreshed.available_balance,
            unrealized_pnl: refreshed.unrealized_pnl,
            ts: now,
        });

    // Push each AI decision to realtime clients
    for signal in &decisions {
        state
            .realtime_hub
            .publish(crate::realtime::RealtimeEvent::AiDecision {
                trader_id: cfg.trader_id.clone(),
                decision: json!({
                    "symbol": signal.symbol,
                    "action": signal.action,
                    "confidence": signal.confidence,
                    "reason": signal.reason,
                    "timeframe": signal.timeframe,
                    "risk_level": signal.risk_level,
                }),
            });
    }

    // heartbeat
    state
        .trading_repo
        .set_trader_running(&cfg.trader_id, true, now)
        .await?;

    Ok(())
}
