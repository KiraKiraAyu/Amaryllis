use reqwest::{Client, Method};
use serde::Deserialize;

use crate::{
    clients::outbound_http::{OutboundRequestLog, send_text},
    error::Result,
};

use super::{
    AvailableLlmModel, LlmClientConfig,
    urls::openai_models_url,
    util::{dedupe_models, provider_api_error},
};

/// Shared `/models` catalog for the OpenAI API family. Chat Completions and
/// Responses are distinct wire protocols but expose the same model listing
/// endpoint, so both clients delegate here.
pub(super) async fn list_openai_models(
    http: &Client,
    config: &LlmClientConfig,
) -> Result<Vec<AvailableLlmModel>> {
    let url = openai_models_url(&config.base_url);
    let response = send_text(
        http.get(&url).bearer_auth(&config.api_key),
        OutboundRequestLog::new("llm.openai_catalog.list_models", Method::GET, &url),
    )
    .await?;

    if !response.status.is_success() {
        return Err(provider_api_error(
            &config.provider,
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
