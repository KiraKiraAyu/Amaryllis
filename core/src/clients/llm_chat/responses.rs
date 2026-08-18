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
    openai_catalog::list_openai_models,
    sse::SseLineReader,
    urls::responses_url,
    util::{provider_api_error, with_system_prompt},
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

/// Output items are tagged by `type`. Only `message` items carry `content`;
/// `reasoning` items (and any type added in the future) must not break
/// deserialization, so unknown tags fall back to `Other`.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ResponsesOutputItem {
    #[serde(rename = "message")]
    Message { content: Vec<ResponsesContent> },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct ResponsesContent {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResponsesPayload {
    #[serde(default)]
    status: Option<String>,
    output: Vec<ResponsesOutputItem>,
}

#[async_trait::async_trait]
impl LlmProviderClient for ResponsesClient {
    async fn list_models(&self) -> Result<Vec<AvailableLlmModel>> {
        list_openai_models(&self.http, &self.config).await
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
        if let Some(status) = parsed.status.as_deref()
            && status != "completed"
        {
            return Err(AppError::BadGateway(format!(
                "{} API returned response status '{status}'",
                self.config.provider
            )));
        }

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
        let mut reader = SseLineReader::default();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            reader.push(&chunk);

            while let Some(line) = reader.next_line() {
                if line.is_empty() || line.starts_with(':') {
                    continue;
                }

                let Some(data) = line.strip_prefix("data: ") else {
                    continue;
                };
                let data = data.trim();
                if data == "[DONE]" {
                    continue;
                }

                let parsed: serde_json::Value = serde_json::from_str(data).map_err(|err| {
                    AppError::BadGateway(format!(
                        "Malformed SSE event from {} API: {err}",
                        self.config.provider
                    ))
                })?;

                match decode_stream_event(&parsed) {
                    StreamEventOutcome::Delta(delta) => {
                        full_response.push_str(&delta);
                        let _ = chunk_tx.send(delta);
                    }
                    StreamEventOutcome::Failure(message) => {
                        return Err(AppError::BadGateway(format!(
                            "{} API stream {message}",
                            self.config.provider
                        )));
                    }
                    StreamEventOutcome::Ignored => {}
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

#[derive(Debug)]
enum StreamEventOutcome {
    Delta(String),
    Failure(String),
    Ignored,
}

/// Terminal events (`response.failed`, `response.incomplete`, `error`) must
/// surface as errors even after text deltas were already received, otherwise
/// a truncated response would be reported as success.
fn decode_stream_event(parsed: &serde_json::Value) -> StreamEventOutcome {
    match parsed.get("type").and_then(|value| value.as_str()) {
        Some("response.output_text.delta") => match parsed
            .get("delta")
            .and_then(|delta| delta.as_str())
            .filter(|delta| !delta.is_empty())
        {
            Some(delta) => StreamEventOutcome::Delta(delta.to_string()),
            None => StreamEventOutcome::Ignored,
        },
        Some("response.failed") => {
            StreamEventOutcome::Failure(stream_failure_message(parsed, "failed"))
        }
        Some("response.incomplete") => {
            StreamEventOutcome::Failure(stream_failure_message(parsed, "incomplete"))
        }
        Some("error") => StreamEventOutcome::Failure(stream_failure_message(parsed, "error")),
        _ => StreamEventOutcome::Ignored,
    }
}

fn stream_failure_message(parsed: &serde_json::Value, kind: &str) -> String {
    // Terminal `response.*` events nest the payload under `response`, while
    // top-level `error` events carry their message directly.
    let scope = parsed.get("response").unwrap_or(parsed);
    let reason = scope
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(|message| message.as_str())
        .or_else(|| scope.get("message").and_then(|message| message.as_str()))
        .or_else(|| {
            scope
                .get("incomplete_details")
                .and_then(|details| details.get("reason"))
                .and_then(|reason| reason.as_str())
        });

    match reason {
        Some(reason) => format!("{kind}: {reason}"),
        None => kind.to_string(),
    }
}

fn collect_output_text(output: &[ResponsesOutputItem]) -> Option<String> {
    let text = output
        .iter()
        .filter_map(|item| match item {
            ResponsesOutputItem::Message { content } => Some(content.iter()),
            ResponsesOutputItem::Other => None,
        })
        .flatten()
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

    const REAL_RESPONSE_JSON: &str = r#"{
        "id": "resp_68af4030592c81938ec0a5fbab4a3e9f05438e46b5f69a3b",
        "object": "response",
        "created_at": 1756315696,
        "status": "completed",
        "model": "gpt-5.5",
        "output": [
            {
                "id": "rs_68af4030baa48193b0b43b4c2a176a1a05438e46b5f69a3b",
                "type": "reasoning",
                "summary": []
            },
            {
                "id": "msg_68af40337e58819392e935fb404414d004414d004414d00",
                "type": "message",
                "status": "completed",
                "role": "assistant",
                "content": [
                    {
                        "type": "output_text",
                        "annotations": [],
                        "logprobs": [],
                        "text": "Under a quilt of moonlight."
                    }
                ]
            }
        ]
    }"#;

    #[test]
    fn deserializes_official_response_shape_without_reasoning_content() {
        let parsed: ResponsesPayload =
            serde_json::from_str(REAL_RESPONSE_JSON).expect("official shape must deserialize");
        assert_eq!(parsed.status.as_deref(), Some("completed"));

        assert_eq!(
            collect_output_text(&parsed.output).as_deref(),
            Some("Under a quilt of moonlight.")
        );
    }

    #[test]
    fn collect_output_text_rejects_empty_or_untextual_output() {
        assert_eq!(collect_output_text(&[]), None);

        let refusal_only: ResponsesPayload =
            serde_json::from_str(r#"{"output": [{"type": "message", "content": [{"type": "refusal", "refusal": "no"}]}]}"#)
                .expect("refusal content must deserialize");
        assert_eq!(collect_output_text(&refusal_only.output), None);

        let reasoning_only: ResponsesPayload =
            serde_json::from_str(r#"{"output": [{"type": "reasoning", "summary": []}]}"#)
                .expect("reasoning item must deserialize");
        assert_eq!(collect_output_text(&reasoning_only.output), None);
    }

    #[test]
    fn stream_event_decoding_distinguishes_delta_failure_and_noise() {
        let delta: serde_json::Value = serde_json::json!({
            "type": "response.output_text.delta",
            "item_id": "msg_1",
            "delta": "你好"
        });
        match decode_stream_event(&delta) {
            StreamEventOutcome::Delta(text) => assert_eq!(text, "你好"),
            other => panic!("expected delta, got {other:?}"),
        }

        let completed: serde_json::Value =
            serde_json::json!({"type": "response.completed", "response": {"id": "resp_1"}});
        assert!(matches!(
            decode_stream_event(&completed),
            StreamEventOutcome::Ignored
        ));

        let empty_delta: serde_json::Value =
            serde_json::json!({"type": "response.output_text.delta", "delta": ""});
        assert!(matches!(
            decode_stream_event(&empty_delta),
            StreamEventOutcome::Ignored
        ));
    }

    #[test]
    fn stream_failure_events_report_their_reason() {
        let failed: serde_json::Value = serde_json::json!({
            "type": "response.failed",
            "response": {
                "status": "failed",
                "error": {"code": "server_error", "message": "model overloaded"}
            }
        });
        match decode_stream_event(&failed) {
            StreamEventOutcome::Failure(message) => {
                assert_eq!(message, "failed: model overloaded")
            }
            other => panic!("expected failure, got {other:?}"),
        }

        let incomplete: serde_json::Value = serde_json::json!({
            "type": "response.incomplete",
            "response": {
                "status": "incomplete",
                "incomplete_details": {"reason": "max_output_tokens"}
            }
        });
        match decode_stream_event(&incomplete) {
            StreamEventOutcome::Failure(message) => {
                assert_eq!(message, "incomplete: max_output_tokens")
            }
            other => panic!("expected failure, got {other:?}"),
        }

        let error: serde_json::Value = serde_json::json!({
            "type": "error",
            "code": "invalid_request",
            "message": "Unknown model"
        });
        match decode_stream_event(&error) {
            StreamEventOutcome::Failure(message) => assert_eq!(message, "error: Unknown model"),
            other => panic!("expected failure, got {other:?}"),
        }
    }
}
