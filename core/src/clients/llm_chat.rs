use std::{sync::Arc, time::Duration};

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::Result;

mod anthropic;
mod chat_completions;
mod gemini;
mod openai_catalog;
mod responses;
mod sse;
mod urls;
mod util;

use anthropic::AnthropicClient;
use chat_completions::ChatCompletionsClient;
use gemini::GeminiClient;
use responses::ResponsesClient;
use urls::{default_base_url, normalize_base_url};

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
    pub fn new(mut config: LlmClientConfig) -> Result<Self> {
        let normalized = normalize_provider_type(&config.provider);
        if normalized.is_empty() {
            config.provider = "chat_completions".to_string();
        } else {
            config.provider = normalized.to_string();
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
        "anthropic_messages" => Arc::new(AnthropicClient::new(http, config)),
        "gemini_generate_content" => Arc::new(GeminiClient::new(http, config)),
        "responses" => Arc::new(ResponsesClient::new(http, config)),
        _ => Arc::new(ChatCompletionsClient::new(http, config)),
    }
}

pub fn provider_config(
    provider: String,
    api_key: String,
    model: String,
    base_url: String,
) -> LlmClientConfig {
    let normalized = normalize_provider_type(&provider);
    let effective_provider = if normalized.is_empty() {
        "chat_completions"
    } else {
        normalized
    };
    let default_url = default_base_url(effective_provider);
    LlmClientConfig {
        provider: effective_provider.to_string(),
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
        || provider.trim().is_empty()
}

pub fn supported_provider_types() -> &'static [SupportedProviderType] {
    SUPPORTED_PROVIDER_TYPES
}

pub fn normalize_provider_type(provider: &str) -> &'static str {
    let normalized = provider.trim().to_ascii_lowercase().replace([' ', '-'], "_");
    match normalized.as_str() {
        "chat_completions" => "chat_completions",
        "responses" => "responses",
        "anthropic_messages" => "anthropic_messages",
        "gemini_generate_content" => "gemini_generate_content",
        _ => "",
    }
}

const SUPPORTED_PROVIDER_TYPES: &[SupportedProviderType] = &[
    SupportedProviderType {
        provider_type: "chat_completions",
        name: "Chat Completions",
    },
    SupportedProviderType {
        provider_type: "responses",
        name: "Responses",
    },
    SupportedProviderType {
        provider_type: "anthropic_messages",
        name: "Anthropic Messages",
    },
    SupportedProviderType {
        provider_type: "gemini_generate_content",
        name: "Gemini GenerateContent",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_api_category_aliases() {
        for provider in [
            "chat_completions",
            "chat-completions",
            "Chat Completions",
        ] {
            assert_eq!(normalize_provider_type(provider), "chat_completions");
        }

        assert_eq!(normalize_provider_type("responses"), "responses");
        assert_eq!(normalize_provider_type("Responses"), "responses");

        assert_eq!(
            normalize_provider_type("anthropic_messages"),
            "anthropic_messages"
        );
        assert_eq!(
            normalize_provider_type("Anthropic-Messages"),
            "anthropic_messages"
        );

        assert_eq!(
            normalize_provider_type("gemini_generate_content"),
            "gemini_generate_content"
        );
        assert_eq!(
            normalize_provider_type("Gemini-Generate-Content"),
            "gemini_generate_content"
        );
        assert_eq!(normalize_provider_type(""), "");
        assert_eq!(normalize_provider_type("unknown"), "");
    }

    #[test]
    fn provider_config_uses_api_category_defaults() {
        let config = provider_config(
            "chat_completions".to_string(),
            "key".to_string(),
            "deepseek-chat".to_string(),
            String::new(),
        );

        assert_eq!(config.provider, "chat_completions");
        assert_eq!(config.base_url, "https://api.openai.com/v1");

        let config = provider_config(
            "gemini_generate_content".to_string(),
            "key".to_string(),
            "gemini-2.0-flash".to_string(),
            String::new(),
        );

        assert_eq!(config.provider, "gemini_generate_content");
        assert_eq!(
            config.base_url,
            "https://generativelanguage.googleapis.com/v1beta"
        );
    }
}
