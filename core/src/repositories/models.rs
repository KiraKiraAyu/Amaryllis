use std::collections::{HashMap, HashSet};

use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter, Set, TransactionTrait,
};

use crate::clients::llm_chat::normalize_provider_type;
use crate::entity::{llm_models, llm_providers};
use crate::time::ts_to_dt;

#[derive(Debug, Clone)]
pub struct ModelRepo {
    db: DatabaseConnection,
}

#[derive(Debug, Clone)]
pub struct ModelConfigRecord {
    pub id: String,
    pub provider_id: String,
    pub name: String,
    pub model_id: String,
}

#[derive(Debug, Clone)]
pub struct ProviderConfigRecord {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub enabled: i64,
    pub api_key: String,
    pub base_url: String,
    pub models: Vec<ModelConfigRecord>,
}

#[derive(Debug, Clone)]
pub struct UpsertProviderConfig {
    pub id: Option<String>,
    pub name: String,
    pub provider_type: String,
    pub enabled: bool,
    pub api_key: String,
    pub base_url: String,
    pub models: Vec<UpsertModelConfig>,
}

#[derive(Debug, Clone)]
pub struct UpsertModelConfig {
    pub id: Option<String>,
    pub name: String,
    pub model_id: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedModelRecord {
    pub id: String,
    pub model_id: String,
    pub provider_type: String,
    pub api_key: String,
    pub base_url: String,
}

#[derive(Debug, Clone, Copy)]
pub struct ProviderPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub provider_type: &'static str,
    pub base_url: &'static str,
    pub models: &'static [ModelPreset],
}

#[derive(Debug, Clone, Copy)]
pub struct ModelPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub model_id: &'static str,
}

const DEEPSEEK_MODELS: &[ModelPreset] = &[
    ModelPreset {
        id: "deepseek-v4-pro",
        name: "DeepSeek V4 Pro",
        model_id: "deepseek-v4-pro",
    },
    ModelPreset {
        id: "deepseek-v4-flash",
        name: "DeepSeek V4 Flash",
        model_id: "deepseek-v4-flash",
    },
];

const OPENAI_MODELS: &[ModelPreset] = &[
    ModelPreset {
        id: "openai-gpt-5-6-sol",
        name: "GPT-5.6 Sol",
        model_id: "gpt-5.6-sol",
    },
    ModelPreset {
        id: "openai-gpt-5-6-terra",
        name: "GPT-5.6 Terra",
        model_id: "gpt-5.6-terra",
    },
];

const CLAUDE_MODELS: &[ModelPreset] = &[
    ModelPreset {
        id: "claude-sonnet-5",
        name: "Claude Sonnet 5",
        model_id: "claude-sonnet-5",
    },
    ModelPreset {
        id: "claude-opus-5",
        name: "Claude Opus 5",
        model_id: "claude-opus-5",
    },
];

const QWEN_MODELS: &[ModelPreset] = &[
    ModelPreset {
        id: "qwen3-7-max",
        name: "Qwen3.7 Max",
        model_id: "qwen3.7-max",
    },
    ModelPreset {
        id: "qwen3-7-plus",
        name: "Qwen3.7 Plus",
        model_id: "qwen3.7-plus",
    },
];

const GEMINI_MODELS: &[ModelPreset] = &[
    ModelPreset {
        id: "gemini-3-5-flash",
        name: "Gemini 3.5 Flash",
        model_id: "gemini-3.5-flash",
    },
    ModelPreset {
        id: "gemini-3-1-flash-lite",
        name: "Gemini 3.1 Flash Lite",
        model_id: "gemini-3.1-flash-lite",
    },
];

const GROK_MODELS: &[ModelPreset] = &[
    ModelPreset {
        id: "grok-4-5",
        name: "Grok 4.5",
        model_id: "grok-4.5",
    },
    ModelPreset {
        id: "grok-4-3",
        name: "Grok 4.3",
        model_id: "grok-4.3",
    },
];

