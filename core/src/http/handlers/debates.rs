use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    contracts::debates::{
        CreateDebateRequest, DebateActionPayload, DebateDetailPayload, DebateExecutionPayload,
        DebateListPayload, DebateMessagePayload, DebateMessagesPayload, DebatePersonalitiesPayload,
        DebateVotesPayload, StartDebateRequest,
    },
    error::Result,
    http::response::ApiResponse,
    state,
};

pub async fn handle_get_debates(
    State(app): State<state::AppState>,
) -> Result<Json<ApiResponse<DebateListPayload>>> {
    let payload = app.services.debate_service.list().await;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_create_debate(
    State(app): State<state::AppState>,
    Json(request): Json<CreateDebateRequest>,
) -> Result<Json<ApiResponse<DebateActionPayload>>> {
    let payload = app
        .services
        .debate_service
        .create(request)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_get_debate_personalities(
    State(app): State<state::AppState>,
) -> Result<Json<ApiResponse<DebatePersonalitiesPayload>>> {
    let payload = app.services.debate_service.personalities();
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_get_debate(
    State(app): State<state::AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<DebateDetailPayload>>> {
    let payload = app.services.debate_service.get(&id).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_delete_debate(
    State(app): State<state::AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<DebateMessagePayload>>> {
    let payload = app.services.debate_service.delete(&id).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_start_debate(
    State(app): State<state::AppState>,
    Path(id): Path<String>,
    Json(_request): Json<StartDebateRequest>,
) -> Result<Json<ApiResponse<DebateActionPayload>>> {
    let payload = app.services.debate_service.start(&id).await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_cancel_debate(
    State(app): State<state::AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<DebateActionPayload>>> {
    let payload = app.services.debate_service.cancel(&id)?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_execute_debate(
    State(app): State<state::AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<DebateExecutionPayload>>> {
    let payload = app
        .services
        .debate_service
        .execution(&id)
        .await?;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_get_debate_messages(
    State(app): State<state::AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<DebateMessagesPayload>>> {
    let payload = app.services.debate_service.messages(&id).await;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}

pub async fn handle_get_debate_votes(
    State(app): State<state::AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<DebateVotesPayload>>> {
    let payload = app.services.debate_service.votes(&id).await;
    Ok(Json(ApiResponse::success(Some(payload), None)))
}
