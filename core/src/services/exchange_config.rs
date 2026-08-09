use std::sync::Arc;

use crate::{
    contracts::exchanges::{
        CreateExchangePayload, ExchangeConfigPatch, MessagePayload, SafeExchangeConfig,
    },
    error::{AppError, Result},
    repositories::{
        ExchangeRepo,
        exchanges::{CreateExchangeAccount, UpdateExchangeAccount},
    },
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ExchangeConfigService {
    repo: Arc<ExchangeRepo>,
}

impl ExchangeConfigService {
    pub fn new(repo: Arc<ExchangeRepo>) -> Self {
        Self { repo }
    }

    pub async fn list_configs(&self) -> Result<Vec<SafeExchangeConfig>> {
        let rows =
            self.repo.list_for_user().await.map_err(|err| {
                AppError::Internal(format!("Failed to get exchange configs: {err}"))
            })?;

        Ok(rows
            .into_iter()
            .map(|row| SafeExchangeConfig {
                id: row.id,
                exchange_type: row.exchange_type,
                account_name: row.account_name,
                name: row.name,
                exchange_kind: row.exchange_kind,
                enabled: row.enabled != 0,
                testnet: row.testnet != 0,
                hyperliquid_wallet_addr: row.hyperliquid_wallet_addr,
            })
            .collect())
    }

    pub async fn create_exchange(
        &self,
        exchange_type: String,
        account_name: String,
        enabled: bool,
        api_key: String,
        secret_key: String,
        passphrase: String,
        testnet: bool,
        hyperliquid_wallet_addr: String,
    ) -> Result<CreateExchangePayload> {
        let exchange_type = exchange_type.trim().to_ascii_lowercase();
        if !is_supported_exchange_type(&exchange_type) {
            return Err(AppError::BadRequest("Invalid exchange type".into()));
        }

        let account_name = if account_name.trim().is_empty() {
            "Default".to_string()
        } else {
            account_name.trim().to_string()
        };

        let (name, exchange_kind) = exchange_name_and_type(&exchange_type);
        validate_exchange_credentials(&exchange_type, &secret_key, &hyperliquid_wallet_addr)?;
        let id = Uuid::now_v7().to_string();
        let now = now_ts();

        self.repo
            .create(CreateExchangeAccount {
                id: id.clone(),
                exchange_type,
                account_name,
                name: name.to_string(),
                exchange_kind: exchange_kind.to_string(),
                enabled,
                api_key: api_key.trim().to_string(),
                secret_key: secret_key.trim().to_string(),
                passphrase: passphrase.trim().to_string(),
                testnet,
                hyperliquid_wallet_addr: hyperliquid_wallet_addr.trim().to_string(),
                created_at: now,
                updated_at: now,
            })
            .await
            .map_err(|err| {
                AppError::Internal(format!("Failed to create exchange account: {err}"))
            })?;

        Ok(CreateExchangePayload {
            message: "Exchange account created",
            id,
        })
    }

    pub async fn update_configs(
        &self,
        exchanges: std::collections::HashMap<String, ExchangeConfigPatch>,
    ) -> Result<MessagePayload> {
        let now = now_ts();

        for (exchange_id, patch) in exchanges {
            let existing = self
                .repo
                .find_runtime_config(&exchange_id)
                .await
                .map_err(|err| {
                    AppError::Internal(format!("Failed to load exchange config: {err}"))
                })?;

            let Some(existing) = existing else {
                continue;
            };

            let effective_secret_key = keep_or_new(existing.secret_key, &patch.secret_key);
            let effective_wallet_addr = if patch.hyperliquid_wallet_addr.trim().is_empty() {
                existing.hyperliquid_wallet_addr
            } else {
                patch.hyperliquid_wallet_addr.trim().to_string()
            };

            validate_updated_credentials(
                &existing.exchange_type,
                &effective_wallet_addr,
                &effective_secret_key,
            )?;

            self.repo
                .update(
                    &exchange_id,
                    UpdateExchangeAccount {
                        enabled: patch.enabled,
                        api_key: keep_or_new(existing.api_key, &patch.api_key),
                        secret_key: effective_secret_key,
                        passphrase: keep_or_new(existing.passphrase, &patch.passphrase),
                        testnet: patch.testnet,
                        hyperliquid_wallet_addr: effective_wallet_addr,
                        updated_at: now,
                    },
                )
                .await
                .map_err(|err| {
                    AppError::Internal(format!("Failed to update exchange config: {err}"))
                })?;
        }

        Ok(MessagePayload {
            message: "Exchange configuration updated",
        })
    }

    pub async fn delete_exchange(&self, exchange_id: &str) -> Result<MessagePayload> {
        if exchange_id.trim().is_empty() {
            return Err(AppError::BadRequest("Exchange ID is required".into()));
        }

        if self
            .repo
            .find_trader_usage(exchange_id)
            .await
            .map_err(|err| AppError::Internal(format!("Failed to check trader usage: {err}")))?
            .is_some()
        {
            return Err(AppError::Conflict(
                "Cannot delete exchange account that is in use by traders".into(),
            ));
        }

        let deleted = self.repo.delete(exchange_id).await.map_err(|err| {
            AppError::Internal(format!("Failed to delete exchange account: {err}"))
        })?;

        if deleted == 0 {
            return Err(AppError::NotFound("Exchange not found".into()));
        }

        Ok(MessagePayload {
            message: "Exchange account deleted",
        })
    }
}

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn keep_or_new(existing: String, incoming: &str) -> String {
    let trimmed = incoming.trim();
    if trimmed.is_empty() {
        existing
    } else {
        trimmed.to_string()
    }
}

fn is_supported_exchange_type(exchange_type: &str) -> bool {
    matches!(
        exchange_type,
        "binance" | "okx" | "bitget" | "hyperliquid" | "aster"
    )
}

fn exchange_name_and_type(exchange_type: &str) -> (&'static str, &'static str) {
    match exchange_type {
        "binance" => ("Binance Futures", "cex"),
        "okx" => ("OKX Futures", "cex"),
        "bitget" => ("Bitget Futures", "cex"),
        "hyperliquid" => ("Hyperliquid", "dex"),
        "aster" => ("Aster DEX", "dex"),
        _ => ("Unknown Exchange", "cex"),
    }
}

fn validate_exchange_credentials(
    exchange_type: &str,
    secret_key: &str,
    hyperliquid_wallet_addr: &str,
) -> Result<()> {
    match exchange_type {
        "aster" => {
            let wallet_addr = hyperliquid_wallet_addr.trim();
            if wallet_addr.is_empty() {
                return Err(AppError::BadRequest(
                    "Main wallet address is required for Aster".into(),
                ));
            }
            let addr_hex = wallet_addr.trim_start_matches("0x");
            if addr_hex.len() != 40 || !addr_hex.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(AppError::BadRequest(format!(
                    "Main wallet address must be 40 hex chars (20 bytes), got {} chars. \
                     This is your MetaMask login wallet address, NOT the API wallet address.",
                    addr_hex.len()
                )));
            }

            let private_key = secret_key.trim();
            if private_key.is_empty() {
                return Err(AppError::BadRequest(
                    "API wallet private key is required for Aster".into(),
                ));
            }
            let pk_hex = private_key.trim_start_matches("0x");
            if pk_hex.len() != 64 || !pk_hex.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(AppError::BadRequest(format!(
                    "API wallet private key must be 64 hex chars (32 bytes), got {} chars. \
                     Make sure you copied the private key, not the API wallet address \
                     (which is only 40 hex chars). You can find it at \
                     https://www.asterdex.com/en/api-wallet under 'Pro API'.",
                    pk_hex.len()
                )));
            }
        }
        "hyperliquid" => {
            let private_key = secret_key.trim();
            if private_key.is_empty() {
                return Err(AppError::BadRequest(
                    "Private key is required for Hyperliquid".into(),
                ));
            }
            let pk_hex = private_key.trim_start_matches("0x");
            if pk_hex.len() != 64 || !pk_hex.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(AppError::BadRequest(format!(
                    "Private key must be 64 hex chars (32 bytes), got {} chars",
                    pk_hex.len()
                )));
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_updated_credentials(
    exchange_type: &str,
    wallet_addr: &str,
    secret_key: &str,
) -> Result<()> {
    match exchange_type {
        "aster" => {
            let addr_hex = wallet_addr.trim().trim_start_matches("0x");
            if addr_hex.len() != 40 || !addr_hex.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(AppError::BadRequest(format!(
                    "Main wallet address must be 40 hex chars (20 bytes), got {} chars. \
                     This is your MetaMask login wallet address, NOT the API wallet address.",
                    addr_hex.len()
                )));
            }

            let pk_hex = secret_key.trim().trim_start_matches("0x");
            if pk_hex.len() != 64 || !pk_hex.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(AppError::BadRequest(format!(
                    "API wallet private key must be 64 hex chars (32 bytes), got {} chars. \
                     Make sure you copied the private key, not the API wallet address \
                     (which is only 40 hex chars). You can find it at \
                     https://www.asterdex.com/en/api-wallet under 'Pro API'.",
                    pk_hex.len()
                )));
            }
        }
        "hyperliquid" => {
            let pk_hex = secret_key.trim().trim_start_matches("0x");
            if pk_hex.len() != 64 || !pk_hex.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(AppError::BadRequest(format!(
                    "Private key must be 64 hex chars (32 bytes), got {} chars",
                    pk_hex.len()
                )));
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exchange_type_support_is_limited_to_current_targets() {
        for exchange_type in ["binance", "okx", "bitget", "hyperliquid", "aster"] {
            assert!(
                is_supported_exchange_type(exchange_type),
                "{exchange_type} should be supported"
            );
        }

        for exchange_type in ["bybit", "kucoin", "gate", "lighter"] {
            assert!(
                !is_supported_exchange_type(exchange_type),
                "{exchange_type} should not be supported"
            );
        }
    }

    #[test]
    fn aster_validation_rejects_address_length_private_key() {
        let err = validate_exchange_credentials(
            "aster",
            "0x21cf8ae13bb72632562c6ff438652ba1a151bb0", // 40 hex chars - address, not private key
            "0x63DD5aCC6b1aa0f563956C0e534DD30B6dcF7C4e",
        )
        .unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
        assert!(err.to_string().contains("64 hex chars"));
    }

    #[test]
    fn aster_validation_accepts_correct_private_key() {
        assert!(
            validate_exchange_credentials(
                "aster",
                "4fd0a42218f3eae43a6ce26d22544e986139a01e5b34a62db53757ffca81bae1",
                "0x63DD5aCC6b1aa0f563956C0e534DD30B6dcF7C4e",
            )
            .is_ok()
        );
    }

    #[test]
    fn update_validation_rejects_address_as_private_key() {
        let err = validate_updated_credentials(
            "aster",
            "0x63DD5aCC6b1aa0f563956C0e534DD30B6dcF7C4e",
            "21cf8ae13bb72632562c6ff438652ba1a151bb0", // 40 chars - address, not private key
        )
        .unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
        assert!(err.to_string().contains("64 hex chars"));
    }

    #[test]
    fn update_validation_accepts_correct_credentials() {
        assert!(
            validate_updated_credentials(
                "aster",
                "0x63DD5aCC6b1aa0f563956C0e534DD30B6dcF7C4e",
                "4fd0a42218f3eae43a6ce26d22544e986139a01e5b34a62db53757ffca81bae1",
            )
            .is_ok()
        );

        assert!(
            validate_updated_credentials(
                "hyperliquid",
                "",
                "4fd0a42218f3eae43a6ce26d22544e986139a01e5b34a62db53757ffca81bae1",
            )
            .is_ok()
        );
    }

    #[test]
    fn update_validation_skips_non_wallet_exchanges() {
        assert!(validate_updated_credentials("binance", "", "any-secret").is_ok());
    }
}
