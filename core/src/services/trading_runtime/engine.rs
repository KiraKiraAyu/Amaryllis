use super::fixed_tpsl::{
    configured_fixed_stop_loss, configured_fixed_take_profit, ensure_fixed_tp_sl_orders,
};
use super::service::*;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Allowed scan intervals in minutes.
pub const ALLOWED_SCAN_INTERVALS: [i64; 12] = [1, 5, 10, 15, 20, 30, 60, 120, 240, 480, 720, 1440];
const LIVE_CIRCUIT_BREAKER_LIMIT: u32 = 5;

fn live_circuit_breaker_message(consecutive_failures: u32) -> String {
    format!(
        "live circuit breaker opened after {} consecutive failures",
        consecutive_failures
    )
}

fn live_circuit_breaker_event(
    trader_id: &str,
    consecutive_failures: u32,
) -> crate::realtime::RealtimeEvent {
    crate::realtime::RealtimeEvent::EngineStatus {
        trader_id: trader_id.to_string(),
        status: "stopped".to_string(),
        message: live_circuit_breaker_message(consecutive_failures),
    }
}

fn publish_live_circuit_breaker_event(
    realtime_hub: &crate::realtime::RealtimeHub,
    trader_id: &str,
    consecutive_failures: u32,
) {
    realtime_hub.publish(live_circuit_breaker_event(
        trader_id,
        consecutive_failures,
    ));
}

/// Calculate the next aligned scan timestamp (Unix seconds).
/// Scans are aligned to epoch boundaries divisible by `interval_secs`.
fn next_aligned_scan(now: Duration, interval_secs: u64) -> u64 {
    let now_secs = now.as_secs();
    let remainder = now_secs % interval_secs;
    if remainder == 0 && now.subsec_nanos() == 0 {
        now_secs
    } else {
        now_secs - remainder + interval_secs
    }
}

