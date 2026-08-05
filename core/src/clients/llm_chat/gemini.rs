use futures_util::StreamExt;
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    clients::outbound_http::{OutboundRequestLog, send_text},
    error::{AppError, Result},
};

use super::{
    AvailableLlmModel, LlmClientConfig, LlmMessage, LlmProviderClient,
    urls::{
        gemini_generate_content_url, gemini_model_url, gemini_models_url,
        gemini_stream_generate_content_url,
    },
    util::{dedupe_models, non_empty_text, normalize_message_role, provider_api_error},
};

#[derive(Clone, Debug)]
pub(super) struct GeminiClient {
    http: Client,
    config: LlmClientConfig,
}

impl GeminiClient {
    pub(super) fn new(http: Client, config: LlmClientConfig) -> Self {
        Self { http, config }
    }
}

#[derive(Debug, Deserialize)]
struct GeminiModelsResponse {
    models: Vec<GeminiModelInfo>,
}

#[derive(Debug, Deserialize)]
struct GeminiModelInfo {
    name: String,
    #[serde(default, rename = "baseModelId")]
    base_model_id: Option<String>,
    #[serde(default, rename = "displayName")]
    display_name: Option<String>,
    #[serde(default, rename = "supportedGenerationMethods")]
    supported_generation_methods: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiRequestPayload {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiSystemInstruction>,
    generation_config: GeminiGenerationConfig,
}

#[derive(Debug, Clone, Serialize)]
struct GeminiSystemInstruction {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Clone, Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiGenerationConfig {
    temperature: f32,
    max_output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct GeminiResponsePayload {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiResponseContent,
}

#[derive(Debug, Deserialize)]
struct GeminiResponseContent {
    parts: Vec<GeminiPart>,
}

#[async_trait::async_trait]
impl LlmProviderClient for GeminiClient {
    async fn list_models(&self) -> Result<Vec<AvailableLlmModel>> {
        let url = gemini_models_url(&self.config.base_url);
        let response = send_text(
            self.http
                .get(&url)
                .query(&[("key", self.config.api_key.as_str()), ("pageSize", "1000")]),
            OutboundRequestLog::new(
                "llm.gemini.list_models",
                Method::GET,
                format!("{url}?key=<redacted>&pageSize=1000"),
            ),
        )
        .await?;

        if !response.status.is_success() {
            return Err(provider_api_error(
                &self.config.provider,
                response.status,
                response.body,
            ));
        }

        let parsed: GeminiModelsResponse = serde_json::from_str(&response.body)?;
        Ok(dedupe_models(
            parsed
                .models
                .into_iter()
                .filter(|model| {
                    model.supported_generation_methods.is_empty()
                        || model
                            .supported_generation_methods
                            .iter()
                            .any(|method| method == "generateContent")
                })
                .map(|model| {
                    let id = gemini_model_id(&model);
                    let name = model.display_name.unwrap_or_else(|| id.clone());
                    AvailableLlmModel { id, name }
                }),
        ))
    }

    async fn check_model(&self) -> Result<()> {
        let url = gemini_model_url(&self.config.base_url, &self.config.model);
        let response = send_text(
            self.http
                .get(&url)
                .query(&[("key", self.config.api_key.as_str())]),
            OutboundRequestLog::new(
                "llm.gemini.check_model",
                Method::GET,
                format!("{url}?key=<redacted>"),
            ),
        )
        .await?;

        if response.status.is_success() {
            return Ok(());
        }

        Err(provider_api_error(
            &self.config.provider,
            response.status,
            response.body,
        ))
    }

    async fn chat(&self, messages: Vec<LlmMessage>, system_prompt: Option<&str>) -> Result<String> {
        let (contents, system_instruction) = gemini_contents(messages, system_prompt);
        let payload = GeminiRequestPayload {
            contents,
            system_instruction,
            generation_config: GeminiGenerationConfig {
                temperature: 0.7,
                max_output_tokens: 1024,
            },
        };
        let body = serde_json::to_string(&payload)?;
        let url = gemini_generate_content_url(&self.config.base_url, &self.config.model);

        let response = send_text(
            self.http
                .post(&url)
                .header("x-goog-api-key", &self.config.api_key)
                .json(&payload),
            OutboundRequestLog::new("llm.gemini.chat", Method::POST, &url).body(body),
        )
        .await?;

        if !response.status.is_success() {
            return Err(provider_api_error(
                &self.config.provider,
                response.status,
                response.body,
            ));
        }

        let parsed: GeminiResponsePayload = serde_json::from_str(&response.body)?;
        parsed
            .candidates
            .first()
            .and_then(|candidate| {
                candidate
                    .content
                    .parts
                    .iter()
                    .find_map(|part| non_empty_text(&part.text))
            })
            .ok_or_else(|| {
                AppError::BadGateway(format!("No response from {}", self.config.provider))
            })
    }

