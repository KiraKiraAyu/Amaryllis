use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use k256::ecdsa::SigningKey;
use reqwest::{Client, Method};
use serde::Deserialize;
use sha3::{Digest, Keccak256};

use crate::{
    clients::{
        exchanges::{
            CancelOrderResponse, ExchangeBalance, ExchangeConditionalOrderType,
            ExchangeCredentials, ExchangeMarginMode, ExchangeOpenOrder, ExchangeOrderDetail,
            ExchangeOrderType, ExchangePosition, ExchangeSide, ExchangeSymbolConstraints,
            ExchangeTradeFill, LiveExchangeAdapter, PlaceConditionalOrderRequest,
            PlaceOrderRequest, PlaceOrderResponse, PositionSide, TimeInForce,
        },
        outbound_http::{OutboundRequestLog, OutboundResponse, send_text},
    },
    error::AppError,
};

const ASTER_API_VERSION: &str = "/fapi/v3";
const ASTER_PUBLIC_VERSION: &str = "/fapi/v1";

#[derive(Debug, Clone)]
pub struct AsterAdapter {
    client: Client,
    user: String,
    signer: String,
    private_key: String,
    base_url: String,
    ws_base_url: String,
    chain_id: u64,
}

impl AsterAdapter {
    pub fn new(credentials: ExchangeCredentials) -> Result<Self, AppError> {
        let user = credentials
            .wallet_addr
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_string();
        if user.is_empty() {
            return Err(AppError::InvalidExchangeConfig(
                "aster main wallet address is required".to_string(),
            ));
        }

        let private_key = credentials.secret_key.trim().to_string();
        if private_key.is_empty() {
            return Err(AppError::InvalidExchangeConfig(
                "aster api wallet private key is required".to_string(),
            ));
        }

        let signer = derive_eth_address(&private_key)?;

        let (base_url, ws_base_url, chain_id) = if credentials.testnet {
            (
                "https://fapi.asterdex-testnet.com",
                "wss://fstream5.asterdex-testnet.com",
                714_u64,
            )
        } else {
            (
                "https://fapi.asterdex.com",
                "wss://fstream.asterdex.com",
                1666_u64,
            )
        };

        Ok(Self {
            client: Client::builder().build().map_err(AppError::ExchangeHttp)?,
            user: normalize_address(&user),
            signer,
            private_key,
            base_url: base_url.to_string(),
            ws_base_url: ws_base_url.to_string(),
            chain_id,
        })
    }

    async fn public_get<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        params: Vec<(&str, String)>,
    ) -> Result<T, AppError> {
        let query = build_query(params);
        let url = if query.is_empty() {
            format!("{}{}", self.base_url, path)
        } else {
            format!("{}{}?{}", self.base_url, path, query)
        };

        let resp = send_text(
            self.client.get(&url),
            OutboundRequestLog::new("exchange.aster.public", Method::GET, &url),
        )
        .await
        .map_err(AppError::ExchangeHttp)?;
        parse_json_response(resp)
    }

    async fn signed_request<T: for<'de> Deserialize<'de>>(
        &self,
        method: Method,
        path: &str,
        mut params: Vec<(&str, String)>,
    ) -> Result<T, AppError> {
        params.push(("nonce", next_nonce()));
        params.push(("user", self.user.clone()));
        params.push(("signer", self.signer.clone()));

        let query = build_query(params);
        let signature = sign_eip712(&self.private_key, &query, self.chain_id)?;
        let url = format!(
            "{}{}?{}&signature={}",
            self.base_url, path, query, signature
        );

        let resp = send_text(
            self.client.request(method.clone(), &url),
            OutboundRequestLog::new("exchange.aster.signed", method, &url),
        )
        .await
        .map_err(AppError::ExchangeHttp)?;
        parse_json_response(resp)
    }

    async fn signed_get<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        params: Vec<(&str, String)>,
    ) -> Result<T, AppError> {
        self.signed_request(Method::GET, path, params).await
    }

    async fn signed_post<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        params: Vec<(&str, String)>,
    ) -> Result<T, AppError> {
        self.signed_request(Method::POST, path, params).await
    }

    async fn signed_put<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        params: Vec<(&str, String)>,
    ) -> Result<T, AppError> {
        self.signed_request(Method::PUT, path, params).await
    }

    async fn signed_delete<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        params: Vec<(&str, String)>,
    ) -> Result<T, AppError> {
        self.signed_request(Method::DELETE, path, params).await
    }

    async fn dual_side_position(&self) -> bool {
        let result: Result<AsterPositionModeResponse, AppError> = self
            .signed_get(&format!("{ASTER_API_VERSION}/positionSide/dual"), vec![])
            .await;
        result.map(|v| v.dual_side_position).unwrap_or(false)
    }
}

