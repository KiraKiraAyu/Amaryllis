use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use hmac::{Hmac, Mac};
use reqwest::{Client, Method, StatusCode};
use serde::Deserialize;
use sha2::Sha256;

use crate::{
    clients::{
        exchanges::{
            CancelOrderResponse, ExchangeBalance, ExchangeCredentials, ExchangeMarginMode,
            ExchangeOpenOrder, ExchangeOrderDetail, ExchangeOrderType, ExchangePosition,
            ExchangeSide, ExchangeSymbolConstraints, ExchangeTradeFill, LiveExchangeAdapter,
            PlaceOrderRequest, PlaceOrderResponse, PositionSide, TimeInForce,
        },
        outbound_http::{OutboundRequestLog, OutboundResponse, send_text},
    },
    error::AppError,
};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
pub struct BinanceFuturesAdapter {
    client: Client,
    exchange_type: &'static str,
    api_key: String,
    secret_key: String,
    base_url: String,
    ws_base_url: String,
}

impl BinanceFuturesAdapter {
    pub fn new(credentials: ExchangeCredentials) -> Result<Self, AppError> {
        Self::new_with_config(
            "binance",
            credentials,
            "https://fapi.binance.com",
            "https://testnet.binancefuture.com",
            "wss://fstream.binance.com",
            "wss://stream.binancefuture.com",
        )
    }

    fn new_with_config(
        exchange_type: &'static str,
        credentials: ExchangeCredentials,
        live_base_url: &str,
        testnet_base_url: &str,
        live_ws_base_url: &str,
        testnet_ws_base_url: &str,
    ) -> Result<Self, AppError> {
        if credentials.api_key.trim().is_empty() {
            return Err(AppError::InvalidExchangeConfig(format!(
                "{exchange_type} api_key is required"
            )));
        }
        if credentials.secret_key.trim().is_empty() {
            return Err(AppError::InvalidExchangeConfig(format!(
                "{exchange_type} secret_key is required"
            )));
        }

        let base_url = if credentials.testnet {
            testnet_base_url.to_string()
        } else {
            live_base_url.to_string()
        };
        let ws_base_url = if credentials.testnet {
            testnet_ws_base_url.to_string()
        } else {
            live_ws_base_url.to_string()
        };

        Ok(Self {
            client: Client::builder().build().map_err(AppError::ExchangeHttp)?,
            exchange_type,
            api_key: credentials.api_key,
            secret_key: credentials.secret_key,
            base_url,
            ws_base_url,
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
            OutboundRequestLog::new("exchange.binance.public", Method::GET, &url),
        )
        .await
        .map_err(AppError::ExchangeHttp)?;
        parse_json_response(resp)
    }

    async fn signed_get<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        mut params: Vec<(&str, String)>,
    ) -> Result<T, AppError> {
        self.sign_and_send(Method::GET, path, &mut params).await
    }

    async fn signed_post<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        mut params: Vec<(&str, String)>,
    ) -> Result<T, AppError> {
        self.sign_and_send(Method::POST, path, &mut params).await
    }

    async fn signed_delete<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        mut params: Vec<(&str, String)>,
    ) -> Result<T, AppError> {
        self.sign_and_send(Method::DELETE, path, &mut params).await
    }

    async fn sign_and_send<T: for<'de> Deserialize<'de>>(
        &self,
        method: Method,
        path: &str,
        params: &mut Vec<(&str, String)>,
    ) -> Result<T, AppError> {
        params.push(("timestamp", now_millis()?.to_string()));
        params.push(("recvWindow", "5000".to_string()));

        let query = build_query(params.clone());
        let signature = sign_hmac_sha256(&self.secret_key, &query)?;
        let full_query = format!("{}&signature={}", query, signature);
        let url = format!("{}{}?{}", self.base_url, path, full_query);

        let req = self
            .client
            .request(method.clone(), &url)
            .header("X-MBX-APIKEY", &self.api_key);

        let resp = send_text(
            req,
            OutboundRequestLog::new("exchange.binance.signed", method, &url),
        )
        .await
        .map_err(AppError::ExchangeHttp)?;
        parse_json_response(resp)
    }

    async fn user_stream_request(
        &self,
        method: Method,
        listen_key: Option<&str>,
    ) -> Result<serde_json::Value, AppError> {
        let url = if let Some(key) = listen_key {
            let key = key.trim();
            if key.is_empty() {
                return Err(AppError::InvalidExchangeConfig(
                    "listen_key is required".to_string(),
                ));
            }
            format!(
                "{}{}?listenKey={}",
                self.base_url,
                "/fapi/v1/listenKey",
                encode_component(key)
            )
        } else {
            format!("{}{}", self.base_url, "/fapi/v1/listenKey")
        };

        let resp = send_text(
            self.client
                .request(method.clone(), &url)
                .header("X-MBX-APIKEY", &self.api_key),
            OutboundRequestLog::new("exchange.binance.user_stream", method, &url),
        )
        .await
        .map_err(AppError::ExchangeHttp)?;

        parse_json_response(resp)
    }

    async fn dual_side_position(&self) -> Result<bool, AppError> {
        let payload: BinancePositionModeResponse = self
            .signed_get("/fapi/v1/positionSide/dual", vec![])
            .await?;
        Ok(payload.dual_side_position)
    }
}

