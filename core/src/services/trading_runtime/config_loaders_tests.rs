use super::service::*;
use crate::clients::exchanges::{
    ExchangeBalance, ExchangeOpenOrder, ExchangeOrderDetail, ExchangePosition,
    ExchangeSymbolConstraints, PlaceOrderResponse,
};
use crate::error::AppError;
use crate::services::trading_runtime::test_support::*;
use crate::{repositories::ModelRepo, services::llm::LlmService};
use envconfig::Envconfig;
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use tokio::sync::Mutex;

fn default_live_config() -> crate::config::LiveRuntimeConfig {
    crate::config::LiveRuntimeConfig::init_from_hashmap(&HashMap::new())
        .expect("load default live runtime config")
}

fn test_llm_service(db: &DatabaseConnection) -> Arc<LlmService> {
    Arc::new(LlmService::new(Arc::new(ModelRepo::new(db.clone()))))
}

async fn test_state_and_cfg() -> (TestRuntimeState, TraderRuntimeConfig) {
    let pool = crate::database::init_database("sqlite::memory:")
        .await
        .expect("connect sqlite memory");

    let state = TestRuntimeState::new(
        pool.clone(),
        default_live_config(),
        Arc::new(RwLock::new(crate::state::RuntimeEngineManager::default())),
        test_llm_service(&pool),
        crate::realtime::RealtimeHub::new(),
    );
    let cfg = TraderRuntimeConfig {
        trader_id: "trader_test_1".to_string(),
        name: "test-trader".to_string(),
        ai_model_id: "deepseek-chat".to_string(),
        ai_model_name: "deepseek-chat".to_string(),
        ai_provider_type: "openai".to_string(),
        ai_api_key: String::new(),
        ai_base_url: "https://api.deepseek.com/v1".to_string(),
        exchange_id: "exchange-test".to_string(),
        scan_interval_minutes: 1,
        initial_balance: 1000.0,
        is_cross_margin: true,
        trading_symbols: "BTCUSDT,ETHUSDT".to_string(),
        custom_prompt: String::new(),
        override_base_prompt: false,
        system_prompt_template: "default".to_string(),
        symbols_config: vec![
            SymbolConfig {
                symbol: "BTCUSDT".to_string(),
                leverage: 5,
                min_cost: None,
                max_cost: None,
                fixed_cost: None,
            },
            SymbolConfig {
                symbol: "ETHUSDT".to_string(),
                leverage: 5,
                min_cost: None,
                max_cost: None,
                fixed_cost: None,
            },
        ],
    };

    (state, cfg)
}

async fn load_execution_intent_by_key(
    state: &TestRuntimeState,
    cfg: &TraderRuntimeConfig,
    intent_key: &str,
) -> execution_intents::Model {
    execution_intents::Entity::find()
        .filter(execution_intents::Column::TraderId.eq(&cfg.trader_id))
        .filter(execution_intents::Column::IntentKey.eq(intent_key))
        .one(&state.db)
        .await
        .expect("query execution intent by key")
        .expect("execution intent should exist")
}

async fn load_execution_intent_by_exchange_order_id(
    state: &TestRuntimeState,
    cfg: &TraderRuntimeConfig,
    exchange_order_id: &str,
) -> execution_intents::Model {
    execution_intents::Entity::find()
        .filter(execution_intents::Column::TraderId.eq(&cfg.trader_id))
        .filter(execution_intents::Column::ExchangeOrderId.eq(exchange_order_id))
        .one(&state.db)
        .await
        .expect("query execution intent by exchange order id")
        .expect("execution intent should exist")
}

async fn load_latest_decision(
    state: &TestRuntimeState,
    cfg: &TraderRuntimeConfig,
    symbol: &str,
) -> trader_decisions::Model {
    trader_decisions::Entity::find()
        .filter(trader_decisions::Column::TraderId.eq(&cfg.trader_id))
        .filter(trader_decisions::Column::Symbol.eq(symbol))
        .order_by_desc(trader_decisions::Column::CreatedAt)
        .one(&state.db)
        .await
        .expect("query latest decision")
        .expect("decision should exist")
}

async fn load_position_by_id(
    state: &TestRuntimeState,
    position_id: &str,
) -> trader_positions::Model {
    trader_positions::Entity::find_by_id(position_id.to_string())
        .one(&state.db)
        .await
        .expect("query position by id")
        .expect("position should exist")
}

async fn load_latest_trade(
    state: &TestRuntimeState,
    cfg: &TraderRuntimeConfig,
) -> trader_trades::Model {
    trader_trades::Entity::find()
        .filter(trader_trades::Column::TraderId.eq(&cfg.trader_id))
        .order_by_desc(trader_trades::Column::CreatedAt)
        .one(&state.db)
        .await
        .expect("query latest trade")
        .expect("trade should exist")
}

async fn load_order_by_id(state: &TestRuntimeState, order_id: &str) -> trader_orders::Model {
    trader_orders::Entity::find_by_id(order_id.to_string())
        .one(&state.db)
        .await
        .expect("query order by id")
        .expect("order should exist")
}

async fn load_order_by_exchange_order_id(
    state: &TestRuntimeState,
    cfg: &TraderRuntimeConfig,
    exchange_order_id: &str,
) -> trader_orders::Model {
    trader_orders::Entity::find()
        .filter(trader_orders::Column::TraderId.eq(&cfg.trader_id))
        .filter(trader_orders::Column::ExchangeOrderId.eq(exchange_order_id))
        .one(&state.db)
        .await
        .expect("query order by exchange order id")
        .expect("order should exist")
}

async fn count_order_fills_by_exchange_trade_id(
    state: &TestRuntimeState,
    cfg: &TraderRuntimeConfig,
    exchange_trade_id: &str,
) -> u64 {
    order_fills::Entity::find()
        .filter(order_fills::Column::TraderId.eq(&cfg.trader_id))
        .filter(order_fills::Column::ExchangeTradeId.eq(exchange_trade_id))
        .count(&state.db)
        .await
        .expect("count order fills")
}

async fn count_trades_by_symbol(
    state: &TestRuntimeState,
    cfg: &TraderRuntimeConfig,
    symbol: &str,
) -> u64 {
    trader_trades::Entity::find()
        .filter(trader_trades::Column::TraderId.eq(&cfg.trader_id))
        .filter(trader_trades::Column::Symbol.eq(symbol))
        .count(&state.db)
        .await
        .expect("count trader trades")
}

async fn count_orders(state: &TestRuntimeState, cfg: &TraderRuntimeConfig) -> u64 {
    trader_orders::Entity::find()
        .filter(trader_orders::Column::TraderId.eq(&cfg.trader_id))
        .count(&state.db)
        .await
        .expect("count trader orders")
}

#[allow(clippy::too_many_arguments)]
async fn insert_test_execution_intent(
    state: &TestRuntimeState,
    cfg: &TraderRuntimeConfig,
    id: &str,
    intent_key: &str,
    symbol: &str,
    side: &str,
    decision: &str,
    status: &str,
    exchange_order_id: &str,
    payload_json: &str,
    created_at: i64,
    updated_at: i64,
) {
    execution_intents::Entity::insert(execution_intents::ActiveModel {
        id: Set(id.to_string()),
        trader_id: Set(cfg.trader_id.clone()),
        intent_key: Set(intent_key.to_string()),
        symbol: Set(symbol.to_string()),
        side: Set(side.to_string()),
        decision: Set(decision.to_string()),
        status: Set(status.to_string()),
        exchange_order_id: Set(exchange_order_id.to_string()),
        payload_json: Set(payload_json.to_string()),
        created_at: Set(ts_to_i32(created_at)),
        updated_at: Set(ts_to_i32(updated_at)),
    })
    .exec(&state.db)
    .await
    .expect("insert test execution intent");
}

