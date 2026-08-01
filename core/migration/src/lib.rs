pub use sea_orm_migration::prelude::*;

mod m20260409_000001_create_app_settings;
mod m20260409_000002_create_llm_providers;
mod m20260409_000003_create_llm_models;
mod m20260409_000004_create_exchanges;
mod m20260409_000005_create_strategies;
mod m20260409_000006_create_backtest_runs;
mod m20260409_000007_create_backtest_equity;
mod m20260409_000008_create_backtest_trades;
mod m20260409_000009_create_backtest_decisions;
mod m20260409_000010_create_traders;
mod m20260409_000011_create_trader_accounts;
mod m20260409_000012_create_trader_positions;
mod m20260409_000013_create_trader_orders;
mod m20260409_000014_create_order_fills;
mod m20260409_000015_create_trader_trades;
mod m20260409_000016_create_execution_intents;
mod m20260409_000017_create_runtime_events;
mod m20260409_000018_create_runtime_alert_history;
mod m20260409_000019_create_runtime_alert_controls;
mod m20260409_000020_create_runtime_alert_delivery_log;
mod m20260409_000021_create_trader_decisions;
mod m20260409_000022_create_debates;
mod m20260409_000023_create_debate_messages;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260409_000001_create_app_settings::Migration),
            Box::new(m20260409_000002_create_llm_providers::Migration),
            Box::new(m20260409_000003_create_llm_models::Migration),
            Box::new(m20260409_000004_create_exchanges::Migration),
            Box::new(m20260409_000005_create_strategies::Migration),
            Box::new(m20260409_000006_create_backtest_runs::Migration),
            Box::new(m20260409_000007_create_backtest_equity::Migration),
            Box::new(m20260409_000008_create_backtest_trades::Migration),
            Box::new(m20260409_000009_create_backtest_decisions::Migration),
            Box::new(m20260409_000010_create_traders::Migration),
            Box::new(m20260409_000011_create_trader_accounts::Migration),
            Box::new(m20260409_000012_create_trader_positions::Migration),
            Box::new(m20260409_000013_create_trader_orders::Migration),
            Box::new(m20260409_000014_create_order_fills::Migration),
            Box::new(m20260409_000015_create_trader_trades::Migration),
            Box::new(m20260409_000016_create_execution_intents::Migration),
            Box::new(m20260409_000017_create_runtime_events::Migration),
            Box::new(m20260409_000018_create_runtime_alert_history::Migration),
            Box::new(m20260409_000019_create_runtime_alert_controls::Migration),
            Box::new(m20260409_000020_create_runtime_alert_delivery_log::Migration),
            Box::new(m20260409_000021_create_trader_decisions::Migration),
            Box::new(m20260409_000022_create_debates::Migration),
            Box::new(m20260409_000023_create_debate_messages::Migration),
        ]
    }
}
