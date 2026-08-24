use std::sync::OnceLock;
use std::time::Instant;

use reqwest::{Client, Method, RequestBuilder, StatusCode};
use serde_json::Value;
use tracing::{info, warn};

static SHARED_HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

/// Returns a shared, connection-pooled `reqwest::Client`.
pub fn http_client() -> &'static Client {
    SHARED_HTTP_CLIENT.get_or_init(Client::new)
}

#[derive(Debug, Clone)]
pub struct OutboundRequestLog {
    context: &'static str,
    method: Method,
    url: String,
    body: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OutboundResponse {
    pub status: StatusCode,
    pub body: String,
}

impl OutboundRequestLog {
    pub fn new(context: &'static str, method: Method, url: impl Into<String>) -> Self {
        Self {
            context,
            method,
            url: url.into(),
            body: None,
        }
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }
}

pub async fn send_text(
    request: RequestBuilder,
    meta: OutboundRequestLog,
) -> Result<OutboundResponse, reqwest::Error> {
    let started = Instant::now();
    let url = redact_url(&meta.url);
    let request_body = meta.body.as_deref().map(redact_body);

    info!(
        target: "quantaura::outbound_http",
        context = meta.context,
        method = %meta.method,
        url = %url,
        body = request_body.as_deref().unwrap_or(""),
        "outbound http request"
    );

    let response = match request.send().await {
        Ok(response) => response,
        Err(err) => {
            warn!(
                target: "quantaura::outbound_http",
                context = meta.context,
                method = %meta.method,
                url = %url,
                elapsed_ms = started.elapsed().as_millis() as u64,
                error = %err,
                "outbound http error"
            );
            return Err(err);
        }
    };

    let status = response.status();
    let body = match response.text().await {
        Ok(body) => body,
        Err(err) => {
            warn!(
                target: "quantaura::outbound_http",
                context = meta.context,
                method = %meta.method,
                url = %url,
                status = status.as_u16(),
                elapsed_ms = started.elapsed().as_millis() as u64,
                error = %err,
                "outbound http response read error"
            );
            return Err(err);
        }
    };
    let response_body = redact_body(&body);

    info!(
        target: "quantaura::outbound_http",
        context = meta.context,
        method = %meta.method,
        url = %url,
        status = status.as_u16(),
        elapsed_ms = started.elapsed().as_millis() as u64,
        body = %response_body,
        "outbound http response"
    );

    Ok(OutboundResponse { status, body })
}

pub fn body_preview(body: impl AsRef<[u8]>) -> String {
    truncate_text(&String::from_utf8_lossy(body.as_ref()))
}

fn redact_url(url: &str) -> String {
    let Some((base, query)) = url.split_once('?') else {
        return truncate_text(url);
    };

    let redacted_query = query
        .split('&')
        .map(|part| {
            let Some((key, value)) = part.split_once('=') else {
                return redact_query_value(part, "");
            };
            redact_query_value(key, value)
        })
        .collect::<Vec<_>>()
        .join("&");

    truncate_text(&format!("{base}?{redacted_query}"))
}

fn redact_query_value(key: &str, value: &str) -> String {
    if is_sensitive_key(key) {
        format!("{key}=<redacted>")
    } else if value.is_empty() {
        key.to_string()
    } else {
        format!("{key}={value}")
    }
}

fn redact_body(body: &str) -> String {
    match serde_json::from_str::<Value>(body) {
        Ok(mut value) => {
            redact_json_value(&mut value);
            truncate_text(&value.to_string())
        }
        Err(_) => truncate_text(body),
    }
}

fn redact_json_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, value) in map.iter_mut() {
                if is_sensitive_key(key) {
                    *value = Value::String("<redacted>".to_string());
                } else {
                    redact_json_value(value);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                redact_json_value(value);
            }
        }
        _ => {}
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.contains("authorization")
        || key.contains("api_key")
        || key.contains("apikey")
        || key.contains("x-mbx-apikey")
        || key == "key"
        || key.contains("secret")
        || key.contains("signature")
        || key.contains("token")
        || key.contains("listenkey")
        || key.contains("listen_key")
        || key.contains("password")
}

fn truncate_text(value: &str) -> String {
    const MAX_CHARS: usize = 4096;

    let mut out = String::new();
    for (idx, ch) in value.chars().enumerate() {
        if idx >= MAX_CHARS {
            out.push_str("...<truncated>");
            return out;
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_sensitive_url_query_values() {
        let url = redact_url(
            "https://example.com/path?symbol=BTCUSDT&signature=abc&listenKey=secret&key=token",
        );

        assert!(url.contains("symbol=BTCUSDT"));
        assert!(url.contains("signature=<redacted>"));
        assert!(url.contains("listenKey=<redacted>"));
        assert!(url.contains("key=<redacted>"));
        assert!(!url.contains("abc"));
        assert!(!url.contains("secret"));
        assert!(!url.contains("token"));
    }

    #[test]
    fn redacts_sensitive_json_body_values() {
        let body = redact_body(
            r#"{"model":"gpt","api_key":"secret","nested":{"Authorization":"Bearer token"}}"#,
        );

        assert!(body.contains("\"model\":\"gpt\""));
        assert!(body.contains("\"api_key\":\"<redacted>\""));
        assert!(body.contains("\"Authorization\":\"<redacted>\""));
        assert!(!body.contains("secret"));
        assert!(!body.contains("Bearer token"));
    }

    #[test]
    fn http_client_singleton_is_stable_and_cached() {
        let client1 = http_client();
        let client2 = http_client();
        assert!(std::ptr::eq(client1, client2));
    }
}
