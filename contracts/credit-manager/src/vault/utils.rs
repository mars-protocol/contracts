use cosmwasm_std::{
    coin, to_binary, Addr, Coin, Deps, QueryRequest, StdResult, Storage, WasmQuery,
};

use mars_oracle_adapter::msg::QueryMsg::PriceableUnderlying;
use rover::adapters::{
    UpdateType, Vault, VaultPosition, VaultPositionState, VaultPositionUpdate,
    VaultUnlockingPosition,
};
use rover::error::{ContractError, ContractResult};

use crate::state::{ALLOWED_VAULTS, ORACLE, VAULT_POSITIONS};
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
        VaultPositionUpdate::Unlocked { amount, kind } => match kind {
            UpdateType::Increment => {
                new_position.unlocked = new_position.unlocked.checked_add(amount)?;
            }
            UpdateType::Decrement => {
                new_position.unlocked = new_position.unlocked.checked_sub(amount)?;
            }
        },
        VaultPositionUpdate::Locked { amount, kind } => match kind {
            UpdateType::Increment => {
                new_position.locked = new_position.locked.checked_add(amount)?;
            }
            UpdateType::Decrement => {
                new_position.locked = new_position.locked.checked_sub(amount)?;
            }
        },
        VaultPositionUpdate::Unlocking { id, amount, kind } => match kind {
            UpdateType::Increment => {
                new_position
                    .unlocking
                    .push(VaultUnlockingPosition { id, amount });
            }
            UpdateType::Decrement => {
                let mut matching_unlock = get_unlocking_position(id, &new_position)?;
                matching_unlock.amount = matching_unlock.amount.checked_sub(amount)?;

                new_position.unlocking.retain(|p| p.id != id);
                if !matching_unlock.amount.is_zero() {
                    new_position.unlocking.push(matching_unlock);
                }
            }
        },
    }

    if new_position == VaultPositionState::default() {
        path.remove(storage);
    } else {
        path.save(storage, &new_position)?;
    }
    Ok(new_position)
}

/// Returns the denoms received on a withdraw, inferred by vault entry requirements
pub fn query_withdraw_denom_balances(
    deps: Deps,
    rover_addr: &Addr,
    vault: &Vault,
) -> StdResult<Vec<Coin>> {
    let vault_info = vault.query_info(&deps.querier)?;
    let denoms = vault_info
        .accepts
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    query_balances(deps, rover_addr, &denoms)
}

/// Returns the mars-oracle accepted priceable coins
pub fn get_priceable_coins(deps: &Deps, positions: &[VaultPosition]) -> ContractResult<Vec<Coin>> {
    let oracle = ORACLE.load(deps.storage)?;
    let mut coins: Vec<Coin> = vec![];
    for p in positions {
        let vault_info = p.vault.query_info(&deps.querier)?;
        let total_vault_coins = p.state.total()?;
        let priceable_coins: Vec<Coin> =
            deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: oracle.address().to_string(),
                msg: to_binary(&PriceableUnderlying {
                    coin: coin(total_vault_coins.u128(), vault_info.token_denom),
                })?,
            }))?;
        coins.extend(priceable_coins)
    }
    Ok(coins)
}

pub fn get_unlocking_position(
    position_id: u64,
    vault_position: &VaultPositionState,
) -> ContractResult<VaultUnlockingPosition> {
    vault_position
        .unlocking
        .iter()
        .find(|p| p.id == position_id)
        .ok_or_else(|| ContractError::NoPositionMatch(position_id.to_string()))
        .cloned()
}
