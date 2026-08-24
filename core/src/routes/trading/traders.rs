use axum::{
    Router,
    routing::{get, post, put},
};

use crate::{http::handlers::trading::traders, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/traders", get(traders::list).post(traders::create))
        .route(
            "/traders/{id}",
            get(traders::get)
                .put(traders::update)
                .delete(traders::delete),
        )
        .route("/traders/{id}/config", get(traders::config))
        .route("/traders/{id}/start", post(traders::start))
        .route("/traders/{id}/stop", post(traders::stop))
        .route("/traders/{id}/wake", post(traders::wake))
        .route("/traders/{id}/prompt", put(traders::update_prompt))
        .route("/equity-history", get(traders::equity_history))
        .route("/status", get(traders::status))
}