#[async_trait]
impl LiveExchangeAdapter for AsterAdapter {
    fn exchange_type(&self) -> &'static str {
        "aster"
    }

    async fn ping(&self) -> Result<(), AppError> {
        let _: serde_json::Value = self
            .public_get(&format!("{ASTER_PUBLIC_VERSION}/ping"), vec![])
            .await?;
        Ok(())
    }

    async fn get_price(&self, symbol: &str) -> Result<f64, AppError> {
        let payload: AsterTickerPrice = self
            .public_get(
                &format!("{ASTER_PUBLIC_VERSION}/ticker/price"),
                vec![("symbol", symbol.trim().to_uppercase())],
            )
            .await?;
        Ok(parse_f64(&payload.price))
    }

    async fn place_order(&self, req: PlaceOrderRequest) -> Result<PlaceOrderResponse, AppError> {
        if req.quantity <= 0.0 {
            return Err(AppError::InvalidExchangeConfig(
                "quantity must be > 0".to_string(),
            ));
        }

        let symbol = req.symbol.trim().to_uppercase();
        if symbol.is_empty() {
            return Err(AppError::InvalidExchangeConfig(
                "symbol is required".to_string(),
            ));
        }

        let hedge_mode = self.dual_side_position().await;
        let mut params = vec![
            ("symbol", symbol),
            (
                "side",
                match req.side {
                    ExchangeSide::Buy => "BUY".to_string(),
                    ExchangeSide::Sell => "SELL".to_string(),
                },
            ),
            (
                "type",
                match req.order_type {
                    ExchangeOrderType::Market => "MARKET".to_string(),
                    ExchangeOrderType::Limit => "LIMIT".to_string(),
                },
            ),
            ("quantity", format_decimal(req.quantity)),
            (
                "newClientOrderId",
                req.client_order_id
                    .unwrap_or_else(|| crate::clients::short_client_order_id("quantaura_")),
            ),
        ];

        if hedge_mode {
            let ps = req
                .position_side
                .unwrap_or_else(|| inferred_position_side_for_order(req.side, req.reduce_only));
            if !matches!(ps, PositionSide::Both) {
                params.push((
                    "positionSide",
                    match ps {
                        PositionSide::Both => "BOTH",
                        PositionSide::Long => "LONG",
                        PositionSide::Short => "SHORT",
                    }
                    .to_string(),
                ));
            }
        } else if req.reduce_only {
            params.push(("reduceOnly", "true".to_string()));
        }

        if !hedge_mode && matches!(req.position_side, Some(PositionSide::Both)) {
            params.push(("positionSide", "BOTH".to_string()));
        }

        if let ExchangeOrderType::Limit = req.order_type {
            let price = req.price.ok_or_else(|| {
                AppError::InvalidExchangeConfig("limit order requires price".to_string())
            })?;
            params.push(("price", format_decimal(price)));
            params.push((
                "timeInForce",
                match req.time_in_force.unwrap_or(TimeInForce::Gtc) {
                    TimeInForce::Gtc => "GTC",
                    TimeInForce::Ioc => "IOC",
                    TimeInForce::Fok => "FOK",
                }
                .to_string(),
            ));
        }

        let payload: AsterOrderResponse = self
            .signed_post(&format!("{ASTER_API_VERSION}/order"), params)
            .await?;
        Ok(PlaceOrderResponse {
            order_id: payload.order_id.to_string(),
            client_order_id: payload.client_order_id,
            symbol: payload.symbol,
            side: payload.side,
            position_side: payload.position_side,
            reduce_only: payload.reduce_only,
            status: payload.status,
            order_type: payload.order_type,
            price: parse_f64(&payload.price),
            orig_qty: parse_f64(&payload.orig_qty),
            executed_qty: parse_f64(&payload.executed_qty),
            update_time: payload.update_time,
        })
    }

    async fn place_conditional_order(
        &self,
        req: PlaceConditionalOrderRequest,
    ) -> Result<PlaceOrderResponse, AppError> {
        if req.quantity <= 0.0 || !req.quantity.is_finite() {
            return Err(AppError::InvalidExchangeConfig(
                "conditional order quantity must be > 0".to_string(),
            ));
        }
        if req.trigger_price <= 0.0 || !req.trigger_price.is_finite() {
            return Err(AppError::InvalidExchangeConfig(
                "conditional order trigger price must be > 0".to_string(),
            ));
        }

        let symbol = req.symbol.trim().to_uppercase();
        if symbol.is_empty() {
            return Err(AppError::InvalidExchangeConfig(
                "symbol is required".to_string(),
            ));
        }

        let hedge_mode = self.dual_side_position().await;
        let mut params = vec![
            ("symbol", symbol),
            (
                "side",
                match req.side {
                    ExchangeSide::Buy => "BUY".to_string(),
                    ExchangeSide::Sell => "SELL".to_string(),
                },
            ),
            (
                "type",
                match req.conditional_type {
                    ExchangeConditionalOrderType::TakeProfitMarket => {
                        "TAKE_PROFIT_MARKET".to_string()
                    }
                    ExchangeConditionalOrderType::StopMarket => "STOP_MARKET".to_string(),
                },
            ),
            ("quantity", format_decimal(req.quantity)),
            ("stopPrice", format_decimal(req.trigger_price)),
            ("workingType", "MARK_PRICE".to_string()),
            (
                "newClientOrderId",
                req.client_order_id
                    .unwrap_or_else(|| crate::clients::short_client_order_id("tpsl_")),
            ),
        ];

        if hedge_mode {
            let position_side = req
                .position_side
                .unwrap_or_else(|| inferred_position_side_for_order(req.side, true));
            params.push((
                "positionSide",
                match position_side {
                    PositionSide::Both => "BOTH",
                    PositionSide::Long => "LONG",
                    PositionSide::Short => "SHORT",
                }
                .to_string(),
            ));
        } else if req.reduce_only {
            params.push(("reduceOnly", "true".to_string()));
        }

        let payload: AsterOrderResponse = self
            .signed_post(&format!("{ASTER_API_VERSION}/order"), params)
            .await?;
        Ok(PlaceOrderResponse {
            order_id: payload.order_id.to_string(),
            client_order_id: payload.client_order_id,
            symbol: payload.symbol,
            side: payload.side,
            position_side: payload.position_side,
            reduce_only: payload.reduce_only,
            status: payload.status,
            order_type: payload.order_type,
            price: parse_f64(&payload.price),
            orig_qty: parse_f64(&payload.orig_qty),
            executed_qty: parse_f64(&payload.executed_qty),
            update_time: payload.update_time,
        })
    }

    async fn cancel_order(
        &self,
        symbol: &str,
        order_id: &str,
    ) -> Result<CancelOrderResponse, AppError> {
        let payload: AsterOrderResponse = self
            .signed_delete(
                &format!("{ASTER_API_VERSION}/order"),
                vec![
                    ("symbol", symbol.trim().to_uppercase()),
                    ("orderId", order_id.trim().to_string()),
                ],
            )
            .await?;

        Ok(CancelOrderResponse {
            order_id: payload.order_id.to_string(),
            client_order_id: payload.client_order_id,
            symbol: payload.symbol,
            status: payload.status,
        })
    }

    async fn get_balances(&self) -> Result<Vec<ExchangeBalance>, AppError> {
        let rows: Vec<AsterBalanceRow> = self
            .signed_get(&format!("{ASTER_API_VERSION}/balance"), vec![])
            .await?;
        Ok(rows
            .into_iter()
            .map(|v| ExchangeBalance {
                asset: v.asset,
                wallet_balance: parse_f64(&v.balance),
                available_balance: parse_f64(&v.available_balance),
                unrealized_pnl: parse_f64(&v.cross_un_pnl),
            })
            .collect())
    }

    async fn get_positions(&self) -> Result<Vec<ExchangePosition>, AppError> {
        let rows: Vec<AsterPositionRiskRow> = self
            .signed_get(&format!("{ASTER_API_VERSION}/positionRisk"), vec![])
            .await?;
        Ok(rows
            .into_iter()
            .filter_map(|v| {
                let qty = parse_f64(&v.position_amt);
                if qty.abs() <= f64::EPSILON {
                    return None;
                }
                Some(ExchangePosition {
                    symbol: v.symbol,
                    position_side: v.position_side,
                    quantity: qty,
                    entry_price: parse_f64(&v.entry_price),
                    mark_price: parse_f64(&v.mark_price),
                    unrealized_pnl: parse_f64(&v.un_realized_profit),
                    leverage: v.leverage.parse::<i64>().unwrap_or(1),
                    liquidation_price: parse_f64(&v.liquidation_price),
                })
            })
            .collect())
    }

    async fn get_open_orders(
        &self,
        symbol: Option<&str>,
    ) -> Result<Vec<ExchangeOpenOrder>, AppError> {
        let mut params = vec![];
        if let Some(sym) = symbol
            && !sym.trim().is_empty()
        {
            params.push(("symbol", sym.trim().to_uppercase()));
        }

        let rows: Vec<AsterOpenOrderRow> = self
            .signed_get(&format!("{ASTER_API_VERSION}/openOrders"), params)
            .await?;
        Ok(rows
            .into_iter()
            .map(|v| ExchangeOpenOrder {
                order_id: v.order_id.to_string(),
                client_order_id: v.client_order_id,
                symbol: v.symbol,
                side: v.side,
                position_side: v.position_side,
                reduce_only: v.reduce_only,
                order_type: v.order_type,
                status: v.status,
                price: parse_f64(&v.price),
                orig_qty: parse_f64(&v.orig_qty),
                executed_qty: parse_f64(&v.executed_qty),
                update_time: v.update_time,
            })
            .collect())
    }

    async fn get_order(
        &self,
        symbol: &str,
        order_id: &str,
    ) -> Result<ExchangeOrderDetail, AppError> {
        let payload: AsterOrderResponse = self
            .signed_get(
                &format!("{ASTER_API_VERSION}/order"),
                vec![
                    ("symbol", symbol.trim().to_uppercase()),
                    ("orderId", order_id.trim().to_string()),
                ],
            )
            .await?;
        Ok(ExchangeOrderDetail {
            order_id: payload.order_id.to_string(),
            client_order_id: payload.client_order_id,
            symbol: payload.symbol,
            side: payload.side,
            position_side: payload.position_side,
            reduce_only: payload.reduce_only,
            order_type: payload.order_type,
            status: payload.status,
            price: parse_f64(&payload.price),
            orig_qty: parse_f64(&payload.orig_qty),
            executed_qty: parse_f64(&payload.executed_qty),
            update_time: payload.update_time,
        })
    }

    async fn get_order_fills(
        &self,
        symbol: &str,
        order_id: &str,
    ) -> Result<Vec<ExchangeTradeFill>, AppError> {
        let rows: Vec<AsterUserTradeRow> = self
            .signed_get(
                &format!("{ASTER_API_VERSION}/userTrades"),
                vec![
                    ("symbol", symbol.trim().to_uppercase()),
                    ("orderId", order_id.trim().to_string()),
                    ("limit", "1000".to_string()),
                ],
            )
            .await?;
        Ok(rows
            .into_iter()
            .map(|v| ExchangeTradeFill {
                trade_id: v.trade_id.to_string(),
                order_id: v.order_id.to_string(),
                symbol: v.symbol,
                side: v.side,
                price: parse_f64(&v.price),
                quantity: parse_f64(&v.qty),
                fee: parse_f64(&v.commission),
                fee_asset: v.commission_asset,
                realized_pnl: parse_f64(&v.realized_pnl),
                executed_at: v.time,
            })
            .collect())
    }

    async fn get_symbol_constraints(
        &self,
        symbol: &str,
    ) -> Result<ExchangeSymbolConstraints, AppError> {
        let symbol_upper = symbol.trim().to_uppercase();
        if symbol_upper.is_empty() {
            return Err(AppError::InvalidExchangeConfig(
                "symbol is required".to_string(),
            ));
        }

        let payload: serde_json::Value = self
            .public_get(
                &format!("{ASTER_PUBLIC_VERSION}/exchangeInfo"),
                vec![("symbol", symbol_upper.clone())],
            )
            .await?;

        parse_symbol_constraints(&payload, &symbol_upper)
    }

    async fn ensure_symbol_settings(
        &self,
        symbol: &str,
        leverage: i64,
        margin_mode: ExchangeMarginMode,
    ) -> Result<(), AppError> {
        let symbol = symbol.trim().to_uppercase();
        if symbol.is_empty() {
            return Err(AppError::InvalidExchangeConfig(
                "symbol is required".to_string(),
            ));
        }
        let margin_type = match margin_mode {
            ExchangeMarginMode::Cross => "CROSSED",
            ExchangeMarginMode::Isolated => "ISOLATED",
        };

        let margin_result: Result<serde_json::Value, AppError> = self
            .signed_post(
                &format!("{ASTER_API_VERSION}/marginType"),
                vec![
                    ("symbol", symbol.clone()),
                    ("marginType", margin_type.to_string()),
                ],
            )
            .await;
        match margin_result {
            Ok(_) => {}
            Err(err) if is_margin_type_noop(&err) => {}
            Err(err) => return Err(err),
        }

        let _: serde_json::Value = self
            .signed_post(
                &format!("{ASTER_API_VERSION}/leverage"),
                vec![("symbol", symbol), ("leverage", leverage.to_string())],
            )
            .await?;
        Ok(())
    }

    async fn start_user_stream(&self) -> Result<String, AppError> {
        let payload: serde_json::Value = self
            .signed_post(&format!("{ASTER_API_VERSION}/listenKey"), vec![])
            .await?;
        let listen_key = payload
            .get("listenKey")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AppError::InvalidExchangeConfig(
                    "missing listenKey in user stream response".to_string(),
                )
            })?;
        Ok(listen_key.to_string())
    }

    async fn keepalive_user_stream(&self, listen_key: &str) -> Result<(), AppError> {
        let _: serde_json::Value = self
            .signed_put(
                &format!("{ASTER_API_VERSION}/listenKey"),
                vec![("listenKey", listen_key.trim().to_string())],
            )
            .await?;
        Ok(())
    }

    async fn close_user_stream(&self, listen_key: &str) -> Result<(), AppError> {
        let _: serde_json::Value = self
            .signed_delete(
                &format!("{ASTER_API_VERSION}/listenKey"),
                vec![("listenKey", listen_key.trim().to_string())],
            )
            .await?;
        Ok(())
    }

    fn user_stream_ws_url(&self, listen_key: &str) -> Result<String, AppError> {
        let key = listen_key.trim();
        if key.is_empty() {
            return Err(AppError::InvalidExchangeConfig(
                "listen_key is required".to_string(),
            ));
        }
        Ok(format!("{}/ws/{}", self.ws_base_url, key))
    }
}

