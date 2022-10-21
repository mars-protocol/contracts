use cosmwasm_std::{
    coin, to_binary, Addr, Coin, CosmosMsg, Deps, DepsMut, QuerierWrapper, Response, Uint128,
    WasmMsg,
};

use rover::adapters::{UpdateType, Vault, VaultPositionUpdate};
use rover::error::{ContractError, ContractResult};
use rover::msg::execute::CallbackMsg;
use rover::msg::ExecuteMsg;

use crate::state::{ORACLE, VAULT_DEPOSIT_CAPS};
use crate::utils::{assert_coins_are_whitelisted, contents_equal, decrement_coin_balance};
use crate::vault::utils::{assert_vault_is_whitelisted, update_vault_position};

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
    assert_deposit_is_under_cap(deps.as_ref(), &vault, coins, rover_addr)?;

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
    let vault_info = vault.query_info(&deps.querier)?;

    update_vault_position(
        deps.storage,
        account_id,
        &vault.address,
        match vault_info.lockup {
            None => VaultPositionUpdate::Unlocked {
                amount: diff,
                kind: UpdateType::Increment,
            },
            Some(_) => VaultPositionUpdate::Locked {
                amount: diff,
                kind: UpdateType::Increment,
            },
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
    coins: &[Coin],
) -> ContractResult<()> {
    let vault_info = vault.query_info(&querier)?;

    let given_denoms = coins.iter().map(|c| c.denom.clone()).collect::<Vec<_>>();
    let fulfills_reqs = contents_equal(&vault_info.accepts, &given_denoms);

    if !fulfills_reqs {
        return Err(ContractError::RequirementsNotMet(format!(
            "Required assets: {} -- do not match given assets: {}",
            vault_info.accepts.join(", "),
            given_denoms.join(", ")
        )));
    }
    Ok(())
}

pub fn assert_deposit_is_under_cap(
    deps: Deps,
    vault: &Vault,
    coins: &[Coin],
    rover_addr: &Addr,
) -> ContractResult<()> {
    let oracle = ORACLE.load(deps.storage)?;
    let deposit_request_value = oracle.query_total_value(&deps.querier, coins)?;

    let deposit_cap = VAULT_DEPOSIT_CAPS.load(deps.storage, &vault.address)?;
    let deposit_cap_value = oracle.query_total_value(&deps.querier, &[deposit_cap])?;

    let vault_info = vault.query_info(&deps.querier)?;
    let rover_vault_coin_balance = vault.query_balance(&deps.querier, rover_addr)?;
    let rover_vault_coins_value = oracle.query_total_value(
        &deps.querier,
        &[coin(
            rover_vault_coin_balance.u128(),
            vault_info.token_denom,
        )],
    )?;

    let new_total_vault_value = rover_vault_coins_value.checked_add(deposit_request_value)?;

    if new_total_vault_value > deposit_cap_value {
        return Err(ContractError::AboveVaultDepositCap {
            new_value: new_total_vault_value.to_string(),
            maximum: deposit_cap_value.to_string(),
        });
    }

    Ok(())
}
