use super::service::*;

pub async fn sync_balance(
    app: &SharedState,
    id: &str,
) -> AppResult<TraderBalanceSyncPayload> {
    let trader = match get_trader_by_owner(app, id).await {
        Ok(Some(t)) => t,
        Ok(None) => return Err(app_error(AppErrorKind::NotFound, "Trader does not exist")),
        Err(_) => {
            return Err(app_error(AppErrorKind::Internal, "Failed to load trader"));
        }
    };

    if let Some(adapter) = enabled_live_adapter(app, &trader).await? {
        let balances = adapter.get_balances().await?;
        let balance = balances
            .iter()
            .find(|row| row.asset.eq_ignore_ascii_case("USDT"))
            .or_else(|| balances.first())
            .ok_or_else(|| {
                app_error(
                    AppErrorKind::BadGateway,
                    "Exchange returned no account balance",
                )
            })?;
        let total_balance = balance.wallet_balance.max(0.0);
        let available_balance = balance.available_balance.max(0.0);
        let now = now_ts();
        let account = TraderAccountRecord {
            trader_id: id.to_string(),
            total_balance,
            available_balance,
            used_margin: (total_balance - available_balance).max(0.0),
            unrealized_pnl: balance.unrealized_pnl,
            realized_pnl: 0.0,
            currency: balance.asset.trim().to_uppercase(),
            snapshot_at: now,
        };

        app.trading_repo
            .insert_account_snapshot(
                Uuid::now_v7().to_string(),
                id,
                &trader.exchange_id,
                &account,
                now,
            )
            .await
            .map_err(|_| {
                app_error(
                    AppErrorKind::Internal,
                    "Failed to persist live balance snapshot",
                )
            })?;

        return Ok(TraderBalanceSyncPayload {
            message: "Live balance synced",
            mode: "live".to_string(),
            account: account_payload(id.to_string(), account),
        });
    }

    let account = match app.trading_repo.latest_account(id).await {
        Ok(Some(account)) => account,
        Ok(None) => TraderAccountRecord {
            trader_id: id.to_string(),
            total_balance: trader.initial_balance.max(0.0),
            available_balance: trader.initial_balance.max(0.0),
            used_margin: 0.0,
            unrealized_pnl: 0.0,
            realized_pnl: 0.0,
            currency: "USDT".to_string(),
            snapshot_at: now_ts(),
        },
        Err(_) => {
            return Err(app_error(AppErrorKind::Internal, "Failed to sync account"));
        }
    };

    let now = now_ts();
    let snapshot_id = Uuid::now_v7().to_string();
    let result = app
        .trading_repo
        .insert_account_snapshot(snapshot_id, id, &trader.exchange_id, &account, now)
        .await;

    match result {
        Ok(_) => Ok(TraderBalanceSyncPayload {
            message: "Balance synced",
            mode: "local".to_string(),
            account: account_payload(id.to_string(), account.with_snapshot_at(now)),
        }),
        Err(_) => Err(app_error(
            AppErrorKind::Internal,
            "Failed to persist balance snapshot",
        )),
    }
}

pub async fn close_position(
    app: &SharedState,
    runtime: &TradingRuntimeService,
    id: &str,
    req: ClosePositionRequest,
) -> AppResult<ClosePositionPayload> {
    let symbol = req.symbol.trim().to_uppercase();
    let side = req.side.trim().to_uppercase();
    if symbol.is_empty() || (side != "LONG" && side != "SHORT") {
        return Err(app_error(
            AppErrorKind::BadRequest,
            "symbol and side(LONG/SHORT) are required",
        ));
    }

    let trader = match get_trader_by_owner(app, id).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            return Err(app_error(
                AppErrorKind::NotFound,
                "Trader does not exist or no permission",
            ));
        }
        Err(_) => {
            return Err(app_error(AppErrorKind::Internal, "Failed to load trader"));
        }
    };

    let open_positions = app
        .trading_repo
        .open_position_records(id, Some(&symbol), Some(&side))
        .await;
    let open_positions = match open_positions {
        Ok(v) if !v.is_empty() => v,
        Ok(_) => return Err(app_error(AppErrorKind::NotFound, "No open position found")),
        Err(_) => {
            return Err(app_error(AppErrorKind::Internal, "Failed to load position"));
        }
    };
    let position_count = open_positions.len();

    if !req.local_only {
        if let Some(adapter) = enabled_live_adapter(app, &trader).await? {
            let order_id = submit_live_close_order(
                &runtime.inner.state,
                &trader,
                adapter.as_ref(),
                &symbol,
                &side,
                &open_positions,
            )
            .await?;

            return Ok(ClosePositionPayload {
                message: "Close order submitted",
                mode: "live".to_string(),
                order_id,
                symbol,
                side,
            });
        }
    }

    let trade_ids = (0..position_count)
        .map(|_| Uuid::now_v7().to_string())
        .collect();
    let closed = app
        .trading_repo
        .close_open_positions(id, &symbol, &side, trade_ids, now_ts())
        .await;

    match closed {
        Ok(_) => Ok(ClosePositionPayload {
            message: "Position closed",
            mode: "local".to_string(),
            order_id: String::new(),
            symbol,
            side,
        }),
        Err(_) => Err(app_error(
            AppErrorKind::Internal,
            "Failed to close position",
        )),
    }
}