#[derive(Debug, Deserialize)]
struct AsterApiErrorPayload {
    #[serde(default)]
    code: i64,
    #[serde(default)]
    msg: String,
}

#[derive(Debug, Deserialize)]
struct AsterTickerPrice {
    price: String,
}

#[derive(Debug, Deserialize)]
struct AsterOrderResponse {
    #[serde(rename = "orderId")]
    order_id: i64,
    #[serde(rename = "clientOrderId")]
    client_order_id: String,
    symbol: String,
    side: String,
    #[serde(rename = "positionSide", default)]
    position_side: String,
    #[serde(rename = "reduceOnly", default)]
    reduce_only: bool,
    status: String,
    #[serde(rename = "type")]
    order_type: String,
    price: String,
    #[serde(rename = "origQty")]
    orig_qty: String,
    #[serde(rename = "executedQty")]
    executed_qty: String,
    #[serde(rename = "updateTime", default)]
    update_time: i64,
}

#[derive(Debug, Deserialize)]
struct AsterOpenOrderRow {
    #[serde(rename = "orderId")]
    order_id: i64,
    #[serde(rename = "clientOrderId")]
    client_order_id: String,
    symbol: String,
    side: String,
    #[serde(rename = "positionSide", default)]
    position_side: String,
    #[serde(rename = "reduceOnly", default)]
    reduce_only: bool,
    #[serde(rename = "type")]
    order_type: String,
    status: String,
    price: String,
    #[serde(rename = "origQty")]
    orig_qty: String,
    #[serde(rename = "executedQty")]
    executed_qty: String,
    #[serde(rename = "updateTime", default)]
    update_time: i64,
}

