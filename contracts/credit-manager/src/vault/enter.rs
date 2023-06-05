use cosmwasm_std::{
    to_binary, Addr, Coin, CosmosMsg, Deps, DepsMut, QuerierWrapper, Response, Uint128, WasmMsg,
};
use mars_rover::{
    adapters::vault::{UpdateType, Vault, VaultPositionUpdate},
    error::{ContractError, ContractResult},
    msg::{
        execute::{ActionAmount, ActionCoin, CallbackMsg},
        ExecuteMsg,
    },
};

use crate::{
    state::{COIN_BALANCES, ORACLE, PARAMS},
    utils::{assert_coin_is_whitelisted, decrement_coin_balance},
    vault::{
        rover_vault_coin_balance_value,
        utils::{assert_vault_is_whitelisted, update_vault_position},
    },
};

pub fn enter_vault(
    mut deps: DepsMut,
    rover_addr: &Addr,
    account_id: &str,
    vault: Vault,
    coin: &ActionCoin,
) -> ContractResult<Response> {
    let amount = match coin.amount {
        ActionAmount::Exact(a) => a,
        ActionAmount::AccountBalance => {
            COIN_BALANCES.load(deps.storage, (account_id, &coin.denom))?
        }
    };
    let coin_to_enter = Coin {
        denom: coin.denom.clone(),
        amount,
    };

    assert_coin_is_whitelisted(&mut deps, &coin.denom)?;
    assert_vault_is_whitelisted(&mut deps, &vault)?;
    assert_denom_matches_vault_reqs(deps.querier, &vault, &coin_to_enter)?;
    assert_deposit_is_under_cap(deps.as_ref(), &vault, &coin_to_enter, rover_addr)?;

    decrement_coin_balance(deps.storage, account_id, &coin_to_enter)?;

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
        .add_message(vault.deposit_msg(&coin_to_enter)?)
        .add_message(update_vault_balance_msg)
        .add_attribute("action", "vault/enter")
        .add_attribute("account_id", account_id)
        .add_attribute("vault_addr", vault.address.to_string())
        .add_attribute("amount_deposited", amount.to_string()))
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
    let duration = vault.query_lockup_duration(&deps.querier).ok();

    update_vault_position(
        deps.storage,
        account_id,
        &vault.address,
        match duration {
            None => VaultPositionUpdate::Unlocked(UpdateType::Increment(diff)),
            Some(_) => VaultPositionUpdate::Locked(UpdateType::Increment(diff)),
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "vault/update_balance")
        .add_attribute("account_id", account_id.to_string())
        .add_attribute("vault_addr", vault.address.to_string())
        .add_attribute("amount_incremented", current_balance.checked_sub(previous_total_balance)?))
}

pub fn assert_denom_matches_vault_reqs(
    querier: QuerierWrapper,
    vault: &Vault,
    coin: &Coin,
) -> ContractResult<()> {
    let vault_info = vault.query_info(&querier)?;
    if vault_info.base_token != coin.denom {
        return Err(ContractError::RequirementsNotMet(format!(
            "Required coin: {} -- does not match given coin: {}",
            vault_info.base_token, coin.denom
        )));
    }
    Ok(())
}

pub fn assert_deposit_is_under_cap(
    deps: Deps,
    vault: &Vault,
    coin_to_add: &Coin,
    rover_addr: &Addr,
) -> ContractResult<()> {
    let oracle = ORACLE.load(deps.storage)?;
    let deposit_request_value = oracle.query_total_value(&deps.querier, &[coin_to_add.clone()])?;
    let rover_vault_balance_value = rover_vault_coin_balance_value(&deps, vault, rover_addr)?;

    let new_total_vault_value = rover_vault_balance_value.checked_add(deposit_request_value)?;

    let params = PARAMS.load(deps.storage)?;
    let config = params.query_vault_config(&deps.querier, &vault.address)?;
    let deposit_cap_value = oracle.query_total_value(&deps.querier, &[config.deposit_cap])?;

    if new_total_vault_value > deposit_cap_value {
        return Err(ContractError::AboveVaultDepositCap {
            new_value: new_total_vault_value.to_string(),
            maximum: deposit_cap_value.to_string(),
        });
    }

    Ok(())
}
