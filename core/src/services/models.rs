use std::sync::Arc;

use crate::{
    clients::llm_chat::provider_config,
    contracts::models::{
        AvailableModelListPayload, AvailableModelPayload, MessagePayload, ModelConfigPayload,
        ProviderAvailabilityPayload, ProviderConfigInput, SafeModelConfig, SafeProviderConfig,
    },
    error::{AppError, Result},
    repositories::{
        ModelRepo,
        models::{UpsertModelConfig, UpsertProviderConfig},
    },
    services::llm::LlmService,
};

#[derive(Debug, Clone)]
pub struct ModelService {
    repo: Arc<ModelRepo>,
    llm_service: Arc<LlmService>,
}

impl ModelService {
    pub fn new(repo: Arc<ModelRepo>, llm_service: Arc<LlmService>) -> Self {
        Self { repo, llm_service }
    }

    pub async fn list_configs(&self) -> Result<ModelConfigPayload> {
        let rows = self.repo.list_for_user().await.map_err(|err| {
            AppError::Internal(format!("Failed to get LLM configurations: {err}"))
        })?;

        Ok(ModelConfigPayload {
            providers: rows
                .into_iter()
                .map(|row| SafeProviderConfig {
                    id: row.id,
                    name: row.name,
                    provider_type: row.provider_type,
                    enabled: row.enabled != 0,
                    api_key: row.api_key,
                    base_url: row.base_url,
                    models: row
                        .models
                        .into_iter()
                        .map(|model| SafeModelConfig {
                            id: model.id,
                            provider_id: model.provider_id,
                            name: model.name,
                            model_id: model.model_id,
                        })
                        .collect(),
                })
                .collect(),
        })
    }

    pub async fn update_configs(
        &self,
        providers: Vec<ProviderConfigInput>,
    ) -> Result<MessagePayload> {
        let mut upsert_providers = Vec::new();

        for provider in providers {
            let provider_type = provider.provider_type.trim().to_ascii_lowercase();
            if provider.name.trim().is_empty() {
                return Err(AppError::BadRequest("Provider name is required".into()));
            }
            if !self.llm_service.is_supported_provider(&provider_type) {
                return Err(AppError::BadRequest("Unsupported provider type".into()));
            }

            let mut models = Vec::new();
            for model in provider.models {
                if model.name.trim().is_empty() || model.model_id.trim().is_empty() {
                    return Err(AppError::BadRequest(
                        "Model name and model_id are required".into(),
                    ));
                }
                models.push(UpsertModelConfig {
                    id: model.id,
                    name: model.name.trim().to_string(),
                    model_id: model.model_id.trim().to_string(),
                });
            }

            upsert_providers.push(UpsertProviderConfig {
                id: provider.id,
                name: provider.name.trim().to_string(),
                provider_type,
                enabled: provider.enabled,
                api_key: provider.api_key.trim().to_string(),
                base_url: provider.base_url.trim().to_string(),
                models,
            });
        }

        self.repo
            .replace_for_user(upsert_providers)
            .await
            .map_err(|err| {
                AppError::Internal(format!("Failed to update LLM configurations: {err}"))
            })?;

        Ok(MessagePayload {
            message: "LLM configuration updated",
        })
    }

    pub async fn list_available_models(
        &self,
        provider_type: String,
        api_key: String,
        base_url: String,
    ) -> Result<AvailableModelListPayload> {
        let config = self.probe_config(provider_type, api_key, String::new(), base_url)?;
        let client = crate::clients::llm_chat::DefaultLlmClient::new(config)?;
        let models = client.list_models().await?;

        Ok(AvailableModelListPayload {
            models: models
                .into_iter()
                .map(|model| AvailableModelPayload {
                    id: model.id,
                    name: model.name,
                })
                .collect(),
        })
    }

    pub async fn check_provider_availability(
        &self,
        provider_type: String,
        api_key: String,
        base_url: String,
        model_id: String,
    ) -> Result<ProviderAvailabilityPayload> {
        let config = self.probe_config(provider_type, api_key, model_id, base_url)?;
        let client = crate::clients::llm_chat::DefaultLlmClient::new(config)?;
        client.check_provider().await?;

        Ok(ProviderAvailabilityPayload {
            available: true,
            message: "Provider is available",
        })
    }

    fn probe_config(
        &self,
        provider_type: String,
        api_key: String,
        model_id: String,
        base_url: String,
    ) -> Result<crate::clients::llm_chat::LlmClientConfig> {
        let provider_type = provider_type.trim().to_ascii_lowercase();
        if !self.llm_service.is_supported_provider(&provider_type) {
            return Err(AppError::BadRequest("Unsupported provider type".into()));
        }
        if api_key.trim().is_empty() {
            return Err(AppError::BadRequest("API key is required".into()));
        }

        Ok(provider_config(
            provider_type,
            api_key.trim().to_string(),
            model_id.trim().to_string(),
            base_url.trim().to_string(),
        ))
    }
}