#[derive(Debug, Deserialize)]
struct AsterUserTradeRow {
    #[serde(rename = "id")]
    trade_id: i64,
    #[serde(rename = "orderId")]
    order_id: i64,
    symbol: String,
    side: String,
    price: String,
    qty: String,
    commission: String,
    #[serde(rename = "commissionAsset")]
    commission_asset: String,
    #[serde(rename = "realizedPnl")]
    realized_pnl: String,
    time: i64,
}

#[derive(Debug, Deserialize)]
struct AsterBalanceRow {
    asset: String,
    balance: String,
    #[serde(rename = "availableBalance")]
    available_balance: String,
    #[serde(rename = "crossUnPnl")]
    cross_un_pnl: String,
}

#[derive(Debug, Deserialize)]
struct AsterPositionRiskRow {
    symbol: String,
    #[serde(rename = "positionAmt")]
    position_amt: String,
    #[serde(rename = "entryPrice")]
    entry_price: String,
    #[serde(rename = "markPrice")]
    mark_price: String,
    #[serde(rename = "unRealizedProfit")]
    un_realized_profit: String,
    leverage: String,
    #[serde(rename = "liquidationPrice")]
    liquidation_price: String,
    #[serde(rename = "positionSide")]
    position_side: String,
}

#[derive(Debug, Deserialize)]
struct AsterPositionModeResponse {
    #[serde(rename = "dualSidePosition", default)]
    dual_side_position: bool,
}

