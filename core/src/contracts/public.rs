use serde::{Deserialize, Serialize};

use crate::{
    clients::llm_chat::SupportedProviderType,
    clients::market_data::{MarketKline, MarketSymbol},
};

#[derive(Debug, Deserialize)]
pub struct EquityHistoryQuery {
    pub trader_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SymbolsQuery {
    pub exchange: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct KlinesQuery {
    pub symbol: String,
    pub interval: Option<String>,
    pub limit: Option<i64>,
    pub exchange: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CryptoConfigPayload {
    pub transport_encryption: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CryptoPublicKeyPayload {
    pub transport_encryption: bool,
    pub public_key: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EquityHistoryPointPayload {
    pub timestamp: String,
    pub total_equity: f64,
    pub available_balance: f64,
    pub total_pnl: f64,
    pub total_pnl_pct: f64,
    pub position_count: i64,
    pub margin_used_pct: f64,
    pub balance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeSymbolPayload {
    pub symbol: String,
    pub name: String,
    pub category: String,
}

impl From<MarketSymbol> for ExchangeSymbolPayload {
    fn from(value: MarketSymbol) -> Self {
        Self {
            symbol: value.symbol,
            name: value.name,
            category: value.category,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KlinePayload {
    #[serde(rename = "openTime")]
    pub open_time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    #[serde(rename = "quoteVolume")]
    pub quote_volume: f64,
    #[serde(rename = "closeTime")]
    pub close_time: i64,
}

impl From<MarketKline> for KlinePayload {
    fn from(value: MarketKline) -> Self {
        Self {
            open_time: value.open_time,
            open: value.open,
            high: value.high,
            low: value.low,
            close: value.close,
            volume: value.volume,
            quote_volume: value.quote_volume,
            close_time: value.close_time,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExchangeSymbolsPayload {
    pub exchange: String,
    pub symbols: Vec<ExchangeSymbolPayload>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SupportedProviderTypePayload {
    #[serde(rename = "providerType")]
    pub provider_type: String,
    pub name: String,
}

impl From<&SupportedProviderType> for SupportedProviderTypePayload {
    fn from(value: &SupportedProviderType) -> Self {
        Self {
            provider_type: value.provider_type.to_string(),
            name: value.name.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SupportedExchangePayload {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub exchange_kind: String,
}