#[allow(clippy::too_many_arguments)]
async fn insert_test_position(
    state: &TestRuntimeState,
    cfg: &TraderRuntimeConfig,
    id: &str,
    symbol: &str,
    side: &str,
    quantity: f64,
    entry_price: f64,
    mark_price: f64,
    liquidation_price: f64,
    leverage: i64,
    unrealized_pnl: f64,
    realized_pnl: f64,
    status: &str,
    opened_at: i64,
    closed_at: Option<i64>,
    created_at: i64,
    updated_at: i64,
) {
    trader_positions::Entity::insert(trader_positions::ActiveModel {
        id: Set(id.to_string()),
        trader_id: Set(cfg.trader_id.clone()),
        symbol: Set(symbol.to_string()),
        side: Set(side.to_string()),
        quantity: Set(decimal_from_f64(quantity)),
        entry_price: Set(decimal_from_f64(entry_price)),
        mark_price: Set(decimal_from_f64(mark_price)),
        liquidation_price: Set(decimal_from_f64(liquidation_price)),
        leverage: Set(leverage as i32),
        margin_mode: Set("cross".to_string()),
        unrealized_pnl: Set(decimal_from_f64(unrealized_pnl)),
        realized_pnl: Set(decimal_from_f64(realized_pnl)),
        status: Set(status.to_string()),
        opened_at: Set(ts_to_i32(opened_at)),
        closed_at: Set(closed_at.map(ts_to_i32)),
        created_at: Set(ts_to_i32(created_at)),
        updated_at: Set(ts_to_i32(updated_at)),
    })
    .exec(&state.db)
    .await
    .expect("insert test position");
}

#[allow(clippy::too_many_arguments)]
async fn insert_test_order(
    state: &TestRuntimeState,
    cfg: &TraderRuntimeConfig,
    id: &str,
    exchange_order_id: &str,
    client_order_id: &str,
    symbol: &str,
    side: &str,
    position_side: &str,
    order_type: &str,
    status: &str,
    price: f64,
    quantity: f64,
    filled_quantity: f64,
    avg_fill_price: f64,
    reduce_only: bool,
    time_in_force: &str,
    placed_at: i64,
    updated_at: i64,
    closed_at: Option<i64>,
) {
    trader_orders::Entity::insert(trader_orders::ActiveModel {
        id: Set(id.to_string()),
        trader_id: Set(cfg.trader_id.clone()),
        exchange_order_id: Set(exchange_order_id.to_string()),
        client_order_id: Set(client_order_id.to_string()),
        symbol: Set(symbol.to_string()),
        side: Set(side.to_string()),
        position_side: Set(position_side.to_string()),
        order_type: Set(order_type.to_string()),
        status: Set(status.to_string()),
        price: Set(decimal_from_f64(price)),
        quantity: Set(decimal_from_f64(quantity)),
        filled_quantity: Set(decimal_from_f64(filled_quantity)),
        avg_fill_price: Set(decimal_from_f64(avg_fill_price)),
        reduce_only: Set(int_flag(reduce_only)),
        time_in_force: Set(time_in_force.to_string()),
        placed_at: Set(ts_to_i32(placed_at)),
        updated_at: Set(ts_to_i32(updated_at)),
        closed_at: Set(closed_at.map(ts_to_i32)),
    })
    .exec(&state.db)
    .await
    .expect("insert test order");
}

#[test]
fn test_status_transition_guard_terminal_not_rollback() {
    assert_eq!(
        resolve_order_status_transition("filled", "new"),
        "filled".to_string()
    );
    assert_eq!(
        resolve_order_status_transition("canceled", "open"),
        "canceled".to_string()
    );
    assert_eq!(
        resolve_order_status_transition("partially_filled", "new"),
        "partially_filled".to_string()
    );
    assert_eq!(
        resolve_order_status_transition("new", "partially_filled"),
        "partially_filled".to_string()
    );
}

#[tokio::test]
async fn test_execution_intent_idempotency_lifecycle() {
    let (state, cfg) = test_state_and_cfg().await;
    let ts = 1_700_000_000_i64;
    let intent_key = "open:trader_test_1:BTCUSDT:LONG:123456";

    let first = try_register_execution_intent(
        &state,
        &cfg,
        intent_key,
        "BTCUSDT",
        "LONG",
        "open",
        "test",
        "open",
        "normal",
        "corr_test_1",
        ts,
    )
    .await
    .expect("first intent insert");
    assert!(first);

    let second = try_register_execution_intent(
        &state,
        &cfg,
        intent_key,
        "BTCUSDT",
        "LONG",
        "open",
        "test",
        "open",
        "normal",
        "corr_test_1",
        ts + 1,
    )
    .await
    .expect("second intent insert");
    assert!(!second);

    mark_execution_intent_submitted(&state, &cfg, intent_key, "987654321", ts + 2)
        .await
        .expect("mark submitted");

    let submitted = load_execution_intent_by_key(&state, &cfg, intent_key).await;

    assert_eq!(submitted.status, "submitted".to_string());
    assert_eq!(submitted.exchange_order_id, "987654321".to_string());

    let payload: Value =
        serde_json::from_str(&submitted.payload_json).expect("parse intent payload json");
    assert_eq!(payload["trigger_source"], "test");
    assert_eq!(payload["action_taken"], "open");
    assert_eq!(payload["risk_level"], "normal");
    assert_eq!(payload["correlation_id"], "corr_test_1");

    finalize_execution_intent_for_exchange_order(&state, &cfg, "987654321", "filled", ts + 3)
        .await
        .expect("finalize intent");

    let finalized = load_execution_intent_by_key(&state, &cfg, intent_key).await;
    assert_eq!(finalized.status, "filled".to_string());
}

#[tokio::test]
async fn test_persist_decision_payload_includes_audit_metadata() {
    let (state, cfg) = test_state_and_cfg().await;
    let ts = 1_700_100_000_i64;

    let mut market = HashMap::new();
    market.insert(
        "BTCUSDT".to_string(),
        MarketState {
            price: 101.0,
            prev_price: 100.0,
            volatility: 0.01,
        },
    );

    let decision = generate_fallback_decision(
        "BTCUSDT",
        market.get("BTCUSDT").expect("market state"),
        false,
        "soft",
        "market_signal",
        "corr_decision_1",
        String::new(),
        None,
    );

    let metrics = AccountMetrics {
        total_balance: 1000.0,
        available_balance: 850.0,
        used_margin: 150.0,
        unrealized_pnl: 10.0,
        realized_pnl: 5.0,
        margin_used_ratio: 0.15,
    };

    persist_decision(&state, &cfg, &decision, &metrics, ts)
        .await
        .expect("persist decision with audit payload");

    let row = load_latest_decision(&state, &cfg, "BTCUSDT").await;
    let payload: Value = serde_json::from_str(&row.payload_json).expect("parse payload");

    assert_eq!(payload["risk_level"], "soft");
    assert_eq!(payload["trigger_source"], "market_signal");
    assert_eq!(payload["correlation_id"], "corr_decision_1");
    assert_eq!(payload["action_taken"], decision.action_taken.as_str());
}