    async fn chat_stream(
        &self,
        messages: Vec<LlmMessage>,
        system_prompt: Option<&str>,
        chunk_tx: UnboundedSender<String>,
    ) -> Result<String> {
        let (contents, system_instruction) = gemini_contents(messages, system_prompt);
        let payload = GeminiRequestPayload {
            contents,
            system_instruction,
            generation_config: GeminiGenerationConfig {
                temperature: 0.7,
                max_output_tokens: 1024,
            },
        };
        let url = gemini_stream_generate_content_url(&self.config.base_url, &self.config.model);

        let response = self
            .http
            .post(&url)
            .header("x-goog-api-key", &self.config.api_key)
            .json(&payload)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(provider_api_error(
                &self.config.provider,
                status,
                body,
            ));
        }

        let mut full_response = String::new();
        let mut stream = response.bytes_stream();
        let mut line_buf = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            line_buf.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(newline_pos) = line_buf.find('\n') {
                let line = line_buf[..newline_pos].trim().to_string();
                line_buf = line_buf[newline_pos + 1..].to_string();

                if line.is_empty() || line.starts_with(':') {
                    continue;
                }

                if let Some(data) = line.strip_prefix("data: ") {
                    let data = data.trim();
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(text) = parsed
                            .get("candidates")
                            .and_then(|c| c.get(0))
                            .and_then(|c| c.get("content"))
                            .and_then(|c| c.get("parts"))
                            .and_then(|p| p.get(0))
                            .and_then(|p| p.get("text"))
                            .and_then(|t| t.as_str())
                        {
                            if !text.is_empty() {
                                full_response.push_str(text);
                                let _ = chunk_tx.send(text.to_string());
                            }
                        }
                    }
                }
            }
        }

        if full_response.is_empty() {
            return Err(AppError::BadGateway(format!(
                "Empty stream response from {}",
                self.config.provider
            )));
        }

        Ok(full_response)
    }
}

fn gemini_contents(
    messages: Vec<LlmMessage>,
    system_prompt: Option<&str>,
) -> (Vec<GeminiContent>, Option<GeminiSystemInstruction>) {
    let mut contents = Vec::new();
    let mut system_parts = Vec::new();

    if let Some(prompt) = system_prompt.and_then(non_empty_text) {
        system_parts.push(GeminiPart { text: prompt });
    }

    for message in messages {
        let Some(text) = non_empty_text(&message.content) else {
            continue;
        };

        match normalize_message_role(&message.role) {
            "system" => system_parts.push(GeminiPart { text }),
            "assistant" => contents.push(GeminiContent {
                role: "model".to_string(),
                parts: vec![GeminiPart { text }],
            }),
            _ => contents.push(GeminiContent {
                role: "user".to_string(),
                parts: vec![GeminiPart { text }],
            }),
        }
    }

    let system_instruction = if system_parts.is_empty() {
        None
    } else {
        Some(GeminiSystemInstruction {
            parts: system_parts,
        })
    };

    (contents, system_instruction)
}

fn gemini_model_id(model: &GeminiModelInfo) -> String {
    model
        .base_model_id
        .as_deref()
        .filter(|id| !id.trim().is_empty())
        .unwrap_or_else(|| model.name.trim().trim_start_matches("models/"))
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gemini_content_uses_native_roles_and_system_instruction() {
        let (contents, system_instruction) = gemini_contents(
            vec![
                LlmMessage {
                    role: "system".to_string(),
                    content: "system".to_string(),
                },
                LlmMessage {
                    role: "assistant".to_string(),
                    content: "prior answer".to_string(),
                },
                LlmMessage {
                    role: "user".to_string(),
                    content: "question".to_string(),
                },
            ],
            None,
        );

        assert_eq!(
            system_instruction.expect("system instruction").parts[0].text,
            "system"
        );
        assert_eq!(contents[0].role, "model");
        assert_eq!(contents[0].parts[0].text, "prior answer");
        assert_eq!(contents[1].role, "user");
        assert_eq!(contents[1].parts[0].text, "question");
    }
}
