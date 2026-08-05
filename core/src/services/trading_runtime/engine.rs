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
    // Consume the first tick (which completes immediately) so the next
    // tick in the loop waits for the full scan interval.
    interval.tick().await;

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
    if let Err(err) = process_cycle(
        &engine.inner.state,
        &cfg,
        &symbols,
        &mut market,
        &exec_ctx,
        live_adapter.as_deref(),
    )
    .await
    {
        if matches!(err, AppError::BudgetExhausted(_)) {
            let _ = engine.inner.state.set_runtime_engine_running(
                &cfg.trader_id,
                false,
                Some(format!("budget circuit breaker: {}", err)),
            );
        }
        return Err(err);
    }

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
                        // Budget circuit breaker — stop immediately regardless of mode
                        if matches!(err, AppError::BudgetExhausted(_)) {
                            let breaker_msg = format!("budget circuit breaker: {}", err);
                            let _ = engine.inner.state.set_runtime_engine_running(
                                &cfg.trader_id,
                                false,
                                Some(breaker_msg.clone()),
                            );
                            warn!(
                                "stopping loop by budget circuit breaker trader={} reason={}",
                                cfg.trader_id, breaker_msg
                            );
                            break;
                        }

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

    // 4.5) budget circuit breaker — if unrealized loss exceeds the budget, close all positions and stop
    if cfg.initial_balance > 0.0 && metrics.unrealized_pnl <= -cfg.initial_balance {
        let budget = cfg.initial_balance;
        let upnl = metrics.unrealized_pnl;
        warn!(
            "budget circuit breaker triggered trader={} budget={} unrealized_pnl={}",
            cfg.trader_id, budget, upnl
        );

        let cycle_correlation_id = format!(
            "budget-breaker:{}:{}:{}",
            cfg.trader_id,
            now,
            Uuid::now_v7().simple()
        );

        // close all open positions
        match (exec_ctx.mode, live_adapter) {
            (RuntimeExecutionMode::LiveExchange, Some(adapter)) => {
                close_worst_positions_live(
                    state,
                    cfg,
                    &open_positions,
                    adapter,
                    now,
                    open_positions.len(),
                    "critical",
                    &cycle_correlation_id,
                )
                .await?;
            }
            _ => {
                for p in &open_positions {
                    let px = market
                        .get(&p.symbol)
                        .map(|m| m.price)
                        .unwrap_or(p.mark_price.max(1e-9));
                    close_position(state, cfg, p, px, now, "budget circuit breaker").await?;
                }
            }
        }

        // emit runtime event
        emit_runtime_event_best_effort(
            state,
            cfg,
            EVENT_BUDGET_CIRCUIT_BREAKER,
            "",
            "",
            "critical",
            "budget-risk-guard",
            "close-all-and-stop",
            &cycle_correlation_id,
            json!({
                "budget": budget,
                "unrealized_pnl": upnl,
                "realized_pnl": metrics.realized_pnl,
                "total_balance": metrics.total_balance,
            }),
            now,
        )
        .await;

        // push realtime event
        state
            .realtime_hub
            .publish(crate::realtime::RealtimeEvent::EngineStatus {
                trader_id: cfg.trader_id.clone(),
                status: "budget_exhausted".to_string(),
                message: format!(
                    "Budget circuit breaker: unrealized PnL {:.2} exceeded budget {:.2}",
                    upnl, budget
                ),
            });

        return Err(AppError::BudgetExhausted(format!(
            "unrealized PnL {:.2} exceeded budget {:.2}",
            upnl, budget
        )));
    }

    // 4.6) fixed TP/SL check — auto-close positions when unrealized PnL rate hits thresholds
    if !open_positions.is_empty() {
        let tp_sl = cfg.strategy_config.get("tp_sl");
        let tp_sl_mode = tp_sl
            .and_then(|t| t.get("mode"))
            .and_then(|v| v.as_str())
            .unwrap_or("fixed");
        if tp_sl_mode == "fixed" {
            let tp_rate = tp_sl
                .and_then(|t| t.get("fixed_tp_pnl_rate"))
                .and_then(|v| v.as_f64())
                .unwrap_or(f64::MAX);
            let sl_rate = tp_sl
                .and_then(|t| t.get("fixed_sl_pnl_rate"))
                .and_then(|v| v.as_f64())
                .unwrap_or(f64::MIN);

            for p in &open_positions {
                let pnl = (p.mark_price - p.entry_price) * p.quantity * if p.side == "LONG" { 1.0 } else { -1.0 };
                // PnL rate = unrealized PnL / position margin
                // position margin = (entry_price * quantity) / leverage
                let margin = (p.entry_price * p.quantity.abs()) / (p.leverage as f64).max(1.0);
                let pnl_rate = if margin > 0.0 { pnl / margin } else { 0.0 };
                let hit_tp = pnl_rate >= tp_rate;
                let hit_sl = pnl_rate <= sl_rate;
                if hit_tp || hit_sl {
                    let reason = if hit_tp { "fixed take-profit" } else { "fixed stop-loss" };
                    info!(
                        "[TP_SL] auto-close trader={} symbol={} side={} pnl={:.2} pnl_rate={:.4} margin={:.2} reason={}",
                        cfg.trader_id, p.symbol, p.side, pnl, pnl_rate, margin, reason
                    );
                    match (exec_ctx.mode, live_adapter) {
                        (RuntimeExecutionMode::LiveExchange, Some(adapter)) => {
                            // For live mode, submit a reduce-only market order
                            let close_side = if p.side == "LONG" { "SELL" } else { "BUY" };
                            let constraints = adapter.get_symbol_constraints(&p.symbol).await?;
                            let quantity = normalize_order_quantity_by_constraints(p.quantity.abs(), &constraints);
                            if quantity > f64::EPSILON {
                                let _ = adapter
                                    .place_order(crate::clients::exchanges::PlaceOrderRequest {
                                        symbol: p.symbol.clone(),
                                        side: if close_side == "SELL" {
                                            crate::clients::exchanges::ExchangeSide::Sell
                                        } else {
                                            crate::clients::exchanges::ExchangeSide::Buy
                                        },
                                        order_type: crate::clients::exchanges::ExchangeOrderType::Market,
                                        quantity,
                                        price: None,
                                        reduce_only: true,
                                        margin_mode: Some(margin_mode_for_config(cfg)),
                                        position_side: Some(if p.side == "LONG" {
                                            crate::clients::exchanges::PositionSide::Long
                                        } else {
                                            crate::clients::exchanges::PositionSide::Short
                                        }),
                                        time_in_force: None,
                                        client_order_id: Some(format!("tpsl_{}", Uuid::now_v7().simple())),
                                    })
                                    .await;
                            }
                        }
                        _ => {
                            let px = market
                                .get(&p.symbol)
                                .map(|m| m.price)
                                .unwrap_or(p.mark_price.max(1e-9));
                            close_position(state, cfg, p, px, now, reason).await?;
                        }
                    }
                }
            }
            // Reload positions after TP/SL closures
            open_positions = load_open_positions(state, &cfg.trader_id).await?;
            mark_to_market_positions(state, cfg, &mut open_positions, market, now).await?;
        }
    }

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
