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
    urls::{chat_completions_models_url, responses_url},
    util::{dedupe_models, provider_api_error, with_system_prompt},
};

#[derive(Clone, Debug)]
pub(super) struct ResponsesClient {
    http: Client,
    config: LlmClientConfig,
}

impl ResponsesClient {
    pub(super) fn new(http: Client, config: LlmClientConfig) -> Self {
        Self { http, config }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ResponsesRequestPayload {
    model: String,
    input: Vec<LlmMessage>,
    stream: bool,
    temperature: f32,
    max_output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ResponsesOutput {
    #[serde(rename = "type")]
    output_type: String,
    content: Vec<ResponsesContent>,
}

#[derive(Debug, Deserialize)]
struct ResponsesContent {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResponsesPayload {
    output: Vec<ResponsesOutput>,
}

#[derive(Debug, Deserialize)]
struct OpenAiModelsResponse {
    data: Vec<OpenAiModelInfo>,
}

#[derive(Debug, Deserialize)]
struct OpenAiModelInfo {
    id: String,
    #[serde(default)]
    name: Option<String>,
}

#[async_trait::async_trait]
impl LlmProviderClient for ResponsesClient {
    async fn list_models(&self) -> Result<Vec<AvailableLlmModel>> {
        let url = chat_completions_models_url(&self.config.base_url);
        let response = send_text(
            self.http.get(&url).bearer_auth(&self.config.api_key),
            OutboundRequestLog::new("llm.responses.list_models", Method::GET, &url),
        )
        .await?;

        if !response.status.is_success() {
            return Err(provider_api_error(
                &self.config.provider,
                response.status,
                response.body,
            ));
        }

        let parsed: OpenAiModelsResponse = serde_json::from_str(&response.body)?;
        Ok(dedupe_models(parsed.data.into_iter().map(|model| {
            let name = model.name.unwrap_or_else(|| model.id.clone());
            AvailableLlmModel { id: model.id, name }
        })))
    }

    async fn check_model(&self) -> Result<()> {
        let payload = ResponsesRequestPayload {
            model: self.config.model.clone(),
            input: vec![LlmMessage {
                role: "user".to_string(),
                content: "ping".to_string(),
            }],
            stream: false,
            temperature: 0.0,
            max_output_tokens: 16,
        };
        let body = serde_json::to_string(&payload)?;
        let url = responses_url(&self.config.base_url);

        let response = send_text(
            self.http
                .post(&url)
                .bearer_auth(&self.config.api_key)
                .json(&payload),
            OutboundRequestLog::new("llm.responses.check_provider", Method::POST, &url).body(body),
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
        let payload = ResponsesRequestPayload {
            model: self.config.model.clone(),
            input: with_system_prompt(messages, system_prompt),
            stream: false,
            temperature: 0.7,
            max_output_tokens: 1024,
        };
        let body = serde_json::to_string(&payload)?;
        let url = responses_url(&self.config.base_url);

        let response = send_text(
            self.http
                .post(&url)
                .bearer_auth(&self.config.api_key)
                .json(&payload),
            OutboundRequestLog::new("llm.responses.chat", Method::POST, &url).body(body),
        )
        .await?;

        if !response.status.is_success() {
            return Err(provider_api_error(
                &self.config.provider,
                response.status,
                response.body,
            ));
        }

        let parsed: ResponsesPayload = serde_json::from_str(&response.body)?;
        collect_output_text(&parsed.output).ok_or_else(|| {
            AppError::BadGateway(format!("No response from {}", self.config.provider))
        })
    }

    async fn chat_stream(
        &self,
        messages: Vec<LlmMessage>,
        system_prompt: Option<&str>,
        chunk_tx: UnboundedSender<String>,
    ) -> Result<String> {
        let payload = ResponsesRequestPayload {
            model: self.config.model.clone(),
            input: with_system_prompt(messages, system_prompt),
            stream: true,
            temperature: 0.7,
            max_output_tokens: 1024,
        };
        let url = responses_url(&self.config.base_url);

        let response = self
            .http
            .post(&url)
            .bearer_auth(&self.config.api_key)
            .json(&payload)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(provider_api_error(&self.config.provider, status, body));
        }

        let mut full_response = String::new();
        let mut stream = response.bytes_stream();
        let mut line_buf = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            line_buf.push_str(&String::from_utf8_lossy(&chunk));

            // Process complete SSE lines
            while let Some(newline_pos) = line_buf.find('\n') {
                let line = line_buf[..newline_pos].trim().to_string();
                line_buf = line_buf[newline_pos + 1..].to_string();

                if line.is_empty() || line.starts_with(':') {
                    continue;
                }

                if let Some(data) = line.strip_prefix("data: ") {
                    let data = data.trim();
                    if data == "[DONE]" {
                        continue;
                    }

                    // Responses API streams typed events; text deltas arrive
                    // as `response.output_text.delta`.
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                        if parsed.get("type").and_then(|t| t.as_str())
                            != Some("response.output_text.delta")
                        {
                            continue;
                        }

                        if let Some(delta) = parsed
                            .get("delta")
                            .and_then(|d| d.as_str())
                            .filter(|delta| !delta.is_empty())
                        {
                            full_response.push_str(delta);
                            let _ = chunk_tx.send(delta.to_string());
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

fn collect_output_text(output: &[ResponsesOutput]) -> Option<String> {
    let text = output
        .iter()
        .filter(|item| item.output_type == "message")
        .flat_map(|item| item.content.iter())
        .filter(|content| content.content_type == "output_text")
        .filter_map(|content| content.text.as_deref())
        .collect::<Vec<_>>()
        .join("");

    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_message_output_text_and_skips_reasoning_items() {
        let output = vec![
            ResponsesOutput {
                output_type: "reasoning".to_string(),
                content: vec![ResponsesContent {
                    content_type: "output_text".to_string(),
                    text: Some("hidden reasoning".to_string()),
                }],
            },
            ResponsesOutput {
                output_type: "message".to_string(),
                content: vec![
                    ResponsesContent {
                        content_type: "output_text".to_string(),
                        text: Some("Hello ".to_string()),
                    },
                    ResponsesContent {
                        content_type: "refusal".to_string(),
                        text: None,
                    },
                    ResponsesContent {
                        content_type: "output_text".to_string(),
                        text: Some("world".to_string()),
                    },
                ],
            },
        ];

        assert_eq!(
            collect_output_text(&output).as_deref(),
            Some("Hello world")
        );
    }

    #[test]
    fn collect_output_text_rejects_empty_output() {
        assert_eq!(collect_output_text(&[]), None);
        assert_eq!(
            collect_output_text(&[ResponsesOutput {
                output_type: "message".to_string(),
                content: vec![],
            }]),
            None
        );
    }
}
