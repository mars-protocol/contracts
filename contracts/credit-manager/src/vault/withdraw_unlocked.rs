use cosmwasm_std::{to_binary, CosmosMsg, DepsMut, Env, Response, Uint128, WasmMsg};

use rover::adapters::{Vault, VaultPositionUpdate};
use rover::error::{ContractError, ContractResult};
use rover::msg::execute::CallbackMsg;
use rover::msg::ExecuteMsg;

use crate::state::VAULT_POSITIONS;
use crate::vault::utils::{
    assert_vault_is_whitelisted, query_withdraw_denom_balances, update_vault_position,
};

pub fn withdraw_unlocked_from_vault(
    deps: DepsMut,
    env: Env,
    account_id: &str,
    vault: Vault,
    position_id: Uint128,
) -> ContractResult<Response> {
    assert_vault_is_whitelisted(deps.storage, &vault)?;

    let vault_position = VAULT_POSITIONS.load(deps.storage, (account_id, vault.address.clone()))?;

    let matching_unlock = vault_position
        .unlocking
        .iter()
        .find(|p| p.id == position_id)
        .ok_or_else(|| ContractError::NoPositionMatch(position_id.to_string()))?;

    let matching_unlock = vault.query_unlocking_position_info(&deps.querier, matching_unlock.id)?;

    if matching_unlock.unlocked_at > env.block.time {
        return Err(ContractError::UnlockNotReady {});
    }

    update_vault_position(
        deps.storage,
        account_id,
        &vault.address,
        VaultPositionUpdate::RemoveUnlocking(position_id),
    )?;

    // Updates coin balances for account after the withdraw has taken place
    let previous_balances =
        query_withdraw_denom_balances(deps.as_ref(), &env.contract.address, &vault)?;
    let update_coin_balance_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        funds: vec![],
        msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::UpdateCoinBalances {
            account_id: account_id.to_string(),
            previous_balances,
        }))?,
    });

    let withdraw_unlocked_msg = vault.withdraw_unlocked_msg(position_id)?;

    Ok(Response::new()
        .add_message(withdraw_unlocked_msg)
        .add_message(update_coin_balance_msg)
        .add_attribute("action", "rover/credit_manager/vault/unlock"))
}
