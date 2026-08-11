use super::service::*;
use crate::{entity::strategies, services::data_template::validate_strategy_data_template};
use sea_orm::EntityTrait;

pub async fn list_traders(app: &SharedState) -> AppResult<TraderListPayload> {
    match app.trading_repo.list_traders().await {
        Ok(traders) => {
            let traders: Vec<TraderPayload> = traders
                .into_iter()
                .map(TraderPayloadExt::into_payload)
                .collect();
            Ok(TraderListPayload {
                count: traders.len(),
                traders,
            })
        }
        Err(_) => Err(app_error(
            AppErrorKind::Internal,
            "Failed to get trader list",
        )),
    }
}

pub async fn get_trader(app: &SharedState, id: &str) -> AppResult<TraderPayload> {
    match get_trader_by_owner(app, id).await {
        Ok(Some(trader)) => Ok(trader.into_payload()),
        Ok(None) => Err(app_error(
            AppErrorKind::NotFound,
            "Trader does not exist or no permission",
        )),
        Err(_) => Err(app_error(AppErrorKind::Internal, "Failed to get trader")),
    }
}

pub async fn get_trader_config(app: &SharedState, id: &str) -> AppResult<TraderPayload> {
    match get_trader_by_owner(app, id).await {
        Ok(Some(trader)) => Ok(trader.into_payload()),
        Ok(None) => Err(app_error(
            AppErrorKind::NotFound,
            "Trader does not exist or no permission",
        )),
        Err(_) => Err(app_error(
            AppErrorKind::Internal,
            "Failed to load trader configuration",
        )),
    }
}

pub async fn create_trader(
    app: &SharedState,
    name: &str,
    ai_model_id: &str,
    exchange_id: &str,
    strategy_id: &str,
    initial_balance: f64,
    scan_interval_minutes: i64,
    is_cross_margin: Option<bool>,
    use_ai500: bool,
    use_oi_top: bool,
    custom_prompt: &str,
    override_base_prompt: bool,
    system_prompt_template: &str,
) -> AppResult<TraderCreatedPayload> {
    let name = name.trim();
    let ai_model_id = ai_model_id.trim();
    let exchange_id = exchange_id.trim();
    let strategy_id = strategy_id.trim();

    if name.is_empty() || ai_model_id.is_empty() || exchange_id.is_empty() || strategy_id.is_empty()
    {
        return Err(app_error(
            AppErrorKind::BadRequest,
            "name, ai_model_id, exchange_id, strategy_id are required",
        ));
    }
    validate_selected_strategy(app, strategy_id).await?;
    if let Err(err) = app.llm_service.resolve_for_user(Some(ai_model_id)).await {
        return Err(match err {
            AppError::BadRequest(_) => app_error(
                AppErrorKind::BadRequest,
                "Selected ai_model_id does not exist",
            ),
            _ => app_error(AppErrorKind::Internal, "Failed to validate ai_model_id"),
        });
    }

    let now = now_ts();
    let trader_id = Uuid::now_v7().to_string();
    let snapshot_id = Uuid::now_v7().to_string();

    let inserted = app
        .trading_repo
        .create_trader_with_snapshot(CreateTraderRecord {
            id: trader_id.clone(),
            snapshot_id,
            name: name.to_string(),
            ai_model_id: ai_model_id.to_string(),
            exchange_id: exchange_id.to_string(),
            strategy_id: strategy_id.to_string(),
            initial_balance: initial_balance.max(0.0),
            scan_interval_minutes: scan_interval_minutes.max(1),
            is_cross_margin: is_cross_margin.unwrap_or(true),
            use_ai500,
            use_oi_top,
            custom_prompt: custom_prompt.trim().to_string(),
            override_base_prompt,
            system_prompt_template: system_prompt_template.trim().to_string(),
            created_at: now,
            updated_at: now,
        })
        .await;

    if inserted.is_err() {
        return Err(app_error(AppErrorKind::Internal, "Failed to create trader"));
    }

    Ok(TraderCreatedPayload {
        id: trader_id,
        message: "Trader created successfully",
    })
}