fn parse_json_response<T: for<'de> Deserialize<'de>>(
    resp: OutboundResponse,
) -> Result<T, AppError> {
    let status = resp.status;
    let text = resp.body;

    if !status.is_success() {
        if let Ok(err_payload) = serde_json::from_str::<AsterApiErrorPayload>(&text) {
            return Err(AppError::ExchangeApi {
                status: status.as_u16(),
                code: err_payload.code,
                message: err_payload.msg,
            });
        }
        return Err(AppError::ExchangeApi {
            status: status.as_u16(),
            code: i64::from(status.as_u16()),
            message: text,
        });
    }

    serde_json::from_str::<T>(&text).map_err(AppError::ExchangeJson)
}

fn parse_symbol_constraints(
    payload: &serde_json::Value,
    requested_symbol: &str,
) -> Result<ExchangeSymbolConstraints, AppError> {
    let symbol_upper = requested_symbol.trim().to_uppercase();
    let symbols = payload
        .get("symbols")
        .and_then(|value| value.as_array())
        .ok_or_else(|| {
            AppError::InvalidExchangeConfig("missing symbols in exchangeInfo".to_string())
        })?;
    let symbol = symbols
        .iter()
        .find(|candidate| {
            candidate
                .get("symbol")
                .and_then(|value| value.as_str())
                .is_some_and(|value| value.eq_ignore_ascii_case(&symbol_upper))
        })
        .ok_or_else(|| {
            AppError::InvalidExchangeConfig(format!(
                "symbol not found in exchangeInfo: {symbol_upper}"
            ))
        })?;

    Ok(ExchangeSymbolConstraints {
        symbol: symbol
            .get("symbol")
            .and_then(|value| value.as_str())
            .unwrap_or(&symbol_upper)
            .to_string(),
        base_asset: symbol
            .get("baseAsset")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string(),
        quote_asset: symbol
            .get("quoteAsset")
            .and_then(|value| value.as_str())
            .unwrap_or("USDT")
            .to_string(),
        min_qty: filter_value(symbol, "LOT_SIZE", "minQty"),
        max_qty: filter_value(symbol, "LOT_SIZE", "maxQty"),
        step_size: filter_value(symbol, "LOT_SIZE", "stepSize"),
        min_notional: filter_value(symbol, "MIN_NOTIONAL", "notional").max(filter_value(
            symbol,
            "MIN_NOTIONAL",
            "minNotional",
        )),
        tick_size: filter_value(symbol, "PRICE_FILTER", "tickSize"),
    })
}

