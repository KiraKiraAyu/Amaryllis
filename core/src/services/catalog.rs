use crate::contracts::public::{SupportedExchangePayload, SupportedProviderTypePayload};

pub fn supported_provider_types() -> Vec<SupportedProviderTypePayload> {
    crate::clients::llm_chat::supported_provider_types()
        .iter()
        .map(SupportedProviderTypePayload::from)
        .collect()
}

pub fn supported_exchanges() -> Vec<SupportedExchangePayload> {
    vec![
        SupportedExchangePayload {
            id: "binance".to_string(),
            name: "Binance".to_string(),
            exchange_kind: "cex".to_string(),
        },
        SupportedExchangePayload {
            id: "okx".to_string(),
            name: "OKX".to_string(),
            exchange_kind: "cex".to_string(),
        },
        SupportedExchangePayload {
            id: "bitget".to_string(),
            name: "Bitget".to_string(),
            exchange_kind: "cex".to_string(),
        },
        SupportedExchangePayload {
            id: "hyperliquid".to_string(),
            name: "Hyperliquid".to_string(),
            exchange_kind: "dex".to_string(),
        },
        SupportedExchangePayload {
            id: "aster".to_string(),
            name: "Aster DEX".to_string(),
            exchange_kind: "dex".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_exchanges_only_include_current_targets() {
        let ids = supported_exchanges()
            .into_iter()
            .map(|exchange| exchange.id)
            .collect::<Vec<_>>();

        assert_eq!(
            ids,
            vec!["binance", "okx", "bitget", "hyperliquid", "aster"]
        );
    }

    #[test]
    fn supported_provider_types_includes_all_known_providers() {
        let types = supported_provider_types()
            .into_iter()
            .map(|p| p.provider_type)
            .collect::<Vec<_>>();

        assert!(types.contains(&"chat_completions".to_string()));
        assert!(types.contains(&"anthropic_messages".to_string()));
        assert!(types.contains(&"gemini_generate_content".to_string()));
        assert!(types.contains(&"responses".to_string()));
    }
}