async fn insert_test_trader(state: &TestRuntimeState, cfg: &TraderRuntimeConfig, is_running: bool) {
    let ts = 1_700_000_000_i64;
    traders::Entity::insert(traders::ActiveModel {
        id: Set(cfg.trader_id.clone()),
        name: Set(cfg.name.clone()),
        ai_model_id: Set(cfg.ai_model_id.clone()),
        exchange_id: Set(cfg.exchange_id.clone()),
        strategy_id: Set(String::new()),
        initial_balance: Set(decimal_from_f64(cfg.initial_balance)),
        scan_interval_minutes: Set(cfg.scan_interval_minutes as i32),
        is_running: Set(int_flag(is_running)),
        is_cross_margin: Set(1),
        use_ai500: Set(0),
        use_oi_top: Set(0),
        custom_prompt: Set(cfg.custom_prompt.clone()),
        override_base_prompt: Set(0),
        system_prompt_template: Set("default".to_string()),
        created_at: Set(ts_to_i32(ts)),
        updated_at: Set(ts_to_i32(ts)),
    })
    .exec(&state.db)
    .await
    .expect("insert trader");
}

#[tokio::test]
async fn test_startup_recovery_resumes_running_trader() {
    let (state, mut cfg) = test_state_and_cfg().await;
    cfg.trader_id = "trader_recovery_1".to_string();
    cfg.exchange_id = "exchange_recovery_missing".to_string();
    cfg.trading_symbols = "BTCUSDT".to_string();
    // Must reference a model that actually resolves (provider presets are seeded
    // into the in-memory database by `ensure_defaults`).
    cfg.ai_model_id = "deepseek-v4-pro".to_string();

    insert_test_trader(&state, &cfg, true).await;

    let engine = TradingRuntimeService::new_for_test(
        state.db.clone(),
        state.config.live.clone(),
        state.runtime_engine_manager.clone(),
        state.llm_service.clone(),
        state.realtime_hub.clone(),
    );
    let recovered = engine
        .recover_running_traders_from_db()
        .await
        .expect("recover running traders");

    assert!(recovered.iter().any(|id| id == &cfg.trader_id));
    assert!(engine.is_running(&cfg.trader_id).await);

    engine
        .stop_trader_for_user(&cfg.trader_id)
        .await
        .expect("stop recovered trader");
}

#[tokio::test]
async fn test_finalize_execution_intent_terminal_only() {
    let (state, cfg) = test_state_and_cfg().await;
    let ts = 1_700_100_000_i64;

    insert_test_execution_intent(
        &state,
        &cfg,
        "intent_finalization_1",
        "open:trader_test_1:BTCUSDT:LONG:777",
        "BTCUSDT",
        "LONG",
        "open",
        "submitted",
        "ex_order_777",
        "{}",
        ts,
        ts,
    )
    .await;

    finalize_execution_intent_for_exchange_order(&state, &cfg, "ex_order_777", "new", ts + 1)
        .await
        .expect("non-terminal finalize no-op");

    let row_non_terminal =
        load_execution_intent_by_exchange_order_id(&state, &cfg, "ex_order_777").await;

    assert_eq!(row_non_terminal.status, "submitted".to_string());

    finalize_execution_intent_for_exchange_order(&state, &cfg, "ex_order_777", "canceled", ts + 2)
        .await
        .expect("terminal finalize");

    let row_terminal =
        load_execution_intent_by_exchange_order_id(&state, &cfg, "ex_order_777").await;

    assert_eq!(row_terminal.status, "canceled".to_string());
}

#[test]
fn test_status_transition_guard_regression_matrix() {
    assert_eq!(
        resolve_order_status_transition("partially_filled", "open"),
        "partially_filled".to_string()
    );
    assert_eq!(
        resolve_order_status_transition("partially_filled", "new"),
        "partially_filled".to_string()
    );
    assert_eq!(
        resolve_order_status_transition("new", "open"),
        "new".to_string()
    );
    assert_eq!(
        resolve_order_status_transition("open", "new"),
        "new".to_string()
    );
    assert_eq!(
        resolve_order_status_transition("new", "filled"),
        "filled".to_string()
    );
    assert_eq!(
        resolve_order_status_transition("rejected", "filled"),
        "rejected".to_string()
    );
    assert_eq!(resolve_order_status_transition("", ""), "open".to_string());
}

#[tokio::test]
async fn test_reduce_only_fill_compensation_partial_close() {
    let (state, cfg) = test_state_and_cfg().await;
    let ts = 1_700_200_000_i64;

    let pos_id = "pos_reduce_only_1";
    insert_test_position(
        &state, &cfg, pos_id, "BTCUSDT", "LONG", 2.0, 100.0, 100.0, 70.0, 5, 0.0, 0.0, "open", ts,
        None, ts, ts,
    )
    .await;

    apply_reduce_only_fill_to_local_positions(
        &state,
        &cfg,
        "BTCUSDT",
        "SELL",
        1.25,
        110.0,
        0.5,
        ts + 5,
        ts + 6,
    )
    .await
    .expect("apply reduce-only fill compensation");

    let updated = load_position_by_id(&state, pos_id).await;

    let qty = decimal_to_f64(&updated.quantity);
    let status = updated.status;
    let realized_pnl = decimal_to_f64(&updated.realized_pnl);
    let closed_at = crate::time::opt_dt_to_ts(updated.closed_at);

    assert!((qty - 0.75).abs() < 1e-9, "remaining qty should be 0.75");
    assert_eq!(status, "open".to_string());
    assert!(
        (realized_pnl - 12.0).abs() < 1e-9,
        "realized pnl should be 12.0"
    );
    assert!(
        closed_at.is_none(),
        "partial close should not close position"
    );

    let trade = load_latest_trade(&state, &cfg).await;

    assert_eq!(trade.symbol, "BTCUSDT".to_string());
    assert_eq!(trade.side, "LONG".to_string());
    assert!((decimal_to_f64(&trade.quantity) - 1.25).abs() < 1e-9);
    assert!((decimal_to_f64(&trade.realized_pnl) - 12.0).abs() < 1e-9);
    assert!((decimal_to_f64(&trade.fees) - 0.5).abs() < 1e-9);
}

#[tokio::test]
async fn test_ws_order_update_does_not_rollback_terminal_status() {
    let (state, cfg) = test_state_and_cfg().await;
    let ts = 1_700_300_000_i64;

    insert_test_order(
        &state,
        &cfg,
        "ord_terminal_1",
        "445566",
        "nfx_terminal_1",
        "BTCUSDT",
        "BUY",
        "LONG",
        "market",
        "filled",
        100.0,
        1.0,
        1.0,
        100.0,
        false,
        "IOC",
        ts,
        ts,
        Some(ts),
    )
    .await;

    let ev = BinanceOrderTradeUpdateEvent {
        event_type: "ORDER_TRADE_UPDATE".to_string(),
        event_time: ts + 10,
        order: BinanceOrderPayload {
            symbol: "BTCUSDT".to_string(),
            side: "BUY".to_string(),
            order_type: "MARKET".to_string(),
            order_status: "NEW".to_string(),
            execution_type: "NEW".to_string(),
            order_id: 445566,
            trade_id: 0,
            client_order_id: "nfx_terminal_1".to_string(),
            orig_qty: "1".to_string(),
            cum_qty: "1".to_string(),
            last_fill_price: "100".to_string(),
            last_fill_qty: "0".to_string(),
            fee: "0".to_string(),
            fee_asset: "USDT".to_string(),
            realized_pnl: "0".to_string(),
            reduce_only: false,
            trade_time: ts + 10,
        },
    };

    apply_order_trade_update_event(&state, &cfg, &ev, ts + 10)
        .await
        .expect("apply ws rollback event");

    let row = load_order_by_id(&state, "ord_terminal_1").await;

    assert_eq!(row.status, "filled".to_string());
    assert_eq!(crate::time::opt_dt_to_ts(row.closed_at), Some(ts));
}

