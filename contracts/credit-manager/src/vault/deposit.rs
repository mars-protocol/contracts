use cosmwasm_std::{
    to_binary, Addr, Coin, CosmosMsg, DepsMut, QuerierWrapper, Response, Uint128, WasmMsg,
};

use rover::adapters::{Vault, VaultPositionState};
use rover::error::{ContractError, ContractResult};
use rover::msg::execute::CallbackMsg;
use rover::msg::ExecuteMsg;
use rover::traits::Stringify;

use crate::state::VAULT_POSITIONS;
use crate::utils::{assert_coins_are_whitelisted, decrement_coin_balance};
use crate::vault::utils::assert_vault_is_whitelisted;

pub fn deposit_into_vault(
    deps: DepsMut,
    rover_addr: &Addr,
    account_id: &str,
    vault: Vault,
    coins: &[Coin],
) -> ContractResult<Response> {
    let denoms = coins.iter().map(|c| c.denom.as_str()).collect();
    assert_coins_are_whitelisted(deps.storage, denoms)?;
    assert_vault_is_whitelisted(deps.storage, &vault)?;
    assert_denoms_match_vault_reqs(deps.querier, &vault, coins)?;

    // Decrement token's coin balance amount
    coins.iter().try_for_each(|coin| -> ContractResult<_> {
        decrement_coin_balance(deps.storage, account_id, coin)?;
        Ok(())
    })?;

    let current_balance = vault.query_balance(&deps.querier, rover_addr)?;
    let update_vault_balance_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: rover_addr.to_string(),
        funds: vec![],
        msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::UpdateVaultCoinBalance {
            vault: vault.clone(),
            account_id: account_id.to_string(),
            previous_total_balance: current_balance,
        }))?,
    });

    Ok(Response::new()
        .add_message(vault.deposit_msg(coins)?)
        .add_message(update_vault_balance_msg)
        .add_attribute("action", "rover/credit_manager/vault/deposit"))
}

pub fn update_vault_coin_balance(
    deps: DepsMut,
    vault: Vault,
    account_id: &str,
    previous_total_balance: Uint128,
    rover_addr: &Addr,
) -> ContractResult<Response> {
    let current_balance = vault.query_balance(&deps.querier, rover_addr)?;

    if previous_total_balance >= current_balance {
        return Err(ContractError::NoVaultCoinsReceived);
    }

    let diff = current_balance.checked_sub(previous_total_balance)?;
    let vault_info = vault.query_vault_info(&deps.querier)?;

    // Increment token's vault position
    VAULT_POSITIONS.update(
        deps.storage,
        (account_id, vault.address),
        |position_opt| -> ContractResult<_> {
            let p = position_opt.unwrap_or_default();
            match vault_info.lockup {
                None => Ok(VaultPositionState {
                    unlocked: p.unlocked.checked_add(diff)?,
                    locked: p.locked,
                }),
                Some(_) => Ok(VaultPositionState {
                    unlocked: p.unlocked,
                    locked: p.locked.checked_add(diff)?,
                }),
            }
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "rover/credit_manager/vault/update_balance")
        .add_attribute(
            "amount_incremented",
            current_balance.checked_sub(previous_total_balance)?,
        ))
}

pub fn assert_denoms_match_vault_reqs(
    querier: QuerierWrapper,
    vault: &Vault,
    assets: &[Coin],
) -> ContractResult<()> {
    let vault_info = vault.query_vault_info(&querier)?;

    let all_req_coins_present = vault_info
        .coins
        .iter()
        .all(|coin| assets.iter().any(|req_coin| req_coin.denom == coin.denom));

    if !all_req_coins_present || assets.len() != vault_info.coins.len() {
        return Err(ContractError::RequirementsNotMet(format!(
            "Required assets: {} -- do not match given assets: {}",
            vault_info.coins.as_slice().to_string(),
            assets.to_string()
        )));
    }
    Ok(())
}
