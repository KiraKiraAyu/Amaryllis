pub mod backtest;
pub mod catalog;
pub mod crypto;
pub mod debates;
pub mod exchanges;
pub mod market;
pub mod models;
pub mod strategies;
pub mod trading;

use std::time::Duration;

use axum::{
    Router,
    http::StatusCode,
    middleware,
    routing::{get, post},
};
use tower_http::{
    cors::{Any, CorsLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

use crate::{http::handlers, http::middleware::require_auth, realtime, state::AppState};

pub fn build_app(state: AppState, timeout_secs: u64) -> Router {
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any);

    let public_api = Router::new()
        .route("/health", get(handlers::system::health))
        .route("/auth/status", get(handlers::auth::status))
        .route("/auth/verify", post(handlers::auth::verify))
        .route("/auth/setup/start", post(handlers::auth::setup_start))
        .route("/auth/setup/confirm", post(handlers::auth::setup_confirm));

    let protected_api = Router::new()
        .route("/config", get(handlers::system::config))
        .route("/settings", get(handlers::system::get_settings).put(handlers::system::update_settings))
        .route("/auth/reset/start", post(handlers::auth::reset_start))
        .route("/auth/reset/confirm", post(handlers::auth::reset_confirm))
        .nest("/catalog", catalog::router())
        .nest("/crypto", crypto::router())
        .nest("/market", market::router())
        .nest("/models", models::router())
        .nest("/exchanges", exchanges::router())
        .nest("/strategies", strategies::router())
        .nest("/trading", trading::router())
        .nest("/backtest", backtest::router())
        .nest("/debates", debates::router())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_auth,
        ));

    let timed_api =
        Router::new()
            .merge(public_api)
            .merge(protected_api)
            .layer(TimeoutLayer::with_status_code(
                StatusCode::REQUEST_TIMEOUT,
                Duration::from_secs(timeout_secs),
            ));

    let stream_api = Router::new()
        .route("/events", get(realtime::events_handler))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_auth,
        ));

    let api = Router::new().merge(timed_api).merge(stream_api);

    Router::new()
        .nest("/api", api)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}
