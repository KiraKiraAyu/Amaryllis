use super::service::*;

pub async fn get_trader_by_owner(
    app: &SharedState,
    trader_id: &str,
) -> Result<Option<TraderRecord>, crate::database::DbErr> {
    app.trading_repo.get_trader(trader_id).await
}

pub async fn resolve_trader_id(
    app: &SharedState,
    requested: Option<String>,
) -> crate::error::Result<String> {
    if let Some(id) = requested
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
    {
        let exists = app
            .trading_repo
            .get_trader(&id)
            .await
            .map_err(|_| app_error(AppErrorKind::Internal, "Failed to validate trader"))?;

        if exists.is_none() {
            return Err(app_error(AppErrorKind::NotFound, "Trader not found"));
        }
        return Ok(id);
    }

    app.trading_repo
        .first_trader_id()
        .await
        .map_err(|_| app_error(AppErrorKind::Internal, "Failed to resolve trader"))?
        .ok_or_else(|| app_error(AppErrorKind::NotFound, "No available traders"))
}

pub async fn trader_owner_missing(app: &SharedState, trader_id: &str) -> bool {
    app.trading_repo
        .get_trader(trader_id)
        .await
        .ok()
        .flatten()
        .is_none()
}
