use cosmwasm_std::{Addr, Coin, Deps, StdResult, Storage};

use rover::adapters::{Vault, VaultPosition, VaultPositionState, VaultPositionUpdate};
use rover::error::{ContractError, ContractResult};

use crate::state::{ALLOWED_VAULTS, VAULT_POSITIONS};
use crate::update_coin_balances::query_balances;

pub fn assert_vault_is_whitelisted(storage: &mut dyn Storage, vault: &Vault) -> ContractResult<()> {
    let is_whitelisted = ALLOWED_VAULTS.has(storage, &vault.address);
    if !is_whitelisted {
        return Err(ContractError::NotWhitelisted(vault.address.to_string()));
    }
    Ok(())
}

pub fn update_vault_position(
    storage: &mut dyn Storage,
    account_id: &str,
    vault_addr: &Addr,
    update: VaultPositionUpdate,
) -> ContractResult<VaultPositionState> {
    let path = VAULT_POSITIONS.key((account_id, vault_addr.clone()));
    let mut new_position = path.may_load(storage)?.unwrap_or_default();

    match update {
        VaultPositionUpdate::DecrementUnlocked(amount) => {
            new_position.unlocked = new_position.unlocked.checked_sub(amount)?;
        }
        VaultPositionUpdate::IncrementUnlocked(amount) => {
            new_position.unlocked = new_position.unlocked.checked_add(amount)?;
        }
        VaultPositionUpdate::DecrementLocked(amount) => {
            new_position.locked = new_position.locked.checked_sub(amount)?;
        }
        VaultPositionUpdate::IncrementLocked(amount) => {
            new_position.locked = new_position.locked.checked_add(amount)?;
        }
        VaultPositionUpdate::AddUnlocking(position) => {
            new_position.unlocking.push(position);
        }
        VaultPositionUpdate::RemoveUnlocking(id) => new_position.unlocking.retain(|p| p.id != id),
    }

    if new_position == VaultPositionState::default() {
        path.remove(storage);
    } else {
        path.save(storage, &new_position)?;
    }
    Ok(new_position)
}

/// Returns the denoms you may receive on a withdraw
/// Inferred by vault entry requirements
pub fn query_withdraw_denom_balances(
    deps: Deps,
    rover_addr: &Addr,
    vault: &Vault,
) -> StdResult<Vec<Coin>> {
    let vault_info = vault.query_info(&deps.querier)?;
    let denoms = vault_info
        .accepts
        .iter()
        .flat_map(|v| v.iter().map(|s| s.as_str()))
        .collect::<Vec<_>>();
    query_balances(deps, rover_addr, denoms.as_slice())
}

/// Does a simulated withdraw from multiple vault positions to see what assets would be returned
pub fn simulate_withdraw(deps: &Deps, positions: &[VaultPosition]) -> ContractResult<Vec<Coin>> {
    let mut coins: Vec<Coin> = vec![];
    for p in positions {
        let total_vault_coins = p.state.total()?;
        let coins_if_withdrawn = p
            .vault
            .query_preview_redeem(&deps.querier, total_vault_coins)?;
        coins.extend(coins_if_withdrawn)
    }
    Ok(coins)
}
