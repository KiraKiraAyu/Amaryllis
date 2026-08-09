use std::sync::Arc;

pub use crate::clients::llm_chat::LlmMessage;

use crate::{
    clients::llm_chat::{DefaultLlmClient, provider_config},
    error::{AppError, Result},
    repositories::{ModelRepo, models::ResolvedModelRecord},
};

#[derive(Debug, Clone)]
pub struct LlmService {
    model_repo: Arc<ModelRepo>,
}

impl LlmService {
    pub fn new(model_repo: Arc<ModelRepo>) -> Self {
        Self { model_repo }
    }

    pub async fn resolve_for_user(&self, model_id: Option<&str>) -> Result<ResolvedModelRecord> {
        self.model_repo
            .resolve_for_user(model_id)
            .await
            .map_err(|err| AppError::Internal(format!("Failed to load LLM configuration: {err}")))?
            .ok_or_else(|| {
                AppError::BadRequest(
                    "No LLM model is available. Configure one in Settings first.".into(),
                )
            })
    }

    pub async fn list_runnable_for_user(&self) -> Result<Vec<ResolvedModelRecord>> {
        self.model_repo
            .list_runnable_for_user()
            .await
            .map_err(|err| AppError::Internal(format!("Failed to load runnable models: {err}")))
    }

    pub async fn chat_for_user(
        &self,
        model_id: Option<&str>,
        messages: Vec<LlmMessage>,
        system_prompt: Option<&str>,
    ) -> Result<String> {
        let model = self.resolve_for_user(model_id).await?;
        self.chat_with_model(&model, messages, system_prompt).await
    }

    pub async fn chat_with_model(
        &self,
        model: &ResolvedModelRecord,
        messages: Vec<LlmMessage>,
        system_prompt: Option<&str>,
    ) -> Result<String> {
        if model.api_key.trim().is_empty() {
            return Err(AppError::BadRequest(
                "Selected LLM provider has no API key configured".into(),
            ));
        }

        let client = DefaultLlmClient::new(provider_config(
            model.provider_type.clone(),
            model.api_key.clone(),
            model.model_id.clone(),
            model.base_url.clone(),
        ))?;

        client.chat(messages, system_prompt).await
    }

    pub async fn chat_with_config(
        &self,
        provider: String,
        api_key: String,
        model: String,
        base_url: String,
        messages: Vec<LlmMessage>,
        system_prompt: Option<&str>,
    ) -> Result<String> {
        if api_key.trim().is_empty() {
            return Err(AppError::BadRequest(
                "Selected LLM provider has no API key configured".into(),
            ));
        }

        let client = DefaultLlmClient::new(provider_config(provider, api_key, model, base_url))?;
        client.chat(messages, system_prompt).await
    }

    /// Stream chat completion chunks through `chunk_tx` and return the full
    /// accumulated response when the stream completes.
    pub async fn chat_stream_with_config(
        &self,
        provider: String,
        api_key: String,
        model: String,
        base_url: String,
        messages: Vec<LlmMessage>,
        system_prompt: Option<&str>,
        chunk_tx: tokio::sync::mpsc::UnboundedSender<String>,
    ) -> Result<String> {
        if api_key.trim().is_empty() {
            return Err(AppError::BadRequest(
                "Selected LLM provider has no API key configured".into(),
            ));
        }

        let client = DefaultLlmClient::new(provider_config(provider, api_key, model, base_url))?;
        client.chat_stream(messages, system_prompt, chunk_tx).await
    }

    pub fn is_supported_provider(&self, provider_type: &str) -> bool {
        crate::clients::llm_chat::is_supported_provider(provider_type)
    }
}