pub async fn update_trader(
    app: &SharedState,
    id: &str,
    name: Option<String>,
    ai_model_id: Option<String>,
    exchange_id: Option<String>,
    strategy_id: Option<String>,
    initial_balance: Option<f64>,
    scan_interval_minutes: Option<i64>,
    is_cross_margin: Option<bool>,
    use_ai500: Option<bool>,
    use_oi_top: Option<bool>,
    custom_prompt: Option<String>,
    override_base_prompt: Option<bool>,
    system_prompt_template: Option<String>,
) -> AppResult<TraderMessagePayload> {
    let existing = match get_trader_by_owner(app, id).await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return Err(app_error(
                AppErrorKind::NotFound,
                "Trader does not exist or no permission",
            ));
        }
        Err(_) => {
            return Err(app_error(AppErrorKind::Internal, "Failed to load trader"));
        }
    };

    if let Some(ai_model_id) = ai_model_id.as_deref().map(str::trim) {
        if ai_model_id.is_empty() {
            return Err(app_error(
                AppErrorKind::BadRequest,
                "ai_model_id cannot be empty",
            ));
        }
        if let Err(err) = app.llm_service.resolve_for_user(Some(ai_model_id)).await {
            return Err(match err {
                AppError::BadRequest(_) => app_error(
                    AppErrorKind::BadRequest,
                    "Selected ai_model_id does not exist",
                ),
                _ => app_error(AppErrorKind::Internal, "Failed to validate ai_model_id"),
            });
        }
    }

    let strategy_id = strategy_id
        .unwrap_or(existing.strategy_id)
        .trim()
        .to_string();
    validate_selected_strategy(app, &strategy_id).await?;

    let now = now_ts();

    let result = app
        .trading_repo
        .update_trader(
            id,
            UpdateTraderRecord {
                name: name.unwrap_or(existing.name).trim().to_string(),
                ai_model_id: ai_model_id
                    .unwrap_or(existing.ai_model_id)
                    .trim()
                    .to_string(),
                exchange_id: exchange_id
                    .unwrap_or(existing.exchange_id)
                    .trim()
                    .to_string(),
                strategy_id,
                initial_balance: initial_balance.unwrap_or(existing.initial_balance).max(0.0),
                scan_interval_minutes: scan_interval_minutes
                    .unwrap_or(existing.scan_interval_minutes)
                    .max(1),
                is_cross_margin: is_cross_margin.unwrap_or(existing.is_cross_margin != 0),
                use_ai500: use_ai500.unwrap_or(existing.use_ai500 != 0),
                use_oi_top: use_oi_top.unwrap_or(existing.use_oi_top != 0),
                custom_prompt: custom_prompt
                    .unwrap_or(existing.custom_prompt)
                    .trim()
                    .to_string(),
                override_base_prompt: override_base_prompt
                    .unwrap_or(existing.override_base_prompt != 0),
                system_prompt_template: system_prompt_template
                    .unwrap_or(existing.system_prompt_template)
                    .trim()
                    .to_string(),
                updated_at: now,
            },
        )
        .await;

    match result {
        Ok(_) => Ok(TraderMessagePayload {
            message: "Trader updated successfully",
        }),
        Err(_) => Err(app_error(AppErrorKind::Internal, "Failed to update trader")),
    }
}

