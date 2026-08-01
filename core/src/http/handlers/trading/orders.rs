use axum::{
    Json,
    extract::{Path, Query, State},
};

use crate::{
    contracts::trading::{
        common::{PaginationQuery, TraderQuery},
        orders::{FillListPayload, OrderListPayload},
    },
    error::Result,
    http::response::ApiResponse,
    state,
};

use super::trading_service;

pub async fn orders(
    State(app): State<state::AppState>,
    Query(q): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<OrderListPayload>>> {
    let payload = trading_service(&app).orders(q).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn order_fills(
    State(app): State<state::AppState>,
    Path(id): Path<String>,
    Query(q): Query<TraderQuery>,
) -> Result<Json<ApiResponse<FillListPayload>>> {
    let payload = trading_service(&app).order_fills(&id, q).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn open_orders(
    State(app): State<state::AppState>,
    Query(q): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<OrderListPayload>>> {
    let payload = trading_service(&app).open_orders(q).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}
