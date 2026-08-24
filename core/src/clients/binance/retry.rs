use std::time::Duration;

use tokio::time;
use tracing::warn;

use crate::{
    clients::exchanges::{
        ExchangeBalance, ExchangeOpenOrder, ExchangeOrderDetail, ExchangePosition,
        ExchangeSymbolConstraints, ExchangeTradeFill, LiveExchangeAdapter,
    },
    error::AppError,
};

pub async fn get_price_with_retry(
    adapter: &dyn LiveExchangeAdapter,
    symbol: &str,
) -> Result<f64, AppError> {
    match adapter.get_price(symbol).await {
        Ok(v) => Ok(v),
        Err(first_err) => {
            warn!(
                "live poll get_price retry symbol={} first_err={}",
                symbol, first_err
            );
            time::sleep(Duration::from_millis(250)).await;
            adapter.get_price(symbol).await
        }
    }
}

pub async fn get_positions_with_retry(
    adapter: &dyn LiveExchangeAdapter,
) -> Result<Vec<ExchangePosition>, AppError> {
    match adapter.get_positions().await {
        Ok(v) => Ok(v),
        Err(first_err) => {
            warn!("live poll get_positions retry first_err={}", first_err);
            time::sleep(Duration::from_millis(250)).await;
            adapter.get_positions().await
        }
    }
}

pub async fn get_balances_with_retry(
    adapter: &dyn LiveExchangeAdapter,
) -> Result<Vec<ExchangeBalance>, AppError> {
    match adapter.get_balances().await {
        Ok(v) => Ok(v),
        Err(first_err) => {
            warn!("live poll get_balances retry first_err={}", first_err);
            time::sleep(Duration::from_millis(250)).await;
            adapter.get_balances().await
        }
    }
}

pub async fn get_open_orders_with_retry(
    adapter: &dyn LiveExchangeAdapter,
) -> Result<Vec<ExchangeOpenOrder>, AppError> {
    match adapter.get_open_orders(None).await {
        Ok(v) => Ok(v),
        Err(first_err) => {
            warn!("live poll get_open_orders retry first_err={}", first_err);
            time::sleep(Duration::from_millis(250)).await;
            adapter.get_open_orders(None).await
        }
    }
}

pub async fn get_order_with_retry(
    adapter: &dyn LiveExchangeAdapter,
    symbol: &str,
    order_id: &str,
) -> Result<ExchangeOrderDetail, AppError> {
    match adapter.get_order(symbol, order_id).await {
        Ok(v) => Ok(v),
        Err(first_err) => {
            warn!(
                "live poll get_order retry symbol={} order_id={} first_err={}",
                symbol, order_id, first_err
            );
            time::sleep(Duration::from_millis(250)).await;
            adapter.get_order(symbol, order_id).await
        }
    }
}

pub async fn get_order_fills_with_retry(
    adapter: &dyn LiveExchangeAdapter,
    symbol: &str,
    order_id: &str,
) -> Result<Vec<ExchangeTradeFill>, AppError> {
    match adapter.get_order_fills(symbol, order_id).await {
        Ok(v) => Ok(v),
        Err(first_err) => {
            warn!(
                "live poll get_order_fills retry symbol={} order_id={} first_err={}",
                symbol, order_id, first_err
            );
            time::sleep(Duration::from_millis(250)).await;
            adapter.get_order_fills(symbol, order_id).await
        }
    }
}

pub async fn get_symbol_constraints_with_retry(
    adapter: &dyn LiveExchangeAdapter,
    symbol: &str,
) -> Result<ExchangeSymbolConstraints, AppError> {
    match adapter.get_symbol_constraints(symbol).await {
        Ok(v) => Ok(v),
        Err(first_err) => {
            warn!(
                "live poll get_symbol_constraints retry symbol={} first_err={}",
                symbol, first_err
            );
            time::sleep(Duration::from_millis(250)).await;
            adapter.get_symbol_constraints(symbol).await
        }
    }
}

pub async fn init_binance_user_stream(
    adapter: &dyn LiveExchangeAdapter,
) -> Result<(String, String), AppError> {
    let listen_key = adapter.start_user_stream().await?;
    let ws_url = adapter.user_stream_ws_url(&listen_key)?;
    Ok((listen_key, ws_url))
}