const KIMI_MODELS: &[ModelPreset] = &[
    ModelPreset {
        id: "kimi-k3",
        name: "Kimi K3",
        model_id: "kimi-k3",
    },
    ModelPreset {
        id: "kimi-k2-6",
        name: "Kimi K2.6",
        model_id: "kimi-k2.6",
    },
];

const PROVIDER_PRESETS: &[ProviderPreset] = &[
    ProviderPreset {
        id: "deepseek",
        name: "DeepSeek",
        provider_type: "openai",
        base_url: "https://api.deepseek.com/v1",
        models: DEEPSEEK_MODELS,
    },
    ProviderPreset {
        id: "openai",
        name: "OpenAI",
        provider_type: "openai",
        base_url: "https://api.openai.com/v1",
        models: OPENAI_MODELS,
    },
    ProviderPreset {
        id: "claude",
        name: "Claude",
        provider_type: "anthropic",
        base_url: "https://api.anthropic.com",
        models: CLAUDE_MODELS,
    },
    ProviderPreset {
        id: "qwen",
        name: "Qwen",
        provider_type: "openai",
        base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        models: QWEN_MODELS,
    },
    ProviderPreset {
        id: "gemini",
        name: "Google Gemini",
        provider_type: "gemini",
        base_url: "https://generativelanguage.googleapis.com/v1beta",
        models: GEMINI_MODELS,
    },
    ProviderPreset {
        id: "grok",
        name: "Grok (xAI)",
        provider_type: "openai",
        base_url: "https://api.x.ai/v1",
        models: GROK_MODELS,
    },
    ProviderPreset {
        id: "kimi",
        name: "Kimi (Moonshot)",
        provider_type: "openai",
        base_url: "https://api.moonshot.cn/v1",
        models: KIMI_MODELS,
    },
];

impl ModelRepo {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn list_for_user(
        &self,
    ) -> Result<Vec<ProviderConfigRecord>, crate::database::DbErr> {
        self.ensure_defaults().await?;

        let providers = llm_providers::Entity::find()
            .all(&self.db)
            .await?;

        let models = llm_models::Entity::find()
            .all(&self.db)
            .await?;

        let mut models_by_provider: HashMap<String, Vec<ModelConfigRecord>> = HashMap::new();
        for row in models {
            models_by_provider
                .entry(row.provider_id.clone())
                .or_default()
                .push(ModelConfigRecord {
                    id: row.id,
                    provider_id: row.provider_id,
                    name: row.name,
                    model_id: row.model_id,
                });
        }

        let mut items: Vec<ProviderConfigRecord> = providers
            .into_iter()
            .map(|row| {
                let mut models = models_by_provider.remove(&row.id).unwrap_or_default();
                models.sort_by(model_sort_key);

                ProviderConfigRecord {
                    id: row.id,
                    name: row.name,
                    provider_type: normalized_provider_type_or_original(&row.provider_type),
                    enabled: i64::from(row.enabled),
                    api_key: row.api_key,
                    base_url: row.base_url,
                    models,
                }
            })
            .collect();

        items.sort_by(provider_sort_key);
        Ok(items)
    }