#[tokio::test]
async fn test_ws_fill_idempotency_no_duplicate_compensation() {
    let (state, cfg) = test_state_and_cfg().await;
    let ts = 1_700_400_000_i64;

    insert_test_position(
        &state,
        &cfg,
        "pos_ws_idem_1",
        "BTCUSDT",
        "LONG",
        2.0,
        100.0,
        100.0,
        70.0,
        5,
        0.0,
        0.0,
        "open",
        ts,
        None,
        ts,
        ts,
    )
    .await;

    let ev = BinanceOrderTradeUpdateEvent {
        event_type: "ORDER_TRADE_UPDATE".to_string(),
        event_time: ts + 20,
        order: BinanceOrderPayload {
            symbol: "BTCUSDT".to_string(),
            side: "SELL".to_string(),
            order_type: "MARKET".to_string(),
            order_status: "PARTIALLY_FILLED".to_string(),
            execution_type: "TRADE".to_string(),
            order_id: 556677,
            trade_id: 9988,
            client_order_id: "nfx_ws_idem_1".to_string(),
            orig_qty: "2".to_string(),
            cum_qty: "1".to_string(),
            last_fill_price: "110".to_string(),
            last_fill_qty: "1".to_string(),
            fee: "0.2".to_string(),
            fee_asset: "USDT".to_string(),
            realized_pnl: "10".to_string(),
            reduce_only: true,
            trade_time: ts + 20,
        },
    };

    apply_order_trade_update_event(&state, &cfg, &ev, ts + 20)
        .await
        .expect("apply ws trade first time");
    apply_order_trade_update_event(&state, &cfg, &ev, ts + 21)
        .await
        .expect("apply ws trade second time");

    let fills_count = count_order_fills_by_exchange_trade_id(&state, &cfg, "ws-556677-9988").await;
    assert_eq!(fills_count, 1_u64);

    let trades_count = count_trades_by_symbol(&state, &cfg, "BTCUSDT").await;
    assert_eq!(trades_count, 1_u64);

    let pos = load_position_by_id(&state, "pos_ws_idem_1").await;

    assert!((decimal_to_f64(&pos.quantity) - 1.0).abs() < 1e-9);
    assert_eq!(pos.status, "open".to_string());
    assert!((decimal_to_f64(&pos.realized_pnl) - 9.8).abs() < 1e-9);
}

#[tokio::test]
async fn test_execution_intent_finalization_status_matrix() {
    let (state, cfg) = test_state_and_cfg().await;
    let ts = 1_700_500_000_i64;

    let cases = vec![
        (
            "intent_can_1",
            "open:trader_test_1:BTCUSDT:LONG:901",
            "ex_can_901",
            "canceled",
        ),
        (
            "intent_rej_1",
            "open:trader_test_1:ETHUSDT:SHORT:902",
            "ex_rej_902",
            "rejected",
        ),
        (
            "intent_exp_1",
            "open:trader_test_1:SOLUSDT:LONG:903",
            "ex_exp_903",
            "expired",
        ),
    ];

    for (id, key, ex_order_id, terminal_status) in &cases {
        insert_test_execution_intent(
            &state,
            &cfg,
            id,
            key,
            "BTCUSDT",
            "LONG",
            "open",
            "submitted",
            ex_order_id,
            "{}",
            ts,
            ts,
        )
        .await;

        finalize_execution_intent_for_exchange_order(
            &state,
            &cfg,
            ex_order_id,
            terminal_status,
            ts + 1,
        )
        .await
        .expect("finalize execution intent");

        let row = load_execution_intent_by_exchange_order_id(&state, &cfg, ex_order_id).await;
        assert_eq!(row.status, (*terminal_status).to_string());
    }
}

#[tokio::test]
async fn test_ws_terminal_closed_at_monotonic_under_out_of_order_updates() {
    let (state, cfg) = test_state_and_cfg().await;
    let ts = 1_700_600_000_i64;

    let ev_terminal = BinanceOrderTradeUpdateEvent {
        event_type: "ORDER_TRADE_UPDATE".to_string(),
        event_time: ts + 90,
        order: BinanceOrderPayload {
            symbol: "BTCUSDT".to_string(),
            side: "BUY".to_string(),
            order_type: "MARKET".to_string(),
            order_status: "FILLED".to_string(),
            execution_type: "TRADE".to_string(),
            order_id: 778899,
            trade_id: 12345,
            client_order_id: "nfx_monotonic_1".to_string(),
            orig_qty: "1".to_string(),
            cum_qty: "1".to_string(),
            last_fill_price: "101".to_string(),
            last_fill_qty: "1".to_string(),
            fee: "0.1".to_string(),
            fee_asset: "USDT".to_string(),
            realized_pnl: "0".to_string(),
            reduce_only: false,
            trade_time: ts + 90,
        },
    };

    apply_order_trade_update_event(&state, &cfg, &ev_terminal, ts + 90)
        .await
        .expect("apply terminal ws event");

    let ev_out_of_order_old = BinanceOrderTradeUpdateEvent {
        event_type: "ORDER_TRADE_UPDATE".to_string(),
        event_time: ts + 30,
        order: BinanceOrderPayload {
            symbol: "BTCUSDT".to_string(),
            side: "BUY".to_string(),
            order_type: "MARKET".to_string(),
            order_status: "NEW".to_string(),
            execution_type: "NEW".to_string(),
            order_id: 778899,
            trade_id: 0,
            client_order_id: "nfx_monotonic_1".to_string(),
            orig_qty: "1".to_string(),
            cum_qty: "0".to_string(),
            last_fill_price: "0".to_string(),
            last_fill_qty: "0".to_string(),
            fee: "0".to_string(),
            fee_asset: "USDT".to_string(),
            realized_pnl: "0".to_string(),
            reduce_only: false,
            trade_time: ts + 30,
        },
    };

    apply_order_trade_update_event(&state, &cfg, &ev_out_of_order_old, ts + 30)
        .await
        .expect("apply out-of-order ws event");

    let row = load_order_by_exchange_order_id(&state, &cfg, "778899").await;

    assert_eq!(row.status, "filled".to_string());
    assert_eq!(crate::time::opt_dt_to_ts(row.closed_at), Some(ts + 90));
}

