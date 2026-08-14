//! Runtime trading orchestration.
//!
//! This module owns engine lifecycle, account simulation, persistence updates, and
//! runtime event handling. Outbound AI, market-data, and exchange protocol code
//! lives under `crate::clients`.

pub mod account_sim;
pub mod ai_decision;
pub mod binance_events;
pub mod config_loaders;
pub mod data_context;
pub mod db_utils;
pub mod engine;
pub mod events;
pub mod execution_live;
pub mod execution_live_limit;
pub mod execution_sim;
pub mod fixed_tpsl;
pub mod market;
pub mod market_seed;
pub mod models;
pub mod prompt_assemble;
pub mod prompt_data;
pub mod service;

#[cfg(test)]
pub mod test_support;

#[cfg(test)]
mod config_loaders_tests;