fn filter_value(symbol: &serde_json::Value, filter_type: &str, field: &str) -> f64 {
    symbol
        .get("filters")
        .and_then(|v| v.as_array())
        .and_then(|filters| {
            filters.iter().find_map(|f| {
                if f.get("filterType").and_then(|x| x.as_str()) == Some(filter_type) {
                    f.get(field).and_then(|x| x.as_str())
                } else {
                    None
                }
            })
        })
        .map(parse_f64)
        .unwrap_or(0.0)
}

fn is_margin_type_noop(err: &AppError) -> bool {
    matches!(err, AppError::ExchangeApi { code: -4046, .. })
        || matches!(
            err,
            AppError::ExchangeApi { message, .. }
                if message.to_ascii_lowercase().contains("no need to change margin type")
        )
}

fn inferred_position_side_for_order(side: ExchangeSide, reduce_only: bool) -> PositionSide {
    match (side, reduce_only) {
        (ExchangeSide::Sell, false) | (ExchangeSide::Buy, true) => PositionSide::Short,
        _ => PositionSide::Long,
    }
}

fn build_query(params: Vec<(&str, String)>) -> String {
    params
        .into_iter()
        .map(|(k, v)| format!("{}={}", encode_component(k), encode_component(&v)))
        .collect::<Vec<_>>()
        .join("&")
}

fn encode_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(b));
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{:02X}", b));
            }
        }
    }
    out
}

fn format_decimal(v: f64) -> String {
    let mut s = format!("{:.10}", v);
    while s.contains('.') && s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    if s.is_empty() { "0".to_string() } else { s }
}

fn parse_f64(v: &str) -> f64 {
    v.parse::<f64>().unwrap_or(0.0)
}

fn normalize_address(address: &str) -> String {
    format!(
        "0x{}",
        address.trim().trim_start_matches("0x").to_ascii_lowercase()
    )
}

fn next_nonce() -> String {
    static LAST: AtomicU64 = AtomicU64::new(0);
    let now_micros = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_micros() as u64)
        .unwrap_or(0);
    let prev = LAST.load(Ordering::Relaxed);
    let next = now_micros.max(prev.saturating_add(1));
    LAST.store(next, Ordering::Relaxed);
    next.to_string()
}

fn sign_eip712(private_key: &str, message: &str, chain_id: u64) -> Result<String, AppError> {
    let digest = eip712_message_digest(message, chain_id);
    let pk_hex = private_key.trim().trim_start_matches("0x");
    let pk_bytes = hex::decode(pk_hex).map_err(|e| {
        AppError::ExchangeCrypto(format!("hex decode failed (len={}): {e}", pk_hex.len()))
    })?;
    if pk_bytes.len() != 32 {
        return Err(AppError::ExchangeCrypto(format!(
            "private key must be 32 bytes, got {} bytes ({} hex chars)",
            pk_bytes.len(),
            pk_hex.len()
        )));
    }
    let signing_key = SigningKey::from_slice(&pk_bytes)
        .map_err(|e| AppError::ExchangeCrypto(format!("invalid secp256k1 private key: {e}")))?;
    let (signature, recovery_id) = signing_key
        .sign_prehash_recoverable(&digest)
        .map_err(|e| AppError::ExchangeCrypto(format!("sign_prehash_recoverable failed: {e}")))?;

    let sig_bytes = signature.to_bytes();
    let v = recovery_id.to_byte() + 27;
    let mut out = Vec::with_capacity(65);
    out.extend_from_slice(&sig_bytes[..]);
    out.push(v);
    Ok(format!("0x{}", hex::encode(&out)))
}