#[derive(Clone, Debug, Default)]
struct FakeLiveExchangeAdapter {
    open_orders: Arc<Mutex<Vec<ExchangeOpenOrder>>>,
    order_details: Arc<Mutex<HashMap<String, ExchangeOrderDetail>>>,
    place_order_script: Arc<Mutex<Vec<Result<PlaceOrderResponse, AppError>>>>,
    cancel_order_script:
        Arc<Mutex<Vec<Result<crate::clients::exchanges::CancelOrderResponse, AppError>>>>,
    symbol_constraints: Arc<Mutex<HashMap<String, ExchangeSymbolConstraints>>>,
    symbol_setting_calls: Arc<Mutex<Vec<(String, i64, String)>>>,
}

impl FakeLiveExchangeAdapter {
    async fn set_open_orders(&self, rows: Vec<ExchangeOpenOrder>) {
        let mut guard = self.open_orders.lock().await;
        *guard = rows;
    }

    async fn set_order_detail(&self, order_id: &str, row: ExchangeOrderDetail) {
        let mut guard = self.order_details.lock().await;
        guard.insert(order_id.trim().to_string(), row);
    }

    async fn enqueue_place_order_result(&self, result: Result<PlaceOrderResponse, AppError>) {
        let mut guard = self.place_order_script.lock().await;
        guard.push(result);
    }

    async fn enqueue_cancel_order_result(
        &self,
        result: Result<crate::clients::exchanges::CancelOrderResponse, AppError>,
    ) {
        let mut guard = self.cancel_order_script.lock().await;
        guard.push(result);
    }

    async fn set_symbol_constraints(&self, symbol: &str, constraints: ExchangeSymbolConstraints) {
        let mut guard = self.symbol_constraints.lock().await;
        guard.insert(symbol.trim().to_uppercase(), constraints);
    }
}

#[async_trait::async_trait]
impl LiveExchangeAdapter for FakeLiveExchangeAdapter {
    fn exchange_type(&self) -> &'static str {
        "fake"
    }

    async fn ping(&self) -> Result<(), AppError> {
        Ok(())
    }

    async fn get_price(&self, _symbol: &str) -> Result<f64, AppError> {
        Ok(100.0)
    }

    async fn place_order(
        &self,
        _req: PlaceOrderRequest,
    ) -> Result<crate::clients::exchanges::PlaceOrderResponse, AppError> {
        let mut guard = self.place_order_script.lock().await;
        if guard.is_empty() {
            return Err(AppError::InvalidExchangeConfig(
                "fake adapter has no scripted place_order result".to_string(),
            ));
        }
        guard.remove(0)
    }

    async fn cancel_order(
        &self,
        _symbol: &str,
        _order_id: &str,
    ) -> Result<crate::clients::exchanges::CancelOrderResponse, AppError> {
        let mut guard = self.cancel_order_script.lock().await;
        if guard.is_empty() {
            return Err(AppError::InvalidExchangeConfig(
                "fake adapter has no scripted cancel_order result".to_string(),
            ));
        }
        guard.remove(0)
    }

    async fn get_balances(&self) -> Result<Vec<ExchangeBalance>, AppError> {
        Ok(vec![])
    }

    async fn get_positions(&self) -> Result<Vec<ExchangePosition>, AppError> {
        Ok(vec![])
    }

    async fn get_open_orders(
        &self,
        symbol: Option<&str>,
    ) -> Result<Vec<ExchangeOpenOrder>, AppError> {
        let guard = self.open_orders.lock().await;
        let mut rows = guard.clone();
        if let Some(sym) = symbol {
            let sym = sym.trim().to_uppercase();
            rows.retain(|r| r.symbol.trim().eq_ignore_ascii_case(&sym));
        }
        Ok(rows)
    }

    async fn get_order(
        &self,
        _symbol: &str,
        order_id: &str,
    ) -> Result<ExchangeOrderDetail, AppError> {
        let guard = self.order_details.lock().await;
        guard
            .get(order_id.trim())
            .cloned()
            .ok_or_else(|| AppError::InvalidExchangeConfig("order not found in fake".to_string()))
    }

    async fn get_order_fills(
        &self,
        _symbol: &str,
        _order_id: &str,
    ) -> Result<Vec<crate::clients::exchanges::ExchangeTradeFill>, AppError> {
        Ok(vec![])
    }

    async fn get_symbol_constraints(
        &self,
        symbol: &str,
    ) -> Result<ExchangeSymbolConstraints, AppError> {
        let guard = self.symbol_constraints.lock().await;
        if let Some(constraints) = guard.get(&symbol.trim().to_uppercase()) {
            return Ok(constraints.clone());
        }

        Ok(ExchangeSymbolConstraints {
            symbol: symbol.trim().to_uppercase(),
            base_asset: "BTC".to_string(),
            quote_asset: "USDT".to_string(),
            min_qty: 0.001,
            max_qty: 1000.0,
            step_size: 0.001,
            min_notional: 5.0,
            tick_size: 0.1,
        })
    }

    async fn ensure_symbol_settings(
        &self,
        symbol: &str,
        leverage: i64,
        margin_mode: ExchangeMarginMode,
    ) -> Result<(), AppError> {
        let mut guard = self.symbol_setting_calls.lock().await;
        guard.push((
            symbol.trim().to_uppercase(),
            leverage,
            match margin_mode {
                ExchangeMarginMode::Cross => "cross",
                ExchangeMarginMode::Isolated => "isolated",
            }
            .to_string(),
        ));
        Ok(())
    }

    async fn start_user_stream(&self) -> Result<String, AppError> {
        Err(AppError::InvalidExchangeConfig(
            "fake adapter does not support user stream".to_string(),
        ))
    }

    async fn keepalive_user_stream(&self, _listen_key: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn close_user_stream(&self, _listen_key: &str) -> Result<(), AppError> {
        Ok(())
    }

    fn user_stream_ws_url(&self, _listen_key: &str) -> Result<String, AppError> {
        Err(AppError::InvalidExchangeConfig(
            "fake adapter does not support user stream".to_string(),
        ))
    }
}

#[tokio::test]
async fn test_sync_open_orders_consistency_with_partial_and_metadata_updates() {
    let (state, cfg) = test_state_and_cfg().await;
    let ts = 1_700_700_000_i64;

    insert_test_order(
        &state,
        &cfg,
        "ord_sync_open_1",
        "99001",
        "cli_sync_open_1",
        "BTCUSDT",
        "BUY",
        "LONG",
        "market",
        "partially_filled",
        100.0,
        1.0,
        0.4,
        100.0,
        false,
        "",
        ts,
        ts,
        None,
    )
    .await;

    let adapter = FakeLiveExchangeAdapter::default();
    adapter
        .set_open_orders(vec![
            ExchangeOpenOrder {
                order_id: "99001".to_string(),
                client_order_id: "cli_sync_open_1".to_string(),
                symbol: "BTCUSDT".to_string(),
                side: "SELL".to_string(),
                position_side: "SHORT".to_string(),
                reduce_only: true,
                order_type: "MARKET".to_string(),
                status: "NEW".to_string(),
                price: 101.0,
                orig_qty: 1.0,
                executed_qty: 0.4,
                update_time: ts + 1,
            },
            ExchangeOpenOrder {
                order_id: "99002".to_string(),
                client_order_id: "cli_sync_open_2".to_string(),
                symbol: "ETHUSDT".to_string(),
                side: "BUY".to_string(),
                position_side: "LONG".to_string(),
                reduce_only: false,
                order_type: "LIMIT".to_string(),
                status: "NEW".to_string(),
                price: 2500.0,
                orig_qty: 2.0,
                executed_qty: 0.0,
                update_time: ts + 1,
            },
        ])
        .await;

    sync_open_orders_from_exchange(&state, &cfg, &adapter, ts + 2)
        .await
        .expect("sync open orders");

    let existing = load_order_by_id(&state, "ord_sync_open_1").await;

    assert_eq!(existing.status, "partially_filled".to_string());
    assert_eq!(existing.side, "SELL".to_string());
    assert_eq!(existing.position_side, "SHORT".to_string());
    assert_eq!(existing.reduce_only, 1);

    let inserted = load_order_by_exchange_order_id(&state, &cfg, "99002").await;

    assert_eq!(inserted.status, "new".to_string());
    assert_eq!(inserted.side, "BUY".to_string());
    assert_eq!(inserted.position_side, "LONG".to_string());
    assert_eq!(inserted.reduce_only, 0);
}

