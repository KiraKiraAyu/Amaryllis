use std::{sync::Arc, time::Duration};

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

mod anthropic;
mod gemini;
mod openai;
mod urls;
mod util;

use anthropic::AnthropicClient;
use gemini::GeminiClient;
use openai::OpenAiCompatibleClient;
use urls::{default_base_url, is_openai_compatible_url, normalize_base_url};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct LlmClientConfig {
    pub provider: String,
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

#[derive(Debug, Clone)]
pub struct AvailableLlmModel {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy)]
pub struct SupportedProviderType {
    pub provider_type: &'static str,
    pub name: &'static str,
}

#[derive(Clone, Debug)]
pub struct DefaultLlmClient {
    config: LlmClientConfig,
    provider_client: Arc<dyn LlmProviderClient>,
}

#[async_trait::async_trait]
pub(super) trait LlmProviderClient: Send + Sync + std::fmt::Debug {
    async fn chat(&self, messages: Vec<LlmMessage>, system_prompt: Option<&str>) -> Result<String>;

    /// Stream chat completion chunks through `chunk_tx` and return the full
    /// accumulated response when the stream completes.
    async fn chat_stream(
        &self,
        messages: Vec<LlmMessage>,
        system_prompt: Option<&str>,
        chunk_tx: tokio::sync::mpsc::UnboundedSender<String>,
    ) -> Result<String>;

    async fn list_models(&self) -> Result<Vec<AvailableLlmModel>>;

    async fn check_model(&self) -> Result<()>;
}

impl DefaultLlmClient {
    pub fn new(config: LlmClientConfig) -> Result<Self> {
        if !is_supported_provider(&config.provider) {
            return Err(AppError::BadRequest(format!(
                "Unsupported LLM provider: {}",
                config.provider
            )));
        }

        let http = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_else(|_| Client::new());

        let provider_client = provider_client_for(http, config.clone());

        Ok(Self {
            config,
            provider_client,
        })
    }

    pub async fn chat(
        &self,
        messages: Vec<LlmMessage>,
        system_prompt: Option<&str>,
    ) -> Result<String> {
        self.provider_client.chat(messages, system_prompt).await
    }

    /// Stream chat completion chunks through `chunk_tx` and return the full
    /// accumulated response when the stream completes.
    pub async fn chat_stream(
        &self,
        messages: Vec<LlmMessage>,
        system_prompt: Option<&str>,
        chunk_tx: tokio::sync::mpsc::UnboundedSender<String>,
    ) -> Result<String> {
        self.provider_client
            .chat_stream(messages, system_prompt, chunk_tx)
            .await
    }

    pub async fn list_models(&self) -> Result<Vec<AvailableLlmModel>> {
        self.provider_client.list_models().await
    }

    pub async fn check_model(&self) -> Result<()> {
        if self.config.model.trim().is_empty() {
            return self.provider_client.list_models().await.map(|_| ());
        }

        self.provider_client.check_model().await
    }

    pub async fn check_provider(&self) -> Result<()> {
        if self.config.model.trim().is_empty() {
            return self.provider_client.list_models().await.map(|_| ());
        }

        self.chat(
            vec![LlmMessage {
                role: "user".to_string(),
                content: "ping".to_string(),
            }],
            None,
        )
        .await
        .map(|_| ())
    }
}

fn provider_client_for(http: Client, config: LlmClientConfig) -> Arc<dyn LlmProviderClient> {
    match normalize_provider_type(&config.provider) {
        "anthropic" => Arc::new(AnthropicClient::new(http, config)),
        "gemini" if !is_openai_compatible_url(&config.base_url) => {
            Arc::new(GeminiClient::new(http, config))
        }
        _ => Arc::new(OpenAiCompatibleClient::new(http, config)),
    }
}

pub fn provider_config(
    provider: String,
    api_key: String,
    model: String,
    base_url: String,
) -> LlmClientConfig {
    let provider = normalize_provider_type(&provider).to_string();
    let default_url = default_base_url(&provider);
    LlmClientConfig {
        provider,
        api_key,
        model,
        base_url: normalize_base_url(base_url, default_url),
    }
}

pub fn is_supported_provider(provider: &str) -> bool {
    let normalized = normalize_provider_type(provider);
    SUPPORTED_PROVIDER_TYPES
        .iter()
        .any(|provider_type| provider_type.provider_type == normalized)
}

pub fn supported_provider_types() -> &'static [SupportedProviderType] {
    SUPPORTED_PROVIDER_TYPES
}

pub fn normalize_provider_type(provider: &str) -> &'static str {
    match provider.trim().to_ascii_lowercase().as_str() {
        "claude" | "anthropic" => "anthropic",
        "gemini" | "google" => "gemini",
        "openai" | "openai-compatible" | "deepseek" | "qwen" | "grok" | "xai" | "kimi"
        | "moonshot" => "openai",
        _ => "",
    }
}

const SUPPORTED_PROVIDER_TYPES: &[SupportedProviderType] = &[
    SupportedProviderType {
        provider_type: "openai",
        name: "OpenAI",
    },
    SupportedProviderType {
        provider_type: "anthropic",
        name: "Anthropic",
    },
    SupportedProviderType {
        provider_type: "gemini",
        name: "Gemini",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_vendor_aliases_to_api_categories() {
        for provider in [
            "openai",
            "openai-compatible",
            "deepseek",
            "qwen",
            "grok",
            "kimi",
        ] {
            assert_eq!(normalize_provider_type(provider), "openai");
        }

        assert_eq!(normalize_provider_type("claude"), "anthropic");
        assert_eq!(normalize_provider_type("anthropic"), "anthropic");
        assert_eq!(normalize_provider_type("gemini"), "gemini");
        assert_eq!(normalize_provider_type("google"), "gemini");
        assert_eq!(normalize_provider_type("unknown"), "");
    }

    #[test]
    fn provider_config_uses_api_category_defaults() {
        let config = provider_config(
            "deepseek".to_string(),
            "key".to_string(),
            "deepseek-chat".to_string(),
            String::new(),
        );

        assert_eq!(config.provider, "openai");
        assert_eq!(config.base_url, "https://api.openai.com/v1");

        let config = provider_config(
            "gemini".to_string(),
            "key".to_string(),
            "gemini-2.0-flash".to_string(),
            String::new(),
        );

        assert_eq!(config.provider, "gemini");
        assert_eq!(
            config.base_url,
            "https://generativelanguage.googleapis.com/v1beta"
        );
    }
}