fn eip712_message_digest(message: &str, chain_id: u64) -> [u8; 32] {
    let domain_separator = aster_domain_separator(chain_id);
    let message_typehash = keccak256(b"Message(string msg)");
    let message_hash = keccak256(message.as_bytes());

    let mut struct_hash = Vec::with_capacity(64);
    struct_hash.extend_from_slice(&message_typehash);
    struct_hash.extend_from_slice(&message_hash);
    let struct_hash = keccak256(&struct_hash);

    let mut digest = Vec::with_capacity(66);
    digest.extend_from_slice(b"\x19\x01");
    digest.extend_from_slice(&domain_separator);
    digest.extend_from_slice(&struct_hash);
    keccak256(&digest)
}

fn aster_domain_separator(chain_id: u64) -> [u8; 32] {
    let domain_typehash = keccak256(
        b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
    );
    let name_hash = keccak256(b"AsterSignTransaction");
    let version_hash = keccak256(b"1");
    let chain_id = to_uint256(chain_id);
    let verifying_contract = [0_u8; 32];

    let mut bytes = Vec::with_capacity(160);
    bytes.extend_from_slice(&domain_typehash);
    bytes.extend_from_slice(&name_hash);
    bytes.extend_from_slice(&version_hash);
    bytes.extend_from_slice(&chain_id);
    bytes.extend_from_slice(&verifying_contract);
    keccak256(&bytes)
}

fn to_uint256(value: u64) -> [u8; 32] {
    let mut out = [0_u8; 32];
    out[24..].copy_from_slice(&value.to_be_bytes());
    out
}

fn derive_eth_address(private_key: &str) -> Result<String, AppError> {
    let pk_hex = private_key.trim().trim_start_matches("0x");
    let pk_bytes = hex::decode(pk_hex).map_err(|e| {
        AppError::ExchangeCrypto(format!("hex decode failed (len={}): {e}", pk_hex.len()))
    })?;
    if pk_bytes.len() != 32 {
        return Err(AppError::ExchangeCrypto(format!(
            "private key must be 32 bytes, got {} bytes ({} hex chars)",
            pk_bytes.len(),
            pk_hex.len()
        )));
    }
    let signing_key = SigningKey::from_slice(&pk_bytes)
        .map_err(|e| AppError::ExchangeCrypto(format!("invalid secp256k1 private key: {e}")))?;
    let verifying_key = signing_key.verifying_key();
    let encoded = verifying_key.to_encoded_point(false);
    let bytes: &[u8] = AsRef::<[u8]>::as_ref(&encoded);
    let hash = keccak256(&bytes[1..]);
    Ok(format!("0x{}", hex::encode(&hash[12..])))
}