#[tokio::test]
async fn test_preflight_live_symbols_configures_margin_leverage_and_validates_constraints() {
    let (_, mut cfg) = test_state_and_cfg().await;
    cfg.is_cross_margin = false;
    cfg.symbols_config.clear(); // Clear so that it tests default leverage resolution

    let adapter = FakeLiveExchangeAdapter::default();

    preflight_live_symbols(
        &adapter,
        &cfg,
        &["BTCUSDT".to_string(), "SOLUSDT".to_string()],
    )
    .await
    .expect("preflight live symbols");

    let calls = adapter.symbol_setting_calls.lock().await.clone();
    assert_eq!(
        calls,
        vec![
            ("BTCUSDT".to_string(), 5, "isolated".to_string()),
            ("SOLUSDT".to_string(), 5, "isolated".to_string()),
        ]
    );
}

#[tokio::test]
async fn test_preflight_live_symbols_configures_margin_leverage_from_symbols_config() {
    let (_, mut cfg) = test_state_and_cfg().await;
    cfg.is_cross_margin = false;
    // Set explicit leverages in symbols_config
    cfg.symbols_config = vec![
        SymbolConfig {
            symbol: "BTCUSDT".to_string(),
            leverage: 12,
            min_cost: None,
            max_cost: None,
            fixed_cost: None,
        },
        SymbolConfig {
            symbol: "SOLUSDT".to_string(),
            leverage: 8,
            min_cost: None,
            max_cost: None,
            fixed_cost: None,
        },
    ];

    let adapter = FakeLiveExchangeAdapter::default();

    preflight_live_symbols(
        &adapter,
        &cfg,
        &["BTCUSDT".to_string(), "SOLUSDT".to_string()],
    )
    .await
    .expect("preflight live symbols");

    let calls = adapter.symbol_setting_calls.lock().await.clone();
    assert_eq!(
        calls,
        vec![
            ("BTCUSDT".to_string(), 12, "isolated".to_string()),
            ("SOLUSDT".to_string(), 8, "isolated".to_string()),
        ]
    );
}

#[tokio::test]
async fn test_preflight_live_symbols_rejects_incomplete_constraints() {
    let (_, cfg) = test_state_and_cfg().await;
    let adapter = FakeLiveExchangeAdapter::default();
    adapter
        .set_symbol_constraints(
            "BTCUSDT",
            ExchangeSymbolConstraints {
                symbol: "BTCUSDT".to_string(),
                base_asset: "BTC".to_string(),
                quote_asset: "USDT".to_string(),
                min_qty: 0.0,
                max_qty: 1000.0,
                step_size: 0.001,
                min_notional: 5.0,
                tick_size: 0.1,
            },
        )
        .await;

    let err = preflight_live_symbols(&adapter, &cfg, &["BTCUSDT".to_string()])
        .await
        .expect_err("invalid constraints should fail live preflight");

    assert!(err.to_string().contains("invalid symbol constraints"));
}

#[tokio::test]
async fn test_sync_terminal_orders_consistency_updates_order_and_intent() {
    let (state, cfg) = test_state_and_cfg().await;
    let ts = 1_700_800_000_i64;

    insert_test_order(
        &state,
        &cfg,
        "ord_sync_terminal_1",
        "88001",
        "cli_sync_terminal_1",
        "ETHUSDT",
        "SELL",
        "SHORT",
        "market",
        "open",
        2450.0,
        1.0,
        0.0,
        0.0,
        false,
        "",
        ts,
        ts,
        None,
    )
    .await;

    insert_test_execution_intent(
        &state,
        &cfg,
        "intent_sync_terminal_1",
        "open:trader_test_1:ETHUSDT:SHORT:14567",
        "ETHUSDT",
        "SHORT",
        "open",
        "submitted",
        "88001",
        "{}",
        ts,
        ts,
    )
    .await;

    let adapter = FakeLiveExchangeAdapter::default();
    adapter
        .set_order_detail(
            "88001",
            ExchangeOrderDetail {
                order_id: "88001".to_string(),
                client_order_id: "cli_sync_terminal_1".to_string(),
                symbol: "ETHUSDT".to_string(),
                side: "SELL".to_string(),
                position_side: "SHORT".to_string(),
                reduce_only: true,
                order_type: "MARKET".to_string(),
                status: "FILLED".to_string(),
                price: 2500.0,
                orig_qty: 1.0,
                executed_qty: 1.0,
                update_time: ts + 10,
            },
        )
        .await;

    sync_terminal_orders_from_exchange(&state, &cfg, &adapter, ts + 10)
        .await
        .expect("sync terminal orders");

    let order = load_order_by_id(&state, "ord_sync_terminal_1").await;

    assert_eq!(order.status, "filled".to_string());
    assert_eq!(order.side, "SELL".to_string());
    assert_eq!(order.position_side, "SHORT".to_string());
    assert_eq!(order.reduce_only, 1);
    assert!((decimal_to_f64(&order.filled_quantity) - 1.0).abs() < 1e-9);
    assert_eq!(crate::time::opt_dt_to_ts(order.closed_at), Some(ts + 10));

    let intent = load_execution_intent_by_exchange_order_id(&state, &cfg, "88001").await;

    assert_eq!(intent.status, "filled".to_string());
}

#[tokio::test]
async fn test_reconcile_stale_submitted_intent_to_filled_by_exchange_detail() {
    let (state, cfg) = test_state_and_cfg().await;
    let ts = 1_700_900_000_i64;

    insert_test_execution_intent(
        &state,
        &cfg,
        "intent_reconcile_fill_1",
        "open:trader_test_1:BTCUSDT:LONG:2001",
        "BTCUSDT",
        "LONG",
        "open",
        "submitted",
        "ex_reconcile_fill_1",
        "{}",
        ts - 600,
        ts - 600,
    )
    .await;

    let adapter = FakeLiveExchangeAdapter::default();
    adapter
        .set_order_detail(
            "ex_reconcile_fill_1",
            ExchangeOrderDetail {
                order_id: "ex_reconcile_fill_1".to_string(),
                client_order_id: "cli_reconcile_fill_1".to_string(),
                symbol: "BTCUSDT".to_string(),
                side: "BUY".to_string(),
                position_side: "LONG".to_string(),
                reduce_only: false,
                order_type: "MARKET".to_string(),
                status: "FILLED".to_string(),
                price: 101.0,
                orig_qty: 1.0,
                executed_qty: 1.0,
                update_time: ts,
            },
        )
        .await;

    reconcile_stale_submitted_execution_intents(&state, &cfg, &adapter, ts, 300)
        .await
        .expect("reconcile stale submitted intents");

    let row = load_execution_intent_by_exchange_order_id(&state, &cfg, "ex_reconcile_fill_1").await;

    assert_eq!(row.status, "filled".to_string());
}

