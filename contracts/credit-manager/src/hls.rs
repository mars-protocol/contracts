use cosmwasm_std::{Deps, Response};
use mars_types::{credit_manager::Positions, health::AccountKind, params::HlsAssetType};

use crate::{
    error::{ContractError, ContractResult},
    query::query_positions,
    state::PARAMS,
};

pub fn assert_hls_rules(deps: Deps, account_id: &str) -> ContractResult<Response> {
    // Rule #1 - There can only be 0 or 1 debt denom in the account
    let Positions {
        // destruct Positions so whenever we add new positions we don't forget to add them here
        account_id,
        account_kind: _,
        deposits: _,
        debts,
        lends,
        vaults,
        staked_astro_lps,
    } = query_positions(deps, account_id)?;

    if debts.len() > 1 {
        return Err(ContractError::HLS {
            reason: "Account has more than one debt denom".to_string(),
        });
    }

    if let Some(debt) = debts.first() {
        let params = PARAMS
            .load(deps.storage)?
            .query_asset_params(&deps.querier, &debt.denom)?
            .ok_or(ContractError::AssetParamsNotFound(debt.denom.to_string()))?;

        // Rule #2: Debt denom must have HLS params set in the Mars-Param contract
        let Some(hls) = params.credit_manager.hls else {
            return Err(ContractError::HLS {
                reason: format!("{} does not have HLS parameters", debt.denom),
            });
        };

        // Rule #3: For that debt denom, verify all collateral assets excluding deposits are only those
        //          within the correlated list for that debt denom.
        //          Deposits can have claimed rewards which are not correlated. These assets will have
        //          LTV = 0 and won't be considered for HF.

        // === Lends ===
        for lend in lends.iter() {
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
        for v in vaults.iter() {
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

        // === Staked Astro LP positions ===
        for staked_astro_lp in staked_astro_lps.iter() {
            hls.correlations
                .iter()
                .find(|h| match h {
                    HlsAssetType::Coin {
                        denom,
                    } => &staked_astro_lp.denom == denom,
                    _ => false,
                })
                .ok_or_else(|| ContractError::HLS {
                    reason: format!(
                        "{} staked astro lp is not a correlated asset to debt {}",
                        staked_astro_lp.denom, debt.denom
                    ),
                })?;
        }
    }

    Ok(Response::new()
        .add_attribute("action", "callback/assert_hls_rules")
        .add_attribute("account_id", account_id)
        .add_attribute("account_kind", AccountKind::HighLeveredStrategy.to_string()))
}
