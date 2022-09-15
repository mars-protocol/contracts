use cosmwasm_std::{Storage, Uint128};

use rover::adapters::{Vault, VaultPosition};
use rover::error::{ContractError, ContractResult};

use crate::state::{ALLOWED_VAULTS, VAULT_POSITIONS};

pub fn assert_vault_is_whitelisted(storage: &mut dyn Storage, vault: &Vault) -> ContractResult<()> {
    let is_whitelisted = ALLOWED_VAULTS.has(storage, vault.address());
    if !is_whitelisted {
        return Err(ContractError::NotWhitelisted(vault.address().to_string()));
    }
    Ok(())
}

pub fn decrement_vault_position(
    storage: &mut dyn Storage,
    token_id: &str,
    vault: &Vault,
    amount: Uint128,
    force: bool,
) -> ContractResult<VaultPosition> {
    let path = VAULT_POSITIONS.key((token_id, vault.address().clone()));
    let mut position = path.load(storage)?;

    // Force indicates that the vault is one with a required lockup that needs to be broken
    // In this case, we'll need to withdraw from the locked bucket
    if force {
        position.locked = position.locked.checked_sub(amount)?;
    } else {
        position.unlocked = position.unlocked.checked_sub(amount)?;
    }

    if position == VaultPosition::default() {
        path.remove(storage);
    } else {
        path.save(storage, &position)?;
    }

    Ok(position)
}
