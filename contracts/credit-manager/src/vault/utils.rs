use cosmwasm_std::{Addr, Coin, Deps, StdResult, Storage};

use rover::adapters::vault::{Total, Vault, VaultPositionAmount, VaultPositionUpdate};
use rover::error::{ContractError, ContractResult};

use crate::state::{VAULT_CONFIGS, VAULT_POSITIONS};
use crate::update_coin_balances::query_balances;

pub fn assert_vault_is_whitelisted(storage: &mut dyn Storage, vault: &Vault) -> ContractResult<()> {
    let config = VAULT_CONFIGS
        .may_load(storage, &vault.address)?
        .and_then(|config| config.whitelisted.then_some(true));
    if config.is_none() {
        return Err(ContractError::NotWhitelisted(vault.address.to_string()));
    }
    Ok(())
}

pub fn update_vault_position(
    storage: &mut dyn Storage,
    account_id: &str,
    vault_addr: &Addr,
    update: VaultPositionUpdate,
) -> ContractResult<VaultPositionAmount> {
    let path = VAULT_POSITIONS.key((account_id, vault_addr.clone()));
    let mut amount = path
        .may_load(storage)?
        .unwrap_or_else(|| update.default_amount());

    amount.update(update)?;

    if amount.total().is_zero() {
        path.remove(storage);
    } else {
        path.save(storage, &amount)?;
    }
    Ok(amount)
}

/// Returns the total vault token balance for rover
pub fn query_withdraw_denom_balances(
    deps: Deps,
    rover_addr: &Addr,
    vault: &Vault,
) -> StdResult<Vec<Coin>> {
    let vault_info = vault.query_info(&deps.querier)?;
    query_balances(deps, rover_addr, &[vault_info.req_denom.as_str()])
}