fn keccak256(input: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(input);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use axum::{
        Router,
        extract::State,
        http::{Method, Uri},
        routing::{get, post},
    };

    use super::*;

    async fn capture_settings_request(
        State(requests): State<Arc<Mutex<Vec<String>>>>,
        uri: Uri,
    ) -> &'static str {
        requests
            .lock()
            .expect("request lock poisoned")
            .push(uri.to_string());
        "{}"
    }

    async fn capture_conditional_request(
        State(requests): State<Arc<Mutex<Vec<String>>>>,
        method: Method,
        uri: Uri,
    ) -> axum::Json<serde_json::Value> {
        requests
            .lock()
            .expect("request lock poisoned")
            .push(format!("{} {}", method, uri));
        if uri.path().ends_with("/positionSide/dual") {
            axum::Json(serde_json::json!({ "dualSidePosition": false }))
        } else {
            axum::Json(serde_json::json!({
                "orderId": 1,
                "clientOrderId": "tpsl_test",
                "symbol": "BTCUSDT",
                "side": "SELL",
                "positionSide": "LONG",
                "reduceOnly": true,
                "status": "NEW",
                "type": "TAKE_PROFIT_MARKET",
                "price": "0",
                "origQty": "0.01",
                "executedQty": "0",
                "updateTime": 1
            }))
        }
    }

    #[tokio::test]
    async fn aster_conditional_order_maps_trigger_and_reduce_only_fields() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let app = Router::new()
            .route(
                "/fapi/v3/positionSide/dual",
                get(capture_conditional_request),
            )
            .route("/fapi/v3/order", post(capture_conditional_request))
            .with_state(requests.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let address = listener.local_addr().expect("test server address");
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve test server");
        });

        let mut adapter = AsterAdapter::new(ExchangeCredentials {
            api_key: String::new(),
            secret_key: format!("{:064x}", 1),
            passphrase: None,
            wallet_addr: Some("0x0000000000000000000000000000000000000001".to_string()),
            testnet: true,
        })
        .expect("adapter");
        adapter.base_url = format!("http://{address}");

        adapter
            .place_conditional_order(PlaceConditionalOrderRequest {
                symbol: "BTCUSDT".to_string(),
                side: ExchangeSide::Sell,
                conditional_type: ExchangeConditionalOrderType::TakeProfitMarket,
                quantity: 0.01,
                trigger_price: 102.0,
                reduce_only: true,
                margin_mode: Some(ExchangeMarginMode::Isolated),
                position_side: Some(PositionSide::Long),
                client_order_id: Some("tpsl_tp_test".to_string()),
            })
            .await
            .expect("conditional order");

        server.abort();
        let requests = requests.lock().expect("request lock poisoned").clone();
        let order_request = requests
            .iter()
            .find(|request| request.starts_with("POST /fapi/v3/order?"))
            .expect("order request");
        assert!(order_request.contains("type=TAKE_PROFIT_MARKET"));
        assert!(order_request.contains("stopPrice=102"));
        assert!(order_request.contains("workingType=MARK_PRICE"));
        assert!(order_request.contains("reduceOnly=true"));
        assert!(!order_request.contains("positionSide=LONG"));
    }

    #[tokio::test]
    async fn aster_forwards_configured_leverage_without_adapter_cap() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let app = Router::new()
            .route("/fapi/v3/marginType", post(capture_settings_request))
            .route("/fapi/v3/leverage", post(capture_settings_request))
            .with_state(requests.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let address = listener.local_addr().expect("test server address");
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve test server");
        });

        let mut adapter = AsterAdapter::new(ExchangeCredentials {
            api_key: String::new(),
            secret_key: format!("{:064x}", 1),
            passphrase: None,
            wallet_addr: Some("0x0000000000000000000000000000000000000001".to_string()),
            testnet: true,
        })
        .expect("adapter");
        adapter.base_url = format!("http://{address}");

        adapter
            .ensure_symbol_settings("BTCUSDT", 200, ExchangeMarginMode::Isolated)
            .await
            .expect("symbol settings");

        server.abort();
        let requests = requests.lock().expect("request lock poisoned").clone();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].starts_with("/fapi/v3/marginType?"));
        assert!(requests[1].starts_with("/fapi/v3/leverage?"));
        assert!(requests[1].contains("leverage=200"));
    }

    #[test]
    fn aster_signs_known_eip712_message() {
        let private_key = "4fd0a42218f3eae43a6ce26d22544e986139a01e5b34a62db53757ffca81bae1";
        let signature =
            sign_eip712(private_key, "nonce=1748310859508867&user=0x63dd5acc6b1aa0f563956c0e534dd30b6dcf7c4e&signer=0x21cf8ae13bb72632562c6ff438652ba1a151bb0", 1666)
                .expect("signature");
        assert!(signature.starts_with("0x"));
        assert_eq!(signature.len(), 2 + 130);
    }

    #[test]
    fn aster_derives_signer_address_from_private_key() {
        let private_key = "4fd0a42218f3eae43a6ce26d22544e986139a01e5b34a62db53757ffca81bae1";
        let address = derive_eth_address(private_key).expect("address");
        assert_eq!(address, "0x21cf8ae13bb72632562c6fff438652ba1a151bb0");
    }

    #[test]
    fn aster_nonce_is_monotonic() {
        let a = next_nonce().parse::<u64>().unwrap();
        let b = next_nonce().parse::<u64>().unwrap();
        assert!(b > a);
    }

    #[test]
    fn aster_constraints_select_the_requested_symbol_from_a_full_response() {
        let payload = serde_json::json!({
            "symbols": [
                {
                    "symbol": "ASTERUSDT",
                    "baseAsset": "ASTER",
                    "quoteAsset": "USDT",
                    "filters": [{
                        "filterType": "LOT_SIZE",
                        "minQty": "0.01",
                        "maxQty": "2000000",
                        "stepSize": "0.01"
                    }]
                },
                {
                    "symbol": "BTCUSDT",
                    "baseAsset": "BTC",
                    "quoteAsset": "USDT",
                    "filters": [{
                        "filterType": "LOT_SIZE",
                        "minQty": "0.001",
                        "maxQty": "1000",
                        "stepSize": "0.001"
                    }]
                }
            ]
        });

        let constraints = parse_symbol_constraints(&payload, "BTCUSDT").expect("constraints");

        assert_eq!(constraints.symbol, "BTCUSDT");
        assert_eq!(constraints.base_asset, "BTC");
        assert_eq!(constraints.min_qty, 0.001);
        assert_eq!(constraints.step_size, 0.001);
    }
}