async fn enabled_live_adapter(
    app: &SharedState,
    trader: &TraderRecord,
) -> AppResult<Option<Box<dyn LiveExchangeAdapter>>> {
    let Some(row) = app
        .exchange_repo
        .find_runtime_config(&trader.exchange_id)
        .await
        .map_err(|_| app_error(AppErrorKind::Internal, "Failed to load exchange config"))?
    else {
        return Ok(None);
    };

    if !row.enabled {
        return Ok(None);
    }

    if exchange_credentials_missing(
        &row.exchange_type,
        &row.api_key,
        &row.secret_key,
        &row.passphrase,
        &row.hyperliquid_wallet_addr,
    ) {
        return Err(AppError::InvalidExchangeConfig(
            "enabled exchange credentials are incomplete".to_string(),
        ));
    }

    let credentials = ExchangeCredentials {
        api_key: row.api_key,
        secret_key: row.secret_key,
        passphrase: if row.passphrase.trim().is_empty() {
            None
        } else {
            Some(row.passphrase)
        },
        wallet_addr: if row.hyperliquid_wallet_addr.trim().is_empty() {
            None
        } else {
            Some(row.hyperliquid_wallet_addr)
        },
        testnet: row.testnet,
    };

    let adapter = create_exchange_adapter(&row.exchange_type, credentials)?;
    adapter.ping().await?;

    Ok(Some(adapter))
}

async fn submit_live_close_order(
    runtime_state: &crate::services::trading_runtime::models::SharedState,
    trader: &TraderRecord,
    adapter: &dyn LiveExchangeAdapter,
    symbol: &str,
    side: &str,
    open_positions: &[TraderPositionRecord],
) -> AppResult<String> {
    let raw_qty = open_positions
        .iter()
        .map(|position| position.quantity.abs())
        .sum::<f64>();
    let constraints = adapter.get_symbol_constraints(symbol).await?;
    let quantity = normalize_order_quantity_by_constraints(raw_qty, &constraints);
    if quantity <= f64::EPSILON {
        return Err(app_error(
            AppErrorKind::BadRequest,
            "Position quantity is below exchange minimum",
        ));
    }

    let cfg = manual_runtime_config(trader);
    let close_side = close_order_side(side)?;
    let position_side = order_position_side(side)?;
    let order = adapter
        .place_order(PlaceOrderRequest {
            symbol: symbol.to_string(),
            side: close_side,
            order_type: ExchangeOrderType::Market,
            quantity,
            price: None,
            reduce_only: true,
            margin_mode: Some(
                crate::services::trading_runtime::config_loaders::margin_mode_for_config(&cfg),
            ),
            position_side: Some(position_side),
            time_in_force: None,
            client_order_id: Some(format!("manual_{}", Uuid::now_v7().simple())),
        })
        .await?;

    persist_live_order_record(runtime_state, &cfg, adapter, &order, side, true, now_ts()).await?;
    Ok(order.order_id)
}

fn close_order_side(side: &str) -> AppResult<ExchangeSide> {
    match side.trim().to_ascii_uppercase().as_str() {
        "LONG" => Ok(ExchangeSide::Sell),
        "SHORT" => Ok(ExchangeSide::Buy),
        _ => Err(app_error(
            AppErrorKind::BadRequest,
            "side must be LONG or SHORT",
        )),
    }
}

fn order_position_side(side: &str) -> AppResult<PositionSide> {
    match side.trim().to_ascii_uppercase().as_str() {
        "LONG" => Ok(PositionSide::Long),
        "SHORT" => Ok(PositionSide::Short),
        _ => Err(app_error(
            AppErrorKind::BadRequest,
            "side must be LONG or SHORT",
        )),
    }
}

