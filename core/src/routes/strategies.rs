use axum::{
    Router,
    routing::{get, post},
};

use crate::{http::handlers::strategies::*, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(handle_get_strategies).post(handle_create_strategy))
        .route("/active", get(handle_get_active_strategy))
        .route("/default-config", get(handle_get_default_strategy_config))
        .route("/preview-prompt", post(handle_preview_prompt))
        .route("/test-run", post(handle_strategy_test_run))
        .route(
            "/{id}",
            get(handle_get_strategy)
                .put(handle_update_strategy)
                .delete(handle_delete_strategy),
        )
        .route("/{id}/activate", post(handle_activate_strategy))
        .route("/{id}/duplicate", post(handle_duplicate_strategy))
        .route("/{id}/positions", get(handle_get_strategy_positions))
}