#[tokio::test]
async fn test_reconcile_stale_submitted_intent_to_expired_by_exchange_detail() {
    let (state, cfg) = test_state_and_cfg().await;
    let ts = 1_701_000_000_i64;

    insert_test_execution_intent(
        &state,
        &cfg,
        "intent_reconcile_exp_1",
        "open:trader_test_1:ETHUSDT:SHORT:2002",
        "ETHUSDT",
        "SHORT",
        "open",
        "submitted",
        "ex_reconcile_exp_1",
        "{}",
        ts - 600,
        ts - 600,
    )
    .await;

    let adapter = FakeLiveExchangeAdapter::default();
    adapter
        .set_order_detail(
            "ex_reconcile_exp_1",
            ExchangeOrderDetail {
                order_id: "ex_reconcile_exp_1".to_string(),
                client_order_id: "cli_reconcile_exp_1".to_string(),
                symbol: "ETHUSDT".to_string(),
                side: "SELL".to_string(),
                position_side: "SHORT".to_string(),
                reduce_only: false,
                order_type: "LIMIT".to_string(),
                status: "EXPIRED".to_string(),
                price: 2500.0,
                orig_qty: 1.0,
                executed_qty: 0.0,
                update_time: ts,
            },
        )
        .await;

    reconcile_stale_submitted_execution_intents(&state, &cfg, &adapter, ts, 300)
        .await
        .expect("reconcile stale submitted intents to expired");

    let row = load_execution_intent_by_exchange_order_id(&state, &cfg, "ex_reconcile_exp_1").await;

    assert_eq!(row.status, "expired".to_string());
}

#[tokio::test]
async fn test_limit_first_open_prefers_limit_when_filled() {
    let adapter = FakeLiveExchangeAdapter::default();
    let constraints = ExchangeSymbolConstraints {
        symbol: "BTCUSDT".to_string(),
        base_asset: "BTC".to_string(),
        quote_asset: "USDT".to_string(),
        min_qty: 0.001,
        max_qty: 1000.0,
        step_size: 0.001,
        min_notional: 5.0,
        tick_size: 0.1,
    };

    adapter
        .enqueue_place_order_result(Ok(PlaceOrderResponse {
            order_id: "lim-1".to_string(),
            client_order_id: "cli-lim-1".to_string(),
            symbol: "BTCUSDT".to_string(),
            side: "BUY".to_string(),
            position_side: "LONG".to_string(),
            reduce_only: false,
            status: "FILLED".to_string(),
            order_type: "LIMIT".to_string(),
            price: 100.0,
            orig_qty: 1.0,
            executed_qty: 1.0,
            update_time: 1_700_000_001,
        }))
        .await;

    let resp = place_live_open_order_limit_first(
        &adapter,
        "BTCUSDT",
        "LONG",
        1.0,
        100.0,
        &constraints,
        ExchangeMarginMode::Cross,
    )
    .await
    .expect("limit-first should use filled limit order");

    assert_eq!(resp.order_id, "lim-1".to_string());
    assert_eq!(resp.order_type, "LIMIT".to_string());
}

#[tokio::test]
async fn test_limit_first_open_falls_back_to_market_when_limit_fails() {
    let adapter = FakeLiveExchangeAdapter::default();
    let constraints = ExchangeSymbolConstraints {
        symbol: "ETHUSDT".to_string(),
        base_asset: "ETH".to_string(),
        quote_asset: "USDT".to_string(),
        min_qty: 0.001,
        max_qty: 1000.0,
        step_size: 0.001,
        min_notional: 5.0,
        tick_size: 0.01,
    };

    adapter
        .enqueue_place_order_result(Err(AppError::ExchangeApi {
            status: 400,
            code: -1013,
            message: "price filter".to_string(),
        }))
        .await;

    adapter
        .enqueue_place_order_result(Ok(PlaceOrderResponse {
            order_id: "mkt-1".to_string(),
            client_order_id: "cli-mkt-1".to_string(),
            symbol: "ETHUSDT".to_string(),
            side: "SELL".to_string(),
            position_side: "SHORT".to_string(),
            reduce_only: false,
            status: "FILLED".to_string(),
            order_type: "MARKET".to_string(),
            price: 2500.0,
            orig_qty: 2.0,
            executed_qty: 2.0,
            update_time: 1_700_000_002,
        }))
        .await;

    let resp = place_live_open_order_limit_first(
        &adapter,
        "ETHUSDT",
        "SHORT",
        2.0,
        2500.0,
        &constraints,
        ExchangeMarginMode::Cross,
    )
    .await
    .expect("limit-first should fallback to market");

    assert_eq!(resp.order_id, "mkt-1".to_string());
    assert_eq!(resp.order_type, "MARKET".to_string());
}

#[tokio::test]
async fn test_cancel_replace_stale_limit_order_replaces_with_new_order() {
    let (state, cfg) = test_state_and_cfg().await;
    let ts = 1_701_100_000_i64;

    insert_test_order(
        &state,
        &cfg,
        "ord_replace_1",
        "old_ex_1",
        "old_cli_1",
        "BTCUSDT",
        "BUY",
        "LONG",
        "limit",
        "new",
        100.0,
        1.5,
        0.5,
        100.0,
        false,
        "GTC",
        ts - 600,
        ts - 600,
        None,
    )
    .await;

    let adapter = FakeLiveExchangeAdapter::default();
    adapter
        .enqueue_cancel_order_result(Ok(crate::clients::exchanges::CancelOrderResponse {
            order_id: "old_ex_1".to_string(),
            client_order_id: "old_cli_1".to_string(),
            symbol: "BTCUSDT".to_string(),
            status: "CANCELED".to_string(),
        }))
        .await;

    adapter
        .enqueue_place_order_result(Ok(PlaceOrderResponse {
            order_id: "new_ex_1".to_string(),
            client_order_id: "new_cli_1".to_string(),
            symbol: "BTCUSDT".to_string(),
            side: "BUY".to_string(),
            position_side: "LONG".to_string(),
            reduce_only: false,
            status: "FILLED".to_string(),
            order_type: "LIMIT".to_string(),
            price: 100.0,
            orig_qty: 1.0,
            executed_qty: 1.0,
            update_time: ts,
        }))
        .await;

    cancel_replace_stale_live_limit_open_orders(
        &state,
        &cfg,
        &adapter,
        ts,
        90,
        3,
        180,
        "normal",
        "test_cancel_replace_stale",
    )
    .await
    .expect("cancel-replace stale limit order");

    let old_order = load_order_by_id(&state, "ord_replace_1").await;

    assert_eq!(old_order.status, "canceled".to_string());
    assert_eq!(crate::time::opt_dt_to_ts(old_order.closed_at), Some(ts));

    let new_order = load_order_by_exchange_order_id(&state, &cfg, "new_ex_1").await;

    assert_eq!(new_order.symbol, "BTCUSDT".to_string());
    assert_eq!(new_order.side, "BUY".to_string());
    assert_eq!(new_order.position_side, "LONG".to_string());
    assert_eq!(new_order.order_type, "limit".to_string());
    assert_eq!(new_order.status, "filled".to_string());
    assert_eq!(new_order.reduce_only, 0);
}

