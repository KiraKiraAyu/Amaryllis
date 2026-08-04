use super::service::*;

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

pub async fn get_trader_config(
    app: &SharedState,
    id: &str,
) -> AppResult<TraderPayload> {
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
    req: CreateTraderRequest,
) -> AppResult<TraderCreatedPayload> {
    let name = req.name.trim();
    let ai_model_id = req.ai_model_id.trim();
    let exchange_id = req.exchange_id.trim();

    if name.is_empty() || ai_model_id.is_empty() || exchange_id.is_empty() {
        return Err(app_error(
            AppErrorKind::BadRequest,
            "name, ai_model_id, exchange_id are required",
        ));
    }
    if !is_valid_leverage(req.btc_eth_leverage) || !is_valid_leverage(req.altcoin_leverage) {
        return Err(app_error(
            AppErrorKind::BadRequest,
            "leverage must be between 1 and 50",
        ));
    }
    if let Err(err) = app
        .llm_service
        .resolve_for_user(Some(ai_model_id))
        .await
    {
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
            strategy_id: req.strategy_id.trim().to_string(),
            initial_balance: req.initial_balance.max(0.0),
            scan_interval_minutes: req.scan_interval_minutes.max(1),
            is_cross_margin: req.is_cross_margin.unwrap_or(true),
            btc_eth_leverage: req.btc_eth_leverage,
            altcoin_leverage: req.altcoin_leverage,
            trading_symbols: req.trading_symbols.trim().to_string(),
            use_ai500: req.use_ai500,
            use_oi_top: req.use_oi_top,
            custom_prompt: req.custom_prompt.trim().to_string(),
            override_base_prompt: req.override_base_prompt,
            system_prompt_template: req.system_prompt_template.trim().to_string(),
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
    req: UpdateTraderRequest,
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

    let btc_lev = req.btc_eth_leverage.unwrap_or(existing.btc_eth_leverage);
    let alt_lev = req.altcoin_leverage.unwrap_or(existing.altcoin_leverage);
    if !is_valid_leverage(btc_lev) || !is_valid_leverage(alt_lev) {
        return Err(app_error(
            AppErrorKind::BadRequest,
            "leverage must be between 1 and 50",
        ));
    }
    if let Some(ai_model_id) = req.ai_model_id.as_deref().map(str::trim) {
        if ai_model_id.is_empty() {
            return Err(app_error(
                AppErrorKind::BadRequest,
                "ai_model_id cannot be empty",
            ));
        }
        if let Err(err) = app
            .llm_service
            .resolve_for_user(Some(ai_model_id))
            .await
        {
            return Err(match err {
                AppError::BadRequest(_) => app_error(
                    AppErrorKind::BadRequest,
                    "Selected ai_model_id does not exist",
                ),
                _ => app_error(AppErrorKind::Internal, "Failed to validate ai_model_id"),
            });
        }
    }

    let now = now_ts();

    let result = app
        .trading_repo
        .update_trader(
            id,
            UpdateTraderRecord {
                name: req.name.unwrap_or(existing.name).trim().to_string(),
                ai_model_id: req
                    .ai_model_id
                    .unwrap_or(existing.ai_model_id)
                    .trim()
                    .to_string(),
                exchange_id: req
                    .exchange_id
                    .unwrap_or(existing.exchange_id)
                    .trim()
                    .to_string(),
                strategy_id: req
                    .strategy_id
                    .unwrap_or(existing.strategy_id)
                    .trim()
                    .to_string(),
                initial_balance: req
                    .initial_balance
                    .unwrap_or(existing.initial_balance)
                    .max(0.0),
                scan_interval_minutes: req
                    .scan_interval_minutes
                    .unwrap_or(existing.scan_interval_minutes)
                    .max(1),
                is_cross_margin: req.is_cross_margin.unwrap_or(existing.is_cross_margin != 0),
                btc_eth_leverage: btc_lev,
                altcoin_leverage: alt_lev,
                trading_symbols: req
                    .trading_symbols
                    .unwrap_or(existing.trading_symbols)
                    .trim()
                    .to_string(),
                use_ai500: req.use_ai500.unwrap_or(existing.use_ai500 != 0),
                use_oi_top: req.use_oi_top.unwrap_or(existing.use_oi_top != 0),
                custom_prompt: req
                    .custom_prompt
                    .unwrap_or(existing.custom_prompt)
                    .trim()
                    .to_string(),
                override_base_prompt: req
                    .override_base_prompt
                    .unwrap_or(existing.override_base_prompt != 0),
                system_prompt_template: req
                    .system_prompt_template
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

pub async fn delete_trader(
    app: &SharedState,
    trading_runtime_service: &TradingRuntimeService,
    id: &str,
) -> AppResult<TraderMessagePayload> {
    if let Err(err) = trading_runtime_service
        .stop_trader_for_user(id)
        .await
    {
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
    match trading_runtime_service
        .stop_trader_for_user(id)
        .await
    {
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
    req: UpdatePromptRequest,
) -> AppResult<TraderMessagePayload> {
    let result = app
        .trading_repo
        .update_prompt(
            id,
            req.custom_prompt.trim().to_string(),
            req.override_base_prompt,
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