    pub async fn replace_for_user(
        &self,
        providers: Vec<UpsertProviderConfig>,
    ) -> Result<(), crate::database::DbErr> {
        let existing_providers = llm_providers::Entity::find()
            .all(&self.db)
            .await?;
        let existing_models = llm_models::Entity::find()
            .all(&self.db)
            .await?;

        let existing_provider_api_keys: HashMap<String, String> = existing_providers
            .iter()
            .map(|row| (row.id.clone(), row.api_key.clone()))
            .collect();

        let mut used_provider_ids: HashSet<String> = existing_providers
            .iter()
            .map(|row| row.id.clone())
            .collect();
        let mut used_model_ids: HashSet<String> =
            existing_models.iter().map(|row| row.id.clone()).collect();

        let mut keep_provider_ids = HashSet::new();
        let mut keep_model_ids = HashSet::new();
        let now = ts_to_dt(now_ts());

        let tx = self.db.begin().await?;

        for provider in providers {
            if let Some(existing_id) = provider.id.as_ref() {
                used_provider_ids.remove(existing_id);
            }
            let base_provider_id = provider.id.unwrap_or_else(|| {
                let base = slugify(&provider.name);
                if base.is_empty() {
                    sanitize_id(&provider.provider_type)
                } else {
                    base
                }
            });
            let provider_id = reserve_unique_id(base_provider_id, &mut used_provider_ids);
            keep_provider_ids.insert(provider_id.clone());

            let api_key = match existing_provider_api_keys.get(&provider_id) {
                Some(existing_key) if provider.api_key.trim().is_empty() => existing_key.clone(),
                _ => provider.api_key.trim().to_string(),
            };

            llm_providers::Entity::insert(llm_providers::ActiveModel {
                id: Set(provider_id.clone()),
                name: Set(provider.name.trim().to_string()),
                provider_type: Set(normalized_provider_type_or_original(
                    &provider.provider_type,
                )),
                enabled: Set(if provider.enabled { 1 } else { 0 }),
                api_key: Set(api_key),
                base_url: Set(provider.base_url.trim().trim_end_matches('/').to_string()),
                created_at: Set(now),
                updated_at: Set(now),
            })
            .exec(&tx)
            .await?;

            for model in provider.models {
                if let Some(existing_id) = model.id.as_ref() {
                    used_model_ids.remove(existing_id);
                }
                let base_model_id = model.id.unwrap_or_else(|| {
                    let raw = format!("{}-{}", provider_id, sanitize_id(&model.model_id));
                    if raw.trim_matches('-').is_empty() {
                        format!("{}-model", provider_id)
                    } else {
                        raw
                    }
                });
                let model_id = reserve_unique_id(base_model_id, &mut used_model_ids);
                keep_model_ids.insert(model_id.clone());

                llm_models::Entity::insert(llm_models::ActiveModel {
                    id: Set(model_id),
                    provider_id: Set(provider_id.clone()),
                    name: Set(model.name.trim().to_string()),
                    model_id: Set(model.model_id.trim().to_string()),
                    created_at: Set(now),
                    updated_at: Set(now),
                })
                .exec(&tx)
                .await?;
            }
        }

        delete_stale_models(&tx, &keep_model_ids).await?;
        delete_stale_providers(&tx, &keep_provider_ids).await?;
        tx.commit().await?;

        Ok(())
    }

    pub async fn resolve_for_user(
        &self,
        model_id: Option<&str>,
    ) -> Result<Option<ResolvedModelRecord>, crate::database::DbErr> {
        let providers = self.list_for_user().await?;

        if let Some(requested_id) = model_id.map(str::trim).filter(|id| !id.is_empty()) {
            return Ok(find_model(&providers, requested_id));
        }

        let preferred = providers
            .iter()
            .filter(|provider| provider.enabled != 0)
            .find_map(|provider| {
                provider
                    .models
                    .first()
                    .map(|model| resolved_model(provider, model))
            });

        Ok(preferred.or_else(|| {
            providers.iter().find_map(|provider| {
                provider
                    .models
                    .first()
                    .map(|model| resolved_model(provider, model))
            })
        }))
    }

    pub async fn list_runnable_for_user(
        &self,
    ) -> Result<Vec<ResolvedModelRecord>, crate::database::DbErr> {
        let providers = self.list_for_user().await?;
        let mut items = Vec::new();

        for provider in providers {
            if provider.enabled == 0 || provider.api_key.trim().is_empty() {
                continue;
            }

            for model in &provider.models {
                items.push(resolved_model(&provider, model));
            }
        }

        Ok(items)
    }

