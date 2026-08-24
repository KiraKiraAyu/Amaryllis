use axum::{Json, extract::Query};

use crate::{
    contracts::public::{ExchangeSymbolsPayload, KlinePayload, KlinesQuery, SymbolsQuery},
    error::{AppError, Result},
    http::response::ApiResponse,
    services::market,
};

pub async fn handle_symbols(
    Query(SymbolsQuery { exchange }): Query<SymbolsQuery>,
) -> Result<Json<ApiResponse<ExchangeSymbolsPayload>>> {
    let payload = market::symbols(exchange).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_klines(
    Query(KlinesQuery {
        symbol,
        interval,
        limit,
        exchange,
    }): Query<KlinesQuery>,
) -> Result<Json<ApiResponse<Vec<KlinePayload>>>> {
    if let Some(l) = limit
        && l <= 0
    {
        return Err(AppError::BadRequest(
            "limit must be a positive number".into(),
        ));
    }
    let payload = market::klines(symbol, interval, limit, exchange).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}
