use super::normalize_provider_type;

pub(super) fn default_base_url(provider: &str) -> &'static str {
    match normalize_provider_type(provider) {
        "anthropic" => "https://api.anthropic.com",
        "gemini" => "https://generativelanguage.googleapis.com/v1beta",
        "openai" => "https://api.openai.com/v1",
        _ => "",
    }
}

pub(super) fn normalize_base_url(base_url: String, default_url: &str) -> String {
    let raw = if base_url.trim().is_empty() {
        default_url
    } else {
        base_url.trim()
    };
    raw.trim_end_matches('/').to_string()
}

pub(super) fn openai_chat_url(base_url: &str) -> String {
    if base_url.ends_with("/chat/completions") {
        base_url.to_string()
    } else if base_url.ends_with("/v1") || base_url.ends_with("/openai") {
        format!("{base_url}/chat/completions")
    } else {
        format!("{base_url}/v1/chat/completions")
    }
}

pub(super) fn openai_models_url(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');
    if base.ends_with("/models") {
        base.to_string()
    } else if let Some(api_base) = base.strip_suffix("/chat/completions") {
        format!("{api_base}/models")
    } else if base.ends_with("/v1") || base.ends_with("/openai") {
        format!("{base}/models")
    } else {
        format!("{base}/v1/models")
    }
}

pub(super) fn anthropic_messages_url(base_url: &str) -> String {
    if base_url.ends_with("/messages") {
        base_url.to_string()
    } else if base_url.ends_with("/v1") {
        format!("{base_url}/messages")
    } else {
        format!("{base_url}/v1/messages")
    }
}

pub(super) fn anthropic_models_url(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');
    if base.ends_with("/models") {
        base.to_string()
    } else if let Some(api_base) = base.strip_suffix("/messages") {
        format!("{api_base}/models")
    } else if base.ends_with("/v1") {
        format!("{base}/models")
    } else {
        format!("{base}/v1/models")
    }
}

pub(super) fn anthropic_model_url(base_url: &str, model: &str) -> String {
    format!(
        "{}/{}",
        anthropic_models_url(base_url),
        encode_path_segment(model)
    )
}

pub(super) fn gemini_generate_content_url(base_url: &str, model: &str) -> String {
    let base = base_url.trim_end_matches('/');
    if base.ends_with(":generateContent") {
        return base.to_string();
    }

    if base.contains("/models/") {
        return format!("{base}:generateContent");
    }

    let model = model.trim().trim_start_matches("models/");
    format!("{base}/models/{model}:generateContent")
}

/// Gemini streaming endpoint uses `:streamGenerateContent` with `alt=sse`.
pub(super) fn gemini_stream_generate_content_url(base_url: &str, model: &str) -> String {
    let base = base_url.trim_end_matches('/');
    if base.ends_with(":streamGenerateContent") {
        return format!("{base}?alt=sse");
    }

    let model = model.trim().trim_start_matches("models/");

    if let Some(pos) = base.find("/models/") {
        let api_base = &base[..pos];
        return format!("{api_base}/models/{model}:streamGenerateContent?alt=sse");
    }

    format!("{base}/models/{model}:streamGenerateContent?alt=sse")
}

pub(super) fn gemini_models_url(base_url: &str) -> String {
    let base = gemini_api_base(base_url);
    if base.ends_with("/models") {
        base
    } else {
        format!("{base}/models")
    }
}

pub(super) fn gemini_model_url(base_url: &str, model: &str) -> String {
    let model = model.trim().trim_start_matches("models/");
    format!(
        "{}/{}",
        gemini_models_url(base_url),
        encode_path_segment(model)
    )
}

fn gemini_api_base(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');
    match base.find("/models/") {
        Some(index) => base[..index].to_string(),
        None => base.to_string(),
    }
}

pub(super) fn is_openai_compatible_url(base_url: &str) -> bool {
    let base = base_url.trim_end_matches('/').to_ascii_lowercase();
    base.ends_with("/chat/completions") || base.ends_with("/openai") || base.contains("/openai/")
}

fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.trim().as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(*byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_chat_url_supports_openai_compatible_base_paths() {
        assert_eq!(
            openai_chat_url("https://api.deepseek.com/v1"),
            "https://api.deepseek.com/v1/chat/completions"
        );
        assert_eq!(
            openai_chat_url("https://generativelanguage.googleapis.com/v1beta/openai"),
            "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
        );
    }

    #[test]
    fn gemini_url_targets_native_generate_content_endpoint() {
        assert_eq!(
            gemini_generate_content_url(
                "https://generativelanguage.googleapis.com/v1beta",
                "gemini-2.0-flash"
            ),
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent"
        );
    }
}
