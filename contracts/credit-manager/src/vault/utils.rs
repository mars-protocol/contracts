use cosmwasm_std::{Addr, Coin, Deps, StdResult, Storage, Uint128};
use mars_rover::{
    adapters::vault::{
        LockingVaultAmount, UnlockingPositions, Vault, VaultAmount, VaultPosition,
        VaultPositionAmount, VaultPositionUpdate,
    },
    error::{ContractError, ContractResult},
};

use crate::{
    state::{MAX_UNLOCKING_POSITIONS, ORACLE, VAULT_CONFIGS, VAULT_POSITIONS},
    update_coin_balances::query_balance,
};

pub fn assert_vault_is_whitelisted(storage: &dyn Storage, vault: &Vault) -> ContractResult<()> {
    let is_whitelisted = vault_is_whitelisted(storage, vault)?;
    if !is_whitelisted {
        return Err(ContractError::NotWhitelisted(vault.address.to_string()));
    }
    Ok(())
}

pub fn vault_is_whitelisted(storage: &dyn Storage, vault: &Vault) -> ContractResult<bool> {
    let config = VAULT_CONFIGS
        .may_load(storage, &vault.address)?
        .and_then(|config| config.whitelisted.then_some(true));
    Ok(config.is_some())
}

pub fn assert_under_max_unlocking_limit(
    storage: &mut dyn Storage,
    account_id: &str,
    vault: &Vault,
) -> ContractResult<()> {
    let maximum = MAX_UNLOCKING_POSITIONS.load(storage)?;
    let new_amount = VAULT_POSITIONS
        .may_load(storage, (account_id, vault.address.clone()))?
        .map(|p| p.unlocking().positions().len())
        .map(|len| Uint128::from(len as u128))
        .unwrap_or(Uint128::zero())
        .checked_add(Uint128::one())?;

    if new_amount > maximum {
        return Err(ContractError::ExceedsMaxUnlockingPositions {
            new_amount,
            maximum,
        });
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
    let mut amount = path.may_load(storage)?.unwrap_or_else(|| update.default_amount());

    amount.update(update)?;

    if amount.is_empty() {
        path.remove(storage);
    } else {
        path.save(storage, &amount)?;
    }
    Ok(amount)
}

/// Returns the total vault token balance for rover
pub fn query_withdraw_denom_balance(
    deps: Deps,
    rover_addr: &Addr,
    vault: &Vault,
) -> StdResult<Coin> {
    let vault_info = vault.query_info(&deps.querier)?;
    query_balance(&deps.querier, rover_addr, vault_info.base_token.as_str())
}

pub fn vault_utilization_in_deposit_cap_denom(
    deps: &Deps,
    vault: &Vault,
    rover_addr: &Addr,
) -> ContractResult<Coin> {
    let rover_vault_balance_value = rover_vault_coin_balance_value(deps, vault, rover_addr)?;
    let config = VAULT_CONFIGS.load(deps.storage, &vault.address)?;
    let oracle = ORACLE.load(deps.storage)?;
    let deposit_cap_denom_price =
        oracle.query_price(&deps.querier, &config.deposit_cap.denom)?.price;

    Ok(Coin {
        denom: config.deposit_cap.denom,
        amount: rover_vault_balance_value.checked_div_floor(deposit_cap_denom_price)?,
    })
}

/// Total value of vault coins under Rover's management for vault
pub fn rover_vault_coin_balance_value(
    deps: &Deps,
    vault: &Vault,
    rover_addr: &Addr,
) -> ContractResult<Uint128> {
    let oracle = ORACLE.load(deps.storage)?;
    let rover_vault_coin_balance = vault.query_balance(&deps.querier, rover_addr)?;
    let lockup = vault.query_lockup_duration(&deps.querier).ok();

    let position = VaultPosition {
        vault: vault.clone(),
        amount: match lockup {
            None => VaultPositionAmount::Unlocked(VaultAmount::new(rover_vault_coin_balance)),
            Some(_) => VaultPositionAmount::Locking(LockingVaultAmount {
                locked: VaultAmount::new(rover_vault_coin_balance),
                unlocking: UnlockingPositions::new(vec![]),
            }),
        },
    };
    let vault_coin_balance_val = position.query_values(&deps.querier, &oracle)?.vault_coin.value;
    Ok(vault_coin_balance_val)
}
