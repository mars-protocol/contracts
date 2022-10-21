use cosmwasm_std::{to_binary, CosmosMsg, DepsMut, Env, Response, Uint128, WasmMsg};

use rover::adapters::vault::{UpdateType, Vault, VaultPositionUpdate};
use rover::error::ContractResult;
use rover::msg::execute::CallbackMsg;
use rover::msg::ExecuteMsg as RoverExecuteMsg;

use crate::vault::utils::{
    assert_vault_is_whitelisted, query_withdraw_denom_balances, update_vault_position,
};

pub fn withdraw_from_vault(
    deps: DepsMut,
    env: Env,
    account_id: &str,
    vault: Vault,
    amount: Uint128,
    force: bool,
) -> ContractResult<Response> {
    assert_vault_is_whitelisted(deps.storage, &vault)?;

    // Force indicates that the vault is one with a required lockup that needs to be broken
    // In this case, we'll need to withdraw from the locked bucket
    update_vault_position(
        deps.storage,
        account_id,
        &vault.address,
        if force {
            VaultPositionUpdate::Locked(UpdateType::Decrement(amount))
        } else {
            VaultPositionUpdate::Unlocked(UpdateType::Decrement(amount))
        },
    )?;

    // Sends vault coins to vault in exchange for underlying assets
    let withdraw_msg = vault.withdraw_msg(&deps.querier, amount, force)?;

    // Updates coin balances for account after a vault withdraw has taken place
    let previous_balances =
        query_withdraw_denom_balances(deps.as_ref(), &env.contract.address, &vault)?;
    let update_coin_balance_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        funds: vec![],
        msg: to_binary(&RoverExecuteMsg::Callback(
            CallbackMsg::UpdateCoinBalances {
                account_id: account_id.to_string(),
                previous_balances,
            },
        ))?,
    });

    Ok(Response::new()
        .add_message(withdraw_msg)
        .add_message(update_coin_balance_msg)
        .add_attribute("action", "rover/credit_manager/vault/withdraw"))
}