#[tokio::test]
async fn test_cancel_replace_stale_limit_order_skips_when_not_stale() {
    let (state, cfg) = test_state_and_cfg().await;
    let ts = 1_701_200_000_i64;

    insert_test_order(
        &state,
        &cfg,
        "ord_replace_not_stale_1",
        "old_ex_not_stale_1",
        "old_cli_not_stale_1",
        "BTCUSDT",
        "BUY",
        "LONG",
        "limit",
        "new",
        100.0,
        1.0,
        0.0,
        0.0,
        false,
        "GTC",
        ts - 10,
        ts - 10,
        None,
    )
    .await;

    let adapter = FakeLiveExchangeAdapter::default();

    cancel_replace_stale_live_limit_open_orders(
        &state,
        &cfg,
        &adapter,
        ts,
        90,
        3,
        180,
        "normal",
        "test_cancel_replace_not_stale",
    )
    .await
    .expect("cancel-replace should skip non-stale order");

    let count = count_orders(&state, &cfg).await;

    assert_eq!(count, 1_u64);
}

#[tokio::test]
async fn test_cancel_replace_stale_limit_order_throttled_by_recent_attempts() {
    let (state, cfg) = test_state_and_cfg().await;
    let ts = 1_701_300_000_i64;

    insert_test_order(
        &state,
        &cfg,
        "ord_replace_throttle_1",
        "old_ex_throttle_1",
        "old_cli_throttle_1",
        "BTCUSDT",
        "BUY",
        "LONG",
        "limit",
        "new",
        100.0,
        1.0,
        0.0,
        0.0,
        false,
        "GTC",
        ts - 600,
        ts - 600,
        None,
    )
    .await;

    for idx in 0..3 {
        let intent_id = format!("intent_replace_throttle_{}", idx);
        let intent_key = format!("replace:trader_test_1:BTCUSDT:LONG:{}:{}", idx, ts / 60);
        insert_test_execution_intent(
            &state,
            &cfg,
            &intent_id,
            &intent_key,
            "BTCUSDT",
            "LONG",
            "replace-open-limit",
            "submitted",
            &format!("old_ex_throttle_seed_{}", idx),
            "{}",
            ts - 30,
            ts - 30,
        )
        .await;
    }

    let adapter = FakeLiveExchangeAdapter::default();

    cancel_replace_stale_live_limit_open_orders(
        &state,
        &cfg,
        &adapter,
        ts,
        90,
        3,
        180,
        "normal",
        "test_cancel_replace_throttled",
    )
    .await
    .expect("cancel-replace should be throttled");

    let row = load_order_by_id(&state, "ord_replace_throttle_1").await;

    assert_eq!(row.status, "new".to_string());
    assert_eq!(row.exchange_order_id, "old_ex_throttle_1".to_string());
}

#[tokio::test]
async fn test_evaluate_live_risk_levels() {
    let (_state, cfg) = test_state_and_cfg().await;
    let config = RuntimeConfig {
        live: default_live_config(),
    };

    let normal = AccountMetrics {
        total_balance: cfg.initial_balance,
        available_balance: cfg.initial_balance,
        used_margin: 0.0,
        unrealized_pnl: 0.0,
        realized_pnl: 0.0,
        margin_used_ratio: 0.50,
    };
    let soft = AccountMetrics {
        total_balance: cfg.initial_balance * 0.82,
        available_balance: cfg.initial_balance * 0.60,
        used_margin: cfg.initial_balance * 0.22,
        unrealized_pnl: 0.0,
        realized_pnl: 0.0,
        margin_used_ratio: 0.72,
    };
    let medium = AccountMetrics {
        total_balance: cfg.initial_balance * 0.80,
        available_balance: cfg.initial_balance * 0.20,
        used_margin: cfg.initial_balance * 0.60,
        unrealized_pnl: 0.0,
        realized_pnl: 0.0,
        margin_used_ratio: 0.84,
    };
    let hard = AccountMetrics {
        total_balance: cfg.initial_balance * 0.70,
        available_balance: cfg.initial_balance * 0.05,
        used_margin: cfg.initial_balance * 0.65,
        unrealized_pnl: 0.0,
        realized_pnl: 0.0,
        margin_used_ratio: 0.93,
    };

    let d_normal = evaluate_live_risk(&config, &cfg, &normal, false);
    assert_eq!(d_normal.level, LiveRiskLevel::Normal);

    let d_soft = evaluate_live_risk(&config, &cfg, &soft, false);
    assert_eq!(d_soft.level, LiveRiskLevel::Soft);

    let d_medium = evaluate_live_risk(&config, &cfg, &medium, false);
    assert_eq!(d_medium.level, LiveRiskLevel::Medium);

    let d_hard = evaluate_live_risk(&config, &cfg, &hard, false);
    assert_eq!(d_hard.level, LiveRiskLevel::Hard);

    let d_hard_override = evaluate_live_risk(&config, &cfg, &normal, true);
    assert_eq!(d_hard_override.level, LiveRiskLevel::Hard);
}

#[tokio::test]
async fn test_evaluate_live_risk_cooldown_multiplier_behavior() {
    let (_state, cfg) = test_state_and_cfg().await;
    let config = RuntimeConfig {
        live: default_live_config(),
    };
    let base = config.live.open_order_cooldown_secs as i64;
    let scaled = base
        .saturating_mul(config.live.risk_soft_open_cooldown_multiplier as i64)
        .max(1);

    let normal = AccountMetrics {
        total_balance: cfg.initial_balance,
        available_balance: cfg.initial_balance,
        used_margin: 0.0,
        unrealized_pnl: 0.0,
        realized_pnl: 0.0,
        margin_used_ratio: 0.50,
    };
    let soft = AccountMetrics {
        total_balance: cfg.initial_balance * 0.84,
        available_balance: cfg.initial_balance * 0.60,
        used_margin: cfg.initial_balance * 0.24,
        unrealized_pnl: 0.0,
        realized_pnl: 0.0,
        margin_used_ratio: 0.71,
    };
    let medium = AccountMetrics {
        total_balance: cfg.initial_balance * 0.79,
        available_balance: cfg.initial_balance * 0.20,
        used_margin: cfg.initial_balance * 0.59,
        unrealized_pnl: 0.0,
        realized_pnl: 0.0,
        margin_used_ratio: 0.83,
    };
    let hard = AccountMetrics {
        total_balance: cfg.initial_balance * 0.69,
        available_balance: cfg.initial_balance * 0.05,
        used_margin: cfg.initial_balance * 0.64,
        unrealized_pnl: 0.0,
        realized_pnl: 0.0,
        margin_used_ratio: 0.91,
    };

    let d_normal = evaluate_live_risk(&config, &cfg, &normal, false);
    assert_eq!(d_normal.open_order_cooldown_secs, base.max(1));

    let d_soft = evaluate_live_risk(&config, &cfg, &soft, false);
    assert_eq!(d_soft.open_order_cooldown_secs, scaled);

    let d_medium = evaluate_live_risk(&config, &cfg, &medium, false);
    assert_eq!(d_medium.open_order_cooldown_secs, scaled);

    let d_hard = evaluate_live_risk(&config, &cfg, &hard, false);
    assert_eq!(d_hard.open_order_cooldown_secs, base.max(1));
}