fn manual_runtime_config(trader: &TraderRecord) -> TraderRuntimeConfig {
    TraderRuntimeConfig {
        trader_id: trader.id.clone(),
        name: trader.name.clone(),
        ai_model_id: trader.ai_model_id.clone(),
        ai_model_name: String::new(),
        ai_provider_type: String::new(),
        ai_api_key: String::new(),
        ai_base_url: String::new(),
        exchange_id: trader.exchange_id.clone(),
        scan_interval_minutes: trader.scan_interval_minutes,
        initial_balance: trader.initial_balance,
        is_cross_margin: trader.is_cross_margin != 0,
        trading_symbols: String::new(),
        custom_prompt: trader.custom_prompt.clone(),
        override_base_prompt: trader.override_base_prompt != 0,
        system_prompt_template: trader.system_prompt_template.clone(),
        symbols_config: Vec::new(),
        strategy_config: serde_json::json!({}),
    }
}

pub async fn grid_risk_info(
    app: &SharedState,
    id: &str,
) -> AppResult<GridRiskInfoPayload> {
    if trader_owner_missing(app, id).await {
        return Err(app_error(
            AppErrorKind::NotFound,
            "Trader does not exist or no permission",
        ));
    }

    let rows = app
        .trading_repo
        .open_position_records(id, None, None)
        .await;

    match rows {
        Ok(open_positions) => {
            let mut total_notional = 0.0;
            let mut by_symbol: HashMap<String, f64> = HashMap::new();
            for position in open_positions {
                let notional = (position.quantity * position.mark_price).abs();
                total_notional += notional;
                *by_symbol.entry(position.symbol).or_insert(0.0) += notional;
            }

            let mut symbol_concentration: Vec<SymbolConcentrationPayload> = by_symbol
                .into_iter()
                .map(|(symbol, value)| SymbolConcentrationPayload {
                    symbol,
                    notional: value,
                    weight_pct: if total_notional > 0.0 {
                        value * 100.0 / total_notional
                    } else {
                        0.0
                    },
                })
                .collect();

            symbol_concentration.sort_by(|a, b| {
                b.notional
                    .partial_cmp(&a.notional)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            Ok(GridRiskInfoPayload {
                trader_id: id.to_string(),
                total_notional,
                symbol_concentration,
            })
        }
        Err(_) => Err(app_error(
            AppErrorKind::Internal,
            "Failed to get grid risk info",
        )),
    }
}

pub async fn status(
    app: &SharedState,
    q: TraderQuery,
) -> AppResult<TraderStatusPayload> {
    let trader_id = match resolve_trader_id(app, q.trader_id).await {
        Ok(v) => v,
        Err(e) => return Err(e),
    };

    let trader = match get_trader_by_owner(app, &trader_id).await {
        Ok(Some(t)) => t,
        _ => return Err(app_error(AppErrorKind::NotFound, "Trader not found")),
    };

    let (open_positions, open_orders) = app
        .trading_repo
        .open_counts(&trader_id)
        .await
        .unwrap_or((0, 0));

    let runtime_engine = app.runtime_engine(&trader_id).ok().flatten();
    let runtime_running = runtime_engine
        .as_ref()
        .map(|v| v.is_running)
        .unwrap_or(trader.is_running != 0);

    Ok(TraderStatusPayload {
        trader_id,
        is_running: runtime_running,
        open_positions,
        open_orders,
        last_updated: trader.updated_at,
        runtime_engine: runtime_engine.as_ref().map(runtime_engine_payload),
    })
}

pub async fn account(
    app: &SharedState,
    q: TraderQuery,
) -> AppResult<TraderAccountPayload> {
    let trader_id = match resolve_trader_id(app, q.trader_id).await {
        Ok(v) => v,
        Err(e) => return Err(e),
    };

    match app.trading_repo.latest_account(&trader_id).await {
        Ok(Some(account)) => Ok(account_payload(trader_id, account)),
        Ok(None) => Err(app_error(
            AppErrorKind::NotFound,
            "Account snapshot not found",
        )),
        Err(_) => Err(app_error(AppErrorKind::Internal, "Failed to load account")),
    }
}

pub async fn positions(
    app: &SharedState,
    q: PositionQuery,
) -> AppResult<PositionListPayload> {
    let trader_id = match resolve_trader_id(app, q.trader_id).await {
        Ok(v) => v,
        Err(e) => return Err(e),
    };

    let status = q
        .status
        .unwrap_or_else(|| "open".to_string())
        .trim()
        .to_lowercase();

    match app
        .trading_repo
        .positions_by_status(&trader_id, &status)
        .await
    {
        Ok(items) => {
            let items: Vec<PositionPayload> = items.into_iter().map(position_payload).collect();
            Ok(PositionListPayload {
                count: items.len(),
                items,
                trader_id,
            })
        }
        Err(_) => Err(app_error(
            AppErrorKind::Internal,
            "Failed to load positions",
        )),
    }
}

pub async fn positions_history(
    app: &SharedState,
    q: PaginationQuery,
) -> AppResult<PositionListPayload> {
    let trader_id = match resolve_trader_id(app, q.trader_id).await {
        Ok(v) => v,
        Err(e) => return Err(e),
    };

    let limit = q.limit.unwrap_or(100).clamp(1, 500);
    let offset = q.offset.unwrap_or(0).max(0);

    match app
        .trading_repo
        .closed_positions(&trader_id, limit, offset)
        .await
    {
        Ok(items) => {
            let items: Vec<PositionPayload> = items.into_iter().map(position_payload).collect();
            Ok(PositionListPayload {
                count: items.len(),
                items,
                trader_id,
            })
        }
        Err(_) => Err(app_error(
            AppErrorKind::Internal,
            "Failed to load position history",
        )),
    }
}

trait AccountSnapshotExt {
    fn with_snapshot_at(self, snapshot_at: i64) -> TraderAccountRecord;
}

impl AccountSnapshotExt for TraderAccountRecord {
    fn with_snapshot_at(mut self, snapshot_at: i64) -> TraderAccountRecord {
        self.snapshot_at = snapshot_at;
        self
    }
}

fn account_payload(trader_id: String, account: TraderAccountRecord) -> TraderAccountPayload {
    TraderAccountPayload {
        trader_id,
        total_balance: account.total_balance,
        available_balance: account.available_balance,
        used_margin: account.used_margin,
        unrealized_pnl: account.unrealized_pnl,
        realized_pnl: account.realized_pnl,
        currency: account.currency,
        snapshot_at: account.snapshot_at,
    }
}

pub(crate) fn position_payload(position: TraderPositionRecord) -> PositionPayload {
    PositionPayload {
        id: position.id,
        trader_id: position.trader_id,
        symbol: position.symbol,
        side: position.side,
        quantity: position.quantity,
        entry_price: position.entry_price,
        mark_price: position.mark_price,
        liquidation_price: position.liquidation_price,
        leverage: position.leverage,
        margin_mode: position.margin_mode,
        unrealized_pnl: position.unrealized_pnl,
        realized_pnl: position.realized_pnl,
        status: position.status,
        opened_at: position.opened_at,
        closed_at: position.closed_at,
        updated_at: position.updated_at,
    }
}

#[cfg(test)]
mod tests {
    use crate::repositories::trading::records::positions::TraderPositionRecord;

    use super::*;

    #[test]
    fn position_payload_preserves_complete_position_record() {
        let payload = position_payload(TraderPositionRecord {
            id: "position_1".to_string(),
            trader_id: "trader_1".to_string(),
            symbol: "BTCUSDT".to_string(),
            side: "LONG".to_string(),
            quantity: 1.25,
            entry_price: 256.5,
            mark_price: 260.75,
            liquidation_price: 128.0,
            leverage: 5,
            margin_mode: "cross".to_string(),
            unrealized_pnl: 12.5,
            realized_pnl: -2.5,
            status: "open".to_string(),
            opened_at: 1_700_000_000,
            closed_at: Some(1_700_000_600),
            updated_at: 1_700_000_900,
        });

        assert_eq!(payload.id, "position_1");
        assert_eq!(payload.trader_id, "trader_1");
        assert_eq!(payload.symbol, "BTCUSDT");
        assert_eq!(payload.side, "LONG");
        assert_eq!(payload.quantity, 1.25);
        assert_eq!(payload.entry_price, 256.5);
        assert_eq!(payload.mark_price, 260.75);
        assert_eq!(payload.liquidation_price, 128.0);
        assert_eq!(payload.leverage, 5);
        assert_eq!(payload.margin_mode, "cross");
        assert_eq!(payload.unrealized_pnl, 12.5);
        assert_eq!(payload.realized_pnl, -2.5);
        assert_eq!(payload.status, "open");
        assert_eq!(payload.opened_at, 1_700_000_000);
        assert_eq!(payload.closed_at, Some(1_700_000_600));
        assert_eq!(payload.updated_at, 1_700_000_900);
    }

    #[test]
    fn live_close_order_side_inverts_position_side() {
        assert!(matches!(
            close_order_side("LONG").expect("long close side"),
            ExchangeSide::Sell
        ));
        assert!(matches!(
            close_order_side("SHORT").expect("short close side"),
            ExchangeSide::Buy
        ));
    }

    #[test]
    fn live_close_position_side_preserves_target_position() {
        assert!(matches!(
            order_position_side("LONG").expect("long position side"),
            PositionSide::Long
        ));
        assert!(matches!(
            order_position_side("SHORT").expect("short position side"),
            PositionSide::Short
        ));
    }
}