    pub async fn ensure_defaults(&self) -> Result<(), crate::database::DbErr> {
        let existing_count = llm_providers::Entity::find()
            .count(&self.db)
            .await?;

        if existing_count > 0 {
            return Ok(());
        }

        let tx = self.db.begin().await?;
        let now = ts_to_dt(now_ts());

        for provider in PROVIDER_PRESETS {
            llm_providers::ActiveModel {
                id: Set(provider.id.to_string()),
                name: Set(provider.name.to_string()),
                provider_type: Set(provider.provider_type.to_string()),
                enabled: Set(1),
                api_key: Set(String::new()),
                base_url: Set(provider.base_url.to_string()),
                created_at: Set(now),
                updated_at: Set(now),
            }
            .insert(&tx)
            .await?;

            for model in provider.models {
                llm_models::ActiveModel {
                    id: Set(model.id.to_string()),
                    provider_id: Set(provider.id.to_string()),
                    name: Set(model.name.to_string()),
                    model_id: Set(model.model_id.to_string()),
                    created_at: Set(now),
                    updated_at: Set(now),
                }
                .insert(&tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(())
    }
}

async fn delete_stale_models<C>(
    db: &C,
    keep_ids: &HashSet<String>,
) -> Result<(), crate::database::DbErr>
where
    C: sea_orm::ConnectionTrait,
{
    let mut condition = Condition::all();
    if !keep_ids.is_empty() {
        condition = condition.add(llm_models::Column::Id.is_not_in(keep_ids.iter().cloned()));
    }

    llm_models::Entity::delete_many()
        .filter(condition)
        .exec(db)
        .await?;

    Ok(())
}

async fn delete_stale_providers<C>(
    db: &C,
    keep_ids: &HashSet<String>,
) -> Result<(), crate::database::DbErr>
where
    C: sea_orm::ConnectionTrait,
{
    let mut condition = Condition::all();
    if !keep_ids.is_empty() {
        condition = condition.add(llm_providers::Column::Id.is_not_in(keep_ids.iter().cloned()));
    }

    llm_providers::Entity::delete_many()
        .filter(condition)
        .exec(db)
        .await?;

    Ok(())
}

fn find_model(
    providers: &[ProviderConfigRecord],
    requested_id: &str,
) -> Option<ResolvedModelRecord> {
    providers.iter().find_map(|provider| {
        provider
            .models
            .iter()
            .find(|model| model.id == requested_id)
            .map(|model| resolved_model(provider, model))
    })
}

fn resolved_model(
    provider: &ProviderConfigRecord,
    model: &ModelConfigRecord,
) -> ResolvedModelRecord {
    ResolvedModelRecord {
        id: model.id.clone(),
        model_id: model.model_id.clone(),
        provider_type: normalized_provider_type_or_original(&provider.provider_type),
        api_key: provider.api_key.clone(),
        base_url: provider.base_url.clone(),
    }
}

fn normalized_provider_type_or_original(provider_type: &str) -> String {
    let normalized = normalize_provider_type(provider_type);
    if normalized.is_empty() {
        provider_type.trim().to_ascii_lowercase()
    } else {
        normalized.to_string()
    }
}

fn provider_sort_key(
    left: &ProviderConfigRecord,
    right: &ProviderConfigRecord,
) -> std::cmp::Ordering {
    preset_position(&left.id, true)
        .cmp(&preset_position(&right.id, true))
        .then_with(|| left.name.cmp(&right.name))
        .then_with(|| left.id.cmp(&right.id))
}

fn model_sort_key(left: &ModelConfigRecord, right: &ModelConfigRecord) -> std::cmp::Ordering {
    preset_position(&left.id, false)
        .cmp(&preset_position(&right.id, false))
        .then_with(|| left.name.cmp(&right.name))
        .then_with(|| left.id.cmp(&right.id))
}

fn preset_position(id: &str, provider: bool) -> usize {
    if provider {
        PROVIDER_PRESETS
            .iter()
            .position(|preset| preset.id == id)
            .unwrap_or(usize::MAX)
    } else {
        PROVIDER_PRESETS
            .iter()
            .flat_map(|preset| preset.models.iter())
            .position(|preset| preset.id == id)
            .unwrap_or(usize::MAX)
    }
}

fn reserve_unique_id(base: String, used: &mut HashSet<String>) -> String {
    let normalized = if base.trim().is_empty() {
        "item".to_string()
    } else {
        sanitize_id(&base)
    };

    if used.insert(normalized.clone()) {
        return normalized;
    }

    let mut idx = 2;
    loop {
        let candidate = format!("{normalized}-{idx}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
        idx += 1;
    }
}

fn slugify(input: &str) -> String {
    sanitize_id(input)
}

fn sanitize_id(input: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;

    for ch in input.trim().chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            out.push(lower);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }

    out.trim_matches('-').to_string()
}

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::init_database;

    #[test]
    fn provider_presets_store_api_category_as_provider_type() {
        let expected = [
            ("deepseek", "openai"),
            ("openai", "openai"),
            ("claude", "anthropic"),
            ("qwen", "openai"),
            ("gemini", "gemini"),
            ("grok", "openai"),
            ("kimi", "openai"),
        ];

        for (id, provider_type) in expected {
            let preset = PROVIDER_PRESETS
                .iter()
                .find(|preset| preset.id == id)
                .expect("preset should exist");

            assert_eq!(preset.provider_type, provider_type, "{id} provider_type");
        }
    }

    #[tokio::test]
    async fn list_for_user_returns_normalized_default_provider_types() {
        let db = init_database("sqlite::memory:")
            .await
            .expect("connect sqlite memory");
        let repo = ModelRepo::new(db);

        let providers = repo
            .list_for_user()
            .await
            .expect("list defaults");

        let provider_types: HashMap<_, _> = providers
            .iter()
            .map(|provider| (provider.id.as_str(), provider.provider_type.as_str()))
            .collect();

        assert_eq!(provider_types.get("deepseek"), Some(&"openai"));
        assert_eq!(provider_types.get("qwen"), Some(&"openai"));
        assert_eq!(provider_types.get("grok"), Some(&"openai"));
        assert_eq!(provider_types.get("kimi"), Some(&"openai"));
        assert_eq!(provider_types.get("claude"), Some(&"anthropic"));
        assert_eq!(provider_types.get("gemini"), Some(&"gemini"));
    }

    #[tokio::test]
    async fn replace_for_user_normalizes_legacy_vendor_provider_types() {
        let db = init_database("sqlite::memory:")
            .await
            .expect("connect sqlite memory");
        let repo = ModelRepo::new(db);

        repo.replace_for_user(vec![
            UpsertProviderConfig {
                id: Some("legacy-deepseek".to_string()),
                name: "Legacy DeepSeek".to_string(),
                provider_type: "deepseek".to_string(),
                enabled: true,
                api_key: "secret".to_string(),
                base_url: "https://api.deepseek.com/v1".to_string(),
                models: vec![UpsertModelConfig {
                    id: Some("legacy-deepseek-chat".to_string()),
                    name: "DeepSeek Chat".to_string(),
                    model_id: "deepseek-chat".to_string(),
                }],
            },
            UpsertProviderConfig {
                id: Some("legacy-claude".to_string()),
                name: "Legacy Claude".to_string(),
                provider_type: "claude".to_string(),
                enabled: true,
                api_key: "secret".to_string(),
                base_url: "https://api.anthropic.com".to_string(),
                models: vec![UpsertModelConfig {
                    id: Some("legacy-claude-sonnet".to_string()),
                    name: "Claude Sonnet".to_string(),
                    model_id: "claude-3-5-sonnet-20241022".to_string(),
                }],
            },
        ])
        .await
        .expect("replace providers");

        let providers = repo
            .list_for_user()
            .await
            .expect("list providers");

        let provider_types: HashMap<_, _> = providers
            .iter()
            .map(|provider| (provider.id.as_str(), provider.provider_type.as_str()))
            .collect();

        assert_eq!(provider_types.get("legacy-deepseek"), Some(&"openai"));
        assert_eq!(provider_types.get("legacy-claude"), Some(&"anthropic"));
    }
}