#[async_trait]
impl LiveExchangeAdapter for BinanceFuturesAdapter {
    fn exchange_type(&self) -> &'static str {
        self.exchange_type
    }

    async fn ping(&self) -> Result<(), AppError> {
        let _: serde_json::Value = self.public_get("/fapi/v1/ping", vec![]).await?;
        Ok(())
    }

    async fn get_price(&self, symbol: &str) -> Result<f64, AppError> {
        let payload: BinanceTickerPrice = self
            .public_get(
                "/fapi/v1/ticker/price",
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

        let hedge_mode = self.dual_side_position().await?;
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

        let payload: BinanceOrderResponse = self.signed_post("/fapi/v1/order", params).await?;
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
        let payload: BinanceOrderResponse = self
            .signed_delete(
                "/fapi/v1/order",
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
        let rows: Vec<BinanceBalanceRow> = self.signed_get("/fapi/v2/balance", vec![]).await?;
        let out = rows
            .into_iter()
            .map(|v| ExchangeBalance {
                asset: v.asset,
                wallet_balance: parse_f64(&v.balance),
                available_balance: parse_f64(&v.available_balance),
                unrealized_pnl: parse_f64(&v.cross_un_pnl),
            })
            .collect::<Vec<_>>();
        Ok(out)
    }

    async fn get_positions(&self) -> Result<Vec<ExchangePosition>, AppError> {
        let rows: Vec<BinancePositionRiskRow> =
            self.signed_get("/fapi/v2/positionRisk", vec![]).await?;

        let out = rows
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
            .collect::<Vec<_>>();

        Ok(out)
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

        let rows: Vec<BinanceOpenOrderRow> = self.signed_get("/fapi/v1/openOrders", params).await?;
        let out = rows
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
            .collect::<Vec<_>>();

        Ok(out)
    }

    async fn get_order(
        &self,
        symbol: &str,
        order_id: &str,
    ) -> Result<ExchangeOrderDetail, AppError> {
        let payload: BinanceOrderResponse = self
            .signed_get(
                "/fapi/v1/order",
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
        let rows: Vec<BinanceUserTradeRow> = self
            .signed_get(
                "/fapi/v1/userTrades",
                vec![
                    ("symbol", symbol.trim().to_uppercase()),
                    ("orderId", order_id.trim().to_string()),
                    ("limit", "1000".to_string()),
                ],
            )
            .await?;

        let fills = rows
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
            .collect::<Vec<_>>();

        Ok(fills)
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
                "/fapi/v1/exchangeInfo",
                vec![("symbol", symbol_upper.clone())],
            )
            .await?;

        let symbols = payload
            .get("symbols")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                AppError::InvalidExchangeConfig("missing symbols in exchangeInfo".to_string())
            })?;

        let first = symbols.first().ok_or_else(|| {
            AppError::InvalidExchangeConfig(format!(
                "symbol not found in exchangeInfo: {}",
                symbol_upper
            ))
        })?;

        let min_qty = first
            .get("filters")
            .and_then(|v| v.as_array())
            .and_then(|filters| {
                filters.iter().find_map(|f| {
                    if f.get("filterType").and_then(|x| x.as_str()) == Some("LOT_SIZE") {
                        f.get("minQty").and_then(|x| x.as_str())
                    } else {
                        None
                    }
                })
            })
            .map(parse_f64)
            .unwrap_or(0.0);

        let max_qty = first
            .get("filters")
            .and_then(|v| v.as_array())
            .and_then(|filters| {
                filters.iter().find_map(|f| {
                    if f.get("filterType").and_then(|x| x.as_str()) == Some("LOT_SIZE") {
                        f.get("maxQty").and_then(|x| x.as_str())
                    } else {
                        None
                    }
                })
            })
            .map(parse_f64)
            .unwrap_or(0.0);

        let step_size = first
            .get("filters")
            .and_then(|v| v.as_array())
            .and_then(|filters| {
                filters.iter().find_map(|f| {
                    if f.get("filterType").and_then(|x| x.as_str()) == Some("LOT_SIZE") {
                        f.get("stepSize").and_then(|x| x.as_str())
                    } else {
                        None
                    }
                })
            })
            .map(parse_f64)
            .unwrap_or(0.0);

        let min_notional = first
            .get("filters")
            .and_then(|v| v.as_array())
            .and_then(|filters| {
                filters.iter().find_map(|f| {
                    if f.get("filterType").and_then(|x| x.as_str()) == Some("MIN_NOTIONAL") {
                        f.get("notional")
                            .and_then(|x| x.as_str())
                            .or_else(|| f.get("minNotional").and_then(|x| x.as_str()))
                    } else {
                        None
                    }
                })
            })
            .map(parse_f64)
            .unwrap_or(0.0);

        let tick_size = first
            .get("filters")
            .and_then(|v| v.as_array())
            .and_then(|filters| {
                filters.iter().find_map(|f| {
                    if f.get("filterType").and_then(|x| x.as_str()) == Some("PRICE_FILTER") {
                        f.get("tickSize").and_then(|x| x.as_str())
                    } else {
                        None
                    }
                })
            })
            .map(parse_f64)
            .unwrap_or(0.0);

        Ok(ExchangeSymbolConstraints {
            symbol: first
                .get("symbol")
                .and_then(|v| v.as_str())
                .unwrap_or(&symbol_upper)
                .to_string(),
            base_asset: first
                .get("baseAsset")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            quote_asset: first
                .get("quoteAsset")
                .and_then(|v| v.as_str())
                .unwrap_or("USDT")
                .to_string(),
            min_qty,
            max_qty,
            step_size,
            min_notional,
            tick_size,
        })
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
        let leverage = leverage.clamp(1, 125);
        let margin_type = match margin_mode {
            ExchangeMarginMode::Cross => "CROSSED",
            ExchangeMarginMode::Isolated => "ISOLATED",
        };

        let margin_result: Result<serde_json::Value, AppError> = self
            .signed_post(
                "/fapi/v1/marginType",
                vec![
                    ("symbol", symbol.clone()),
                    ("marginType", margin_type.to_string()),
                ],
            )
            .await;
        match margin_result {
            Ok(_) => {}
            Err(err) if is_binance_margin_type_noop(&err) => {}
            Err(err) => return Err(err),
        }

        let _: serde_json::Value = self
            .signed_post(
                "/fapi/v1/leverage",
                vec![("symbol", symbol), ("leverage", leverage.to_string())],
            )
            .await?;
        Ok(())
    }

    async fn start_user_stream(&self) -> Result<String, AppError> {
        let payload = self.user_stream_request(Method::POST, None).await?;
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
        let _ = self
            .user_stream_request(Method::PUT, Some(listen_key))
            .await?;
        Ok(())
    }

    async fn close_user_stream(&self, listen_key: &str) -> Result<(), AppError> {
        let _ = self
            .user_stream_request(Method::DELETE, Some(listen_key))
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
struct BinanceApiErrorPayload {
    #[serde(default)]
    code: i64,
    #[serde(default)]
    msg: String,
}

#[derive(Debug, Deserialize)]
struct BinanceTickerPrice {
    price: String,
}

#[derive(Debug, Deserialize)]
struct BinanceOrderResponse {
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
struct BinanceOpenOrderRow {
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
struct BinanceUserTradeRow {
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
struct BinanceBalanceRow {
    asset: String,
    balance: String,
    #[serde(rename = "availableBalance")]
    available_balance: String,
    #[serde(rename = "crossUnPnl")]
    cross_un_pnl: String,
}

#[derive(Debug, Deserialize)]
struct BinancePositionRiskRow {
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
struct BinancePositionModeResponse {
    #[serde(rename = "dualSidePosition", default)]
    dual_side_position: bool,
}

fn parse_json_response<T: for<'de> Deserialize<'de>>(
    resp: OutboundResponse,
) -> Result<T, AppError> {
    let status = resp.status;
    let text = resp.body;

    if !status.is_success() {
        if let Ok(err_payload) = serde_json::from_str::<BinanceApiErrorPayload>(&text) {
            return Err(AppError::ExchangeApi {
                status: status.as_u16(),
                code: err_payload.code,
                message: err_payload.msg,
            });
        }

        return Err(AppError::ExchangeApi {
            status: status.as_u16(),
            code: status_to_code(status),
            message: text,
        });
    }

    let val = serde_json::from_str::<T>(&text).map_err(AppError::ExchangeJson)?;
    Ok(val)
}

fn status_to_code(status: StatusCode) -> i64 {
    i64::from(status.as_u16())
}

fn now_millis() -> Result<u128, AppError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AppError::ExchangeTime(e.to_string()))?;
    Ok(now.as_millis())
}

fn sign_hmac_sha256(secret: &str, payload: &str) -> Result<String, AppError> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| AppError::ExchangeCrypto(e.to_string()))?;
    mac.update(payload.as_bytes());
    let bytes = mac.finalize().into_bytes();
    Ok(hex::encode(bytes))
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
                out.push(char::from(b))
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

fn inferred_position_side_for_order(side: ExchangeSide, reduce_only: bool) -> PositionSide {
    match (side, reduce_only) {
        (ExchangeSide::Sell, false) | (ExchangeSide::Buy, true) => PositionSide::Short,
        _ => PositionSide::Long,
    }
}

fn is_binance_margin_type_noop(err: &AppError) -> bool {
    matches!(err, AppError::ExchangeApi { code: -4046, .. })
        || matches!(
            err,
            AppError::ExchangeApi { message, .. }
                if message.to_ascii_lowercase().contains("no need to change margin type")
        )
}
