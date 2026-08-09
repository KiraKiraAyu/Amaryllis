use async_trait::async_trait;

use crate::{
    clients::{
        aster::AsterAdapter, binance::BinanceFuturesAdapter, bitget::BitgetFuturesAdapter,
        hyperliquid::HyperliquidAdapter, okx::OkxFuturesAdapter,
    },
    error::AppError,
};

use super::types::{
    CancelOrderResponse, ExchangeBalance, ExchangeCredentials, ExchangeMarginMode,
    ExchangeOpenOrder, ExchangeOrderDetail, ExchangePosition, ExchangeSymbolConstraints,
    ExchangeTradeFill, PlaceConditionalOrderRequest, PlaceOrderRequest, PlaceOrderResponse,
};
use super::user_stream::ExchangeUserStreamSession;

#[async_trait]
pub trait LiveExchangeAdapter: Send + Sync {
    fn exchange_type(&self) -> &'static str;
    async fn ping(&self) -> Result<(), AppError>;
    async fn get_price(&self, symbol: &str) -> Result<f64, AppError>;
    async fn place_order(&self, req: PlaceOrderRequest) -> Result<PlaceOrderResponse, AppError>;
    async fn place_conditional_order(
        &self,
        _req: PlaceConditionalOrderRequest,
    ) -> Result<PlaceOrderResponse, AppError> {
        Err(AppError::UnsupportedExchange(format!(
            "{} does not support exchange-hosted conditional orders",
            self.exchange_type()
        )))
    }
    async fn cancel_order(
        &self,
        symbol: &str,
        order_id: &str,
    ) -> Result<CancelOrderResponse, AppError>;
    async fn get_balances(&self) -> Result<Vec<ExchangeBalance>, AppError>;
    async fn get_positions(&self) -> Result<Vec<ExchangePosition>, AppError>;
    async fn get_open_orders(
        &self,
        symbol: Option<&str>,
    ) -> Result<Vec<ExchangeOpenOrder>, AppError>;
    async fn get_order(
        &self,
        symbol: &str,
        order_id: &str,
    ) -> Result<ExchangeOrderDetail, AppError>;
    async fn get_order_fills(
        &self,
        symbol: &str,
        order_id: &str,
    ) -> Result<Vec<ExchangeTradeFill>, AppError>;
    async fn get_symbol_constraints(
        &self,
        symbol: &str,
    ) -> Result<ExchangeSymbolConstraints, AppError>;
    async fn ensure_symbol_settings(
        &self,
        _symbol: &str,
        _leverage: i64,
        _margin_mode: ExchangeMarginMode,
    ) -> Result<(), AppError> {
        Ok(())
    }
    async fn start_user_stream(&self) -> Result<String, AppError>;
    async fn keepalive_user_stream(&self, listen_key: &str) -> Result<(), AppError>;
    async fn close_user_stream(&self, listen_key: &str) -> Result<(), AppError>;
    fn user_stream_ws_url(&self, listen_key: &str) -> Result<String, AppError>;
    async fn user_stream_session(&self) -> Result<ExchangeUserStreamSession, AppError> {
        let listen_key = self.start_user_stream().await?;
        let ws_url = self.user_stream_ws_url(&listen_key)?;
        Ok(ExchangeUserStreamSession::listen_key(
            self.exchange_type(),
            ws_url,
            listen_key,
        ))
    }
    async fn keepalive_user_stream_session(
        &self,
        session: &ExchangeUserStreamSession,
    ) -> Result<(), AppError> {
        if let Some(listen_key) = session.listen_key.as_deref() {
            self.keepalive_user_stream(listen_key).await?;
        }
        Ok(())
    }
    async fn close_user_stream_session(
        &self,
        session: &ExchangeUserStreamSession,
    ) -> Result<(), AppError> {
        if let Some(listen_key) = session.listen_key.as_deref() {
            self.close_user_stream(listen_key).await?;
        }
        Ok(())
    }
}

pub fn create_exchange_adapter(
    exchange_type: &str,
    credentials: ExchangeCredentials,
) -> Result<Box<dyn LiveExchangeAdapter>, AppError> {
    match exchange_type.to_ascii_lowercase().as_str() {
        "binance" => Ok(Box::new(BinanceFuturesAdapter::new(credentials)?)),
        "aster" => Ok(Box::new(AsterAdapter::new(credentials)?)),
        "okx" => Ok(Box::new(OkxFuturesAdapter::new(credentials)?)),
        "bitget" => Ok(Box::new(BitgetFuturesAdapter::new(credentials)?)),
        "hyperliquid" => Ok(Box::new(HyperliquidAdapter::new(credentials)?)),
        other => Err(AppError::UnsupportedExchange(other.to_string())),
    }
}
