use axum::{
    Router,
    routing::{get, post},
};

use crate::{http::handlers::backtest::*, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/start", post(handle_backtest_start))
        .route("/stop", post(handle_backtest_stop))
        .route("/delete", post(handle_backtest_delete))
        .route("/runs", get(handle_backtest_runs))
}