async fn validate_selected_strategy(app: &SharedState, strategy_id: &str) -> AppResult<()> {
    let strategy_id = strategy_id.trim();
    if strategy_id.is_empty() {
        return Err(app_error(
            AppErrorKind::BadRequest,
            "strategy_id is required",
        ));
    }

    let strategy = strategies::Entity::find_by_id(strategy_id.to_string())
        .one(app.trading_repo.db())
        .await
        .map_err(|_| app_error(AppErrorKind::Internal, "Failed to validate strategy"))?
        .ok_or_else(|| app_error(AppErrorKind::BadRequest, "Selected strategy does not exist"))?;
    let config = serde_json::from_str(&strategy.config).map_err(|_| {
        app_error(
            AppErrorKind::BadRequest,
            "Selected strategy has invalid configuration",
        )
    })?;
    validate_strategy_data_template(&config).map_err(|message| {
        AppError::from_kind(
            AppErrorKind::BadRequest,
            format!("Selected strategy has invalid data template: {message}"),
        )
    })?;
    Ok(())
}

pub async fn delete_trader(
    app: &SharedState,
    trading_runtime_service: &TradingRuntimeService,
    id: &str,
) -> AppResult<TraderMessagePayload> {
    if let Err(err) = trading_runtime_service.stop_trader_for_user(id).await {
        if !matches!(err, AppError::NotRunning(_)) {
            return Err(app_error(
                AppErrorKind::Internal,
                "Failed to stop running trader",
            ));
        }
    }

    let deleted = app.trading_repo.delete_trader(id).await;
    match deleted {
        Ok(0) => {
            return Err(app_error(
                AppErrorKind::NotFound,
                "Trader does not exist or no permission",
            ));
        }
        Ok(_) => {}
        Err(_) => {
            return Err(app_error(AppErrorKind::Internal, "Failed to delete trader"));
        }
    }

    let _ = app.remove_runtime_engine(id);

    Ok(TraderMessagePayload {
        message: "Trader deleted successfully",
    })
}

pub async fn start_trader(
    trading_runtime_service: &TradingRuntimeService,
    id: &str,
) -> AppResult<TraderMessagePayload> {
    match trading_runtime_service.start_trader(id).await {
        Ok(_) => Ok(TraderMessagePayload {
            message: "Trader started successfully",
        }),
        Err(AppError::TraderNotFound(_)) => Err(app_error(
            AppErrorKind::NotFound,
            "Trader does not exist or no permission",
        )),
        Err(AppError::AlreadyRunning(_)) => Ok(TraderMessagePayload {
            message: "Trader already running",
        }),
        Err(err) => {
            tracing::error!("Failed to start trader={id}: {err}");
            Err(AppError::Internal(format!("Failed to start trader: {err}")))
        }
    }
}

pub async fn stop_trader(
    trading_runtime_service: &TradingRuntimeService,
    id: &str,
) -> AppResult<TraderMessagePayload> {
    match trading_runtime_service.stop_trader_for_user(id).await {
        Ok(_) => Ok(TraderMessagePayload {
            message: "Trader stopped successfully",
        }),
        Err(AppError::NotRunning(_)) => Ok(TraderMessagePayload {
            message: "Trader already stopped",
        }),
        Err(AppError::TraderNotFound(_)) => Err(app_error(
            AppErrorKind::NotFound,
            "Trader does not exist or no permission",
        )),
        Err(err) => {
            tracing::error!("Failed to stop trader={id}: {err}");
            Err(AppError::Internal(format!("Failed to stop trader: {err}")))
        }
    }
}

pub async fn update_trader_prompt(
    app: &SharedState,
    id: &str,
    custom_prompt: &str,
    override_base_prompt: bool,
) -> AppResult<TraderMessagePayload> {
    let result = app
        .trading_repo
        .update_prompt(
            id,
            custom_prompt.trim().to_string(),
            override_base_prompt,
            now_ts(),
        )
        .await;

    match result {
        Ok(rows_affected) if rows_affected > 0 => Ok(TraderMessagePayload {
            message: "Prompt updated successfully",
        }),
        Ok(_) => Err(app_error(
            AppErrorKind::NotFound,
            "Trader does not exist or no permission",
        )),
        Err(_) => Err(app_error(AppErrorKind::Internal, "Failed to update prompt")),
    }
}
