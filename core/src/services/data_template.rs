use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const DEFAULT_KLINE_TIMEFRAME: &str = "5m";
pub const DEFAULT_KLINE_COUNT: usize = 100;
pub const MAX_KLINE_COUNT: usize = 1500;
const SUPPORTED_TIMEFRAMES: [&str; 7] = ["5m", "15m", "30m", "1h", "4h", "1d", "1w"];
const SUPPORTED_INDICATORS: [&str; 5] = ["ema", "macd", "rsi", "atr", "bollinger"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyDataTemplate {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub items: Vec<StrategyDataItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyDataItem {
    pub id: String,
    #[serde(rename = "type")]
    pub item_type: String,
    #[serde(default)]
    pub timeframes: Vec<String>,
    #[serde(default)]
    pub count: usize,
    #[serde(default = "default_closed_only")]
    pub closed_only: bool,
    #[serde(default)]
    pub indicator: String,
    #[serde(default = "default_params")]
    pub params: Value,
    #[serde(default = "default_output_mode")]
    pub output_mode: String,
}

pub fn default_data_template() -> StrategyDataTemplate {
    StrategyDataTemplate {
        schema_version: 1,
        items: vec![StrategyDataItem {
            id: "primary-kline".to_string(),
            item_type: "raw_kline".to_string(),
            timeframes: vec![DEFAULT_KLINE_TIMEFRAME.to_string()],
            count: DEFAULT_KLINE_COUNT,
            closed_only: true,
            indicator: String::new(),
            params: json!({}),
            output_mode: "latest".to_string(),
        }],
    }
}

pub fn validate_strategy_data_template(
    config: &Value,
) -> std::result::Result<StrategyDataTemplate, String> {
    let Some(raw_template) = config.get("data_template") else {
        return Ok(default_data_template());
    };

    let template = serde_json::from_value::<StrategyDataTemplate>(raw_template.clone())
        .map_err(|_| "data_template has an invalid shape".to_string())?;
    validate_data_template(&template)?;
    Ok(template)
}

pub fn data_template_value(template: &StrategyDataTemplate) -> Value {
    serde_json::to_value(template).unwrap_or_else(|_| json!({}))
}

fn validate_data_template(template: &StrategyDataTemplate) -> std::result::Result<(), String> {
    if template.schema_version != 1 {
        return Err("data_template schema_version must be 1".to_string());
    }

    if template.items.is_empty() {
        return Err("data_template requires at least one data item".to_string());
    }

    let mut ids = HashSet::new();
    let mut raw_items_count = 0usize;

    for item in &template.items {
        let id = item.id.trim();
        if id.is_empty() {
            return Err("data_template item id is required".to_string());
        }
        if !ids.insert(id) {
            return Err(format!("data_template item id '{id}' is duplicated"));
        }

        match item.item_type.as_str() {
            "raw_kline" => {
                validate_timeframes(&item.timeframes, id)?;
                if !(1..=MAX_KLINE_COUNT).contains(&item.count) {
                    return Err(format!(
                        "raw_kline '{id}' count must be between 1 and {MAX_KLINE_COUNT}"
                    ));
                }
                raw_items_count += 1;
                if raw_items_count > 1 {
                    return Err("data_template supports at most one raw_kline item".to_string());
                }
            }
            "indicator" => {}
            _ => return Err(format!("data_template item '{id}' has an unsupported type")),
        }
    }

    for item in &template.items {
        if item.item_type != "indicator" {
            continue;
        }

        let id = item.id.trim();
        validate_timeframes(&item.timeframes, id)?;
        if !SUPPORTED_INDICATORS.contains(&item.indicator.as_str()) {
            return Err(format!(
                "indicator '{id}' has an unsupported indicator type"
            ));
        }
        if item.output_mode != "latest" {
            return Err(format!("indicator '{id}' has an unsupported output_mode"));
        }

        if let Some(period) = item.params.get("period").and_then(Value::as_u64) {
            if period == 0 || period as usize > MAX_KLINE_COUNT {
                return Err(format!(
                    "indicator '{id}' period must be between 1 and {MAX_KLINE_COUNT}"
                ));
            }
        }
    }

    Ok(())
}

fn validate_timeframes(timeframes: &[String], item_id: &str) -> std::result::Result<(), String> {
    if timeframes.is_empty() {
        return Err(format!(
            "data_template item '{item_id}' requires at least one timeframe"
        ));
    }
    if timeframes
        .iter()
        .any(|timeframe| !SUPPORTED_TIMEFRAMES.contains(&timeframe.as_str()))
    {
        return Err(format!(
            "data_template item '{item_id}' has an unsupported timeframe"
        ));
    }
    if timeframes.iter().collect::<HashSet<_>>().len() != timeframes.len() {
        return Err(format!(
            "data_template item '{item_id}' has duplicate timeframes"
        ));
    }
    Ok(())
}

fn default_schema_version() -> u32 {
    1
}

fn default_closed_only() -> bool {
    true
}

fn default_params() -> Value {
    json!({})
}

fn default_output_mode() -> String {
    "latest".to_string()
}
