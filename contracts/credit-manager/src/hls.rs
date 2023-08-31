use cosmwasm_std::{Deps, Response};
use mars_params::types::hls::HlsAssetType;
use mars_rover::error::{ContractError, ContractResult};
use mars_rover_health_types::AccountKind;

use crate::{query::query_positions, state::PARAMS};

pub fn assert_hls_rules(deps: Deps, account_id: &str) -> ContractResult<Response> {
    // Rule #1 - There can only be 0 or 1 debt denom in the account
    let positions = query_positions(deps, account_id)?;

    if positions.debts.len() > 1 {
        return Err(ContractError::HLS {
            reason: "Account has more than one debt denom".to_string(),
        });
    }

    if let Some(debt) = positions.debts.first() {
        let params = PARAMS.load(deps.storage)?.query_asset_params(&deps.querier, &debt.denom)?;

        // Rule #2: Debt denom must have HLS params set in the Mars-Param contract
        let Some(hls) = params.credit_manager.hls else {
            return Err(ContractError::HLS {
                reason: format!("{} does not have HLS parameters", debt.denom),
            });
        };

        // Rule #3: For that debt denom, verify all collateral assets are only those
        //          within the correlated list for that debt denom

        // === Deposits ===
        for deposit in positions.deposits.iter() {
            hls.correlations
                .iter()
                .find(|h| match h {
                    HlsAssetType::Coin {
                        denom,
                    } => &deposit.denom == denom,
                    _ => false,
                })
                .ok_or_else(|| ContractError::HLS {
                    reason: format!(
                        "{} deposit is not a correlated asset to debt {}",
                        deposit.denom, debt.denom
                    ),
                })?;
        }

        // === Lends ===
        for lend in positions.lends.iter() {
            hls.correlations
                .iter()
                .find(|h| match h {
                    HlsAssetType::Coin {
                        denom,
                    } => &lend.denom == denom,
                    _ => false,
                })
                .ok_or_else(|| ContractError::HLS {
                    reason: format!(
                        "{} lend is not a correlated asset to debt {}",
                        lend.denom, debt.denom
                    ),
                })?;
        }

        // === Vault positions ===
        for v in positions.vaults.iter() {
            hls.correlations
                .iter()
                .find(|h| match h {
                    HlsAssetType::Vault {
                        addr,
                    } => v.vault.address == addr,
                    _ => false,
                })
                .ok_or_else(|| ContractError::HLS {
                    reason: format!(
                        "{} vault is not a correlated asset to debt {}",
                        v.vault.address, debt.denom
                    ),
                })?;
        }
    }

    Ok(Response::new()
        .add_attribute("action", "callback/assert_hls_rules")
        .add_attribute("account_id", account_id)
        .add_attribute("account_kind", AccountKind::HighLeveredStrategy.to_string()))
}