/// Return the exact delay until the next epoch-aligned scan boundary.
fn next_aligned_scan_delay(now: Duration, interval_secs: u64) -> Duration {
    let seconds_into_interval = now.as_secs() % interval_secs;
    if seconds_into_interval == 0 && now.subsec_nanos() == 0 {
        Duration::ZERO
    } else {
        Duration::from_secs(interval_secs - seconds_into_interval)
            .saturating_sub(Duration::from_nanos(u64::from(now.subsec_nanos())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn live_circuit_breaker_event_is_terminal_and_explains_shutdown() {
        let realtime_hub = crate::realtime::RealtimeHub::new();
        let mut receiver = realtime_hub.subscribe();
        publish_live_circuit_breaker_event(&realtime_hub, "trader-1", 5);
        let event = receiver.try_recv().expect("expected shutdown event");

        let crate::realtime::RealtimeEvent::EngineStatus {
            trader_id,
            status,
            message,
        } = event.as_ref()
        else {
            panic!("expected an engine status event");
        };

        assert_eq!(trader_id, "trader-1");
        assert_eq!(status, "stopped");
        assert_eq!(
            message,
            "live circuit breaker opened after 5 consecutive failures"
        );
    }

    #[test]
    fn aligned_scan_delay_preserves_subsecond_precision() {
        assert_eq!(
            next_aligned_scan_delay(Duration::from_millis(299_200), 300),
            Duration::from_millis(800)
        );
        assert_eq!(next_aligned_scan(Duration::from_millis(299_200), 300), 300);
    }

    #[test]
    fn aligned_scan_delay_is_zero_at_an_exact_boundary() {
        assert_eq!(
            next_aligned_scan_delay(Duration::from_secs(300), 300),
            Duration::ZERO
        );
        assert_eq!(next_aligned_scan(Duration::from_secs(300), 300), 300);
    }
}

/// Update `next_scan_at` in the runtime engine state and push an SSE event
/// so the frontend countdown updates in real time without relying on polling.
fn update_next_scan_at(state: &SharedState, trader_id: &str, next_scan: Option<u64>) {
    if let Ok(mut manager) = state.runtime_engine_manager.write() {
        manager.set_next_scan_at(trader_id, next_scan, now_u64());
    }
    // Push real-time update to all SSE clients
    state
        .realtime_hub
        .publish(crate::realtime::RealtimeEvent::ScanSchedule {
            trader_id: trader_id.to_string(),
            next_scan_at: next_scan,
        });
}

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
    let interval_secs = (cfg.scan_interval_minutes.max(1) as u64) * 60;

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
    let live_circuit_breaker_limit = LIVE_CIRCUIT_BREAKER_LIMIT;

    let mut user_stream_rx: Option<mpsc::Receiver<ExchangeUserStreamEvent>> = None;
    let mut user_stream_session: Option<ExchangeUserStreamSession> = None;
    let mut user_stream_keepalive = time::interval(Duration::from_secs(30 * 60));
    user_stream_keepalive.set_missed_tick_behavior(MissedTickBehavior::Skip);
    // Consume the immediate first tick so it doesn't fire instantly in select!
    user_stream_keepalive.tick().await;
    let user_stream_reconnect_backoff = Duration::from_secs(2);

    // Mark the initial watch value as "seen" so changed() doesn't fire
    // immediately (Tokio watch receivers start with seen_version=0 while the
    // state version is 1, so changed() would return Ready right away).
    let _ = stop_rx.borrow_and_update();

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

    // Aligned scan loop: wait until the next epoch-aligned boundary, then scan.
    loop {
        // Calculate next aligned scan time and publish it for the frontend countdown.
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO);
        let next_scan = next_aligned_scan(now, interval_secs);
        let scan_delay = next_aligned_scan_delay(now, interval_secs);
        let scan_deadline = time::Instant::now() + scan_delay;
        update_next_scan_at(&engine.inner.state, &cfg.trader_id, Some(next_scan));

        info!(
            "trader={} waiting {:?} until next aligned scan at {}",
            cfg.trader_id, scan_delay, next_scan
        );

        // Wait until the aligned time (or stop signal / user stream events)
        if !scan_delay.is_zero() {
            tokio::select! {
                biased;
                _ = time::sleep_until(scan_deadline) => {}
                _ = user_stream_keepalive.tick() => {
                    handle_user_stream_keepalive(
                        &exec_ctx,
                        live_adapter.as_deref(),
                        &mut user_stream_session,
                        &mut user_stream_rx,
                        &cfg,
                        &mut stop_rx,
                        user_stream_reconnect_backoff,
                    ).await;
                    continue; // re-calculate sleep and try again
                }
                event = recv_user_stream_event(&mut user_stream_rx) => {
                    handle_user_stream_event_safe(
                        &engine.inner.state,
                        &cfg,
                        event,
                        &exec_ctx,
                        live_adapter.as_deref(),
                        &mut user_stream_session,
                        &mut user_stream_rx,
                        &mut stop_rx,
                        user_stream_reconnect_backoff,
                    ).await;
                    continue; // re-calculate sleep and try again
                }
                changed = stop_rx.changed() => {
                    match changed {
                        Ok(_) => {
                            if *stop_rx.borrow() {
                                break;
                            }
                            // Stop flag is false — re-calculate sleep and wait again.
                            continue;
                        }
                        Err(_) => break,
                    }
                }
            }
        }

        // Clear next_scan_at while scanning
        update_next_scan_at(&engine.inner.state, &cfg.trader_id, None);

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

                    // Broadcast cycle error via SSE so frontend can display a warning
                    engine.inner.state.realtime_hub.publish(
                        crate::realtime::RealtimeEvent::EngineStatus {
                            trader_id: cfg.trader_id.clone(),
                            status: "cycle_error".to_string(),
                            message: failure_msg.clone(),
                        },
                    );

                    if consecutive_live_failures >= live_circuit_breaker_limit {
                        let breaker_msg =
                            live_circuit_breaker_message(consecutive_live_failures);
                        let _ = engine.inner.state.set_runtime_engine_running(
                            &cfg.trader_id,
                            false,
                            Some(breaker_msg.clone()),
                        );
                        publish_live_circuit_breaker_event(
                            &engine.inner.state.realtime_hub,
                            &cfg.trader_id,
                            consecutive_live_failures,
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

/// Handle user stream keepalive tick — extracted from the old inline select! branch.
async fn handle_user_stream_keepalive(
    exec_ctx: &RuntimeExecutionContext,
    live_adapter: Option<&dyn LiveExchangeAdapter>,
    user_stream_session: &mut Option<ExchangeUserStreamSession>,
    user_stream_rx: &mut Option<mpsc::Receiver<ExchangeUserStreamEvent>>,
    cfg: &TraderRuntimeConfig,
    stop_rx: &mut watch::Receiver<bool>,
    backoff: Duration,
) {
    if exec_ctx.mode != RuntimeExecutionMode::LiveExchange {
        return;
    }
    if let (Some(adapter), Some(session)) = (live_adapter, user_stream_session.as_ref()) {
        if let Err(err) = adapter.keepalive_user_stream_session(session).await {
            warn!(
                "exchange user stream keepalive failed trader={} err={}",
                cfg.trader_id, err
            );

            if let Some(old_session) = user_stream_session.take() {
                let _ = adapter.close_user_stream_session(&old_session).await;
            }

            time::sleep(backoff).await;
            match init_exchange_user_stream(adapter).await {
                Ok(session) => {
                    *user_stream_rx =
                        Some(spawn_exchange_user_stream_reader(session.clone(), stop_rx.clone()));
                    *user_stream_session = Some(session);
                    info!(
                        "exchange user stream reconnected after keepalive failure trader={}",
                        cfg.trader_id
                    );
                }
                Err(reconnect_err) => {
                    warn!(
                        "exchange user stream reconnect failed after keepalive error trader={} err={}",
                        cfg.trader_id, reconnect_err
                    );
                    *user_stream_rx = None;
                    *user_stream_session = None;
                }
            }
        }
    }
}

/// Handle user stream events (and reconnection logic) — extracted from the old inline select! branch.
async fn handle_user_stream_event_safe(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    event: Option<ExchangeUserStreamEvent>,
    exec_ctx: &RuntimeExecutionContext,
    live_adapter: Option<&dyn LiveExchangeAdapter>,
    user_stream_session: &mut Option<ExchangeUserStreamSession>,
    user_stream_rx: &mut Option<mpsc::Receiver<ExchangeUserStreamEvent>>,
    stop_rx: &mut watch::Receiver<bool>,
    backoff: Duration,
) {
    if let Some(event) = event {
        let should_reconnect = matches!(event, ExchangeUserStreamEvent::ListenKeyExpired { .. });

        let now = now_i64();
        if let Err(err) = handle_exchange_user_stream_event(state, cfg, event, now).await {
            warn!(
                "exchange user stream event handling failed trader={} err={}",
                cfg.trader_id, err
            );
        }

        if should_reconnect && exec_ctx.mode == RuntimeExecutionMode::LiveExchange {
            if let Some(adapter) = live_adapter {
                if let Some(old_session) = user_stream_session.take() {
                    let _ = adapter.close_user_stream_session(&old_session).await;
                }

                time::sleep(backoff).await;
                match init_exchange_user_stream(adapter).await {
                    Ok(session) => {
                        *user_stream_rx =
                            Some(spawn_exchange_user_stream_reader(session.clone(), stop_rx.clone()));
                        *user_stream_session = Some(session);
                        info!(
                            "exchange user stream reconnected after listen key expiration trader={}",
                            cfg.trader_id
                        );
                    }
                    Err(reconnect_err) => {
                        warn!(
                            "exchange user stream reconnect failed after listen key expiration trader={} err={}",
                            cfg.trader_id, reconnect_err
                        );
                        *user_stream_rx = None;
                        *user_stream_session = None;
                    }
                }
            }
        }
    } else if exec_ctx.mode == RuntimeExecutionMode::LiveExchange {
        if let Some(adapter) = live_adapter {
            warn!(
                "exchange user stream disconnected trader={}, attempting reconnect",
                cfg.trader_id
            );

            if let Some(old_session) = user_stream_session.take() {
                let _ = adapter.close_user_stream_session(&old_session).await;
            }

            time::sleep(backoff).await;
            match init_exchange_user_stream(adapter).await {
                Ok(session) => {
                    *user_stream_rx =
                        Some(spawn_exchange_user_stream_reader(session.clone(), stop_rx.clone()));
                    *user_stream_session = Some(session);
                    info!(
                        "exchange user stream reconnected after disconnect trader={}",
                        cfg.trader_id
                    );
                }
                Err(reconnect_err) => {
                    warn!(
                        "exchange user stream reconnect failed after disconnect trader={} err={}",
                        cfg.trader_id, reconnect_err
                    );
                    *user_stream_rx = None;
                    *user_stream_session = None;
                }
            }
        }
    }
}

pub async fn process_cycle(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    symbols: &[String],
    market: &mut HashMap<String, MarketState>,
    exec_ctx: &RuntimeExecutionContext,
    live_adapter: Option<&dyn LiveExchangeAdapter>,
) -> Result<(), AppError> {
    let cycle_started_at = now_i64();

    // 1) advance synthetic market baseline
    advance_market(cfg, cycle_started_at as u64, symbols, market);

    // 2) if live mode, pre-sync account/positions and overlay prices from exchange
    if exec_ctx.mode == RuntimeExecutionMode::LiveExchange {
        if let Some(adapter) = live_adapter {
            sync_live_positions_and_balances(state, cfg, adapter, cycle_started_at).await?;
            ensure_fixed_tp_sl_orders(adapter, cfg).await?;
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
    mark_to_market_positions(state, cfg, &mut open_positions, market, cycle_started_at).await?;

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
            cycle_started_at,
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
                    cycle_started_at,
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
                    close_position(state, cfg, p, px, cycle_started_at, "budget circuit breaker").await?;
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
            cycle_started_at,
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

    // 4.6) Keep the polling fallback only for simulation and adapters without
    // exchange-hosted conditional orders. Aster installs fixed protection orders
    // during the live sync above and must not replace them with a delayed market close.
    let exchange_managed_fixed_tp_sl = exec_ctx.mode == RuntimeExecutionMode::LiveExchange
        && live_adapter
            .map(|adapter| adapter.exchange_type() == "aster")
            .unwrap_or(false);
    if !exchange_managed_fixed_tp_sl && !open_positions.is_empty() {
        let take_profit = configured_fixed_take_profit(&cfg.strategy_config)?;
        let stop_loss = configured_fixed_stop_loss(&cfg.strategy_config)?;
        if take_profit.is_some() || stop_loss.is_some() {
            for p in &open_positions {
                let pnl = (p.mark_price - p.entry_price)
                    * p.quantity
                    * if p.side == "LONG" { 1.0 } else { -1.0 };
                let margin = (p.entry_price * p.quantity.abs()) / (p.leverage as f64).max(1.0);
                let pnl_rate = if margin > 0.0 { pnl / margin } else { 0.0 };
                let hit_tp = take_profit
                    .is_some_and(|rule| pnl_rate >= rule.pnl_rate);
                let hit_sl = stop_loss
                    .is_some_and(|rule| pnl_rate <= rule.pnl_rate);
                if hit_tp || hit_sl {
                    let reason = if hit_tp {
                        "fixed take-profit"
                    } else {
                        "fixed stop-loss"
                    };
                    info!(
                        "[TP_SL] auto-close trader={} symbol={} side={} pnl={:.2} pnl_rate={:.4} margin={:.2} reason={}",
                        cfg.trader_id, p.symbol, p.side, pnl, pnl_rate, margin, reason
                    );
                    let px = market
                        .get(&p.symbol)
                        .map(|m| m.price)
                        .unwrap_or(p.mark_price.max(1e-9));
                    close_position(state, cfg, p, px, cycle_started_at, reason).await?;
                }
            }
            open_positions = load_open_positions(state, &cfg.trader_id).await?;
            mark_to_market_positions(state, cfg, &mut open_positions, market, cycle_started_at)
                .await?;
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
        cycle_started_at,
        Uuid::now_v7().simple()
    );

    let mut decisions = Vec::with_capacity(symbols.len());
    for sym in symbols {
        let trigger_source = if hard_risk_trigger {
            "hard_risk_guard"
        } else {
            "ai_model"
        };

        let decision_started_at = now_i64();
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
            cycle_started_at,
        )
        .await;

        let timing = DecisionTiming {
            cycle_started_at,
            decision_started_at,
            completed_at: now_i64(),
        };
        persist_decision(state, cfg, &signal, &metrics, timing).await?;
        decisions.push(TimedDecision { signal, timing });
    }

    // 7) execute decisions (live / simulated)
    let execution_started_at = now_i64();
    let decision_signals: Vec<DecisionSignal> = decisions
        .iter()
        .map(|decision| decision.signal.clone())
        .collect();
    match (exec_ctx.mode, live_adapter) {
        (RuntimeExecutionMode::LiveExchange, Some(adapter)) => {
            execute_decisions_live(
                state,
                cfg,
                &decision_signals,
                &open_positions,
                &metrics,
                market,
                adapter,
                execution_started_at,
                hard_risk_trigger,
                &live_risk_decision,
                &cycle_correlation_id,
            )
            .await?;
        }
        _ => {
            if hard_risk_trigger {
                close_worst_positions(
                    state,
                    cfg,
                    &open_positions,
                    market,
                    execution_started_at,
                )
                .await?;
            } else {
                execute_decisions(
                    state,
                    cfg,
                    &decision_signals,
                    &open_positions,
                    &metrics,
                    market,
                    execution_started_at,
                )
                .await?;
            }
        }
    }

    // 8) refresh account snapshot after execution
    let refreshed = compute_account_metrics(state, cfg).await?;
    let cycle_completed_at = now_i64();
    insert_account_snapshot(state, cfg, &refreshed, cycle_completed_at).await?;

    // Push equity snapshot to realtime clients
    state
        .realtime_hub
        .publish(crate::realtime::RealtimeEvent::EquitySnapshot {
            trader_id: cfg.trader_id.clone(),
            equity: refreshed.total_balance,
            available_cash: refreshed.available_balance,
            unrealized_pnl: refreshed.unrealized_pnl,
            ts: cycle_completed_at,
        });

    // Push each AI decision to realtime clients
    for decision in &decisions {
        state
            .realtime_hub
            .publish(crate::realtime::RealtimeEvent::AiDecision {
                trader_id: cfg.trader_id.clone(),
                decision: json!({
                    "symbol": decision.signal.symbol,
                    "action": decision.signal.action,
                    "confidence": decision.signal.confidence,
                    "reason": decision.signal.reason,
                    "timeframe": decision.signal.timeframe,
                    "risk_level": decision.signal.risk_level,
                    "correlation_id": decision.signal.correlation_id,
                    "cycle_started_at": decision.timing.cycle_started_at,
                    "decision_started_at": decision.timing.decision_started_at,
                    "completed_at": decision.timing.completed_at,
                }),
            });
    }

    // heartbeat
    state
        .trading_repo
        .set_trader_running(&cfg.trader_id, true, cycle_completed_at)
        .await?;

    Ok(())
}
