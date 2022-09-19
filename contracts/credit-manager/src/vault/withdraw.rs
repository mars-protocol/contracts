use cosmwasm_std::{to_binary, CosmosMsg, DepsMut, Env, Response, Uint128, WasmMsg};

use rover::adapters::Vault;
use rover::error::ContractResult;
use rover::msg::execute::CallbackMsg;
use rover::msg::ExecuteMsg as RoverExecuteMsg;

use crate::update_coin_balances::query_balances;
use crate::vault::utils::{assert_vault_is_whitelisted, decrement_vault_position};

pub fn withdraw_from_vault(
    deps: DepsMut,
    env: Env,
    account_id: &str,
    vault: Vault,
    amount: Uint128,
    force: bool,
) -> ContractResult<Response> {
    assert_vault_is_whitelisted(deps.storage, &vault)?;

    decrement_vault_position(deps.storage, account_id, &vault, amount, force)?;

    // Sends vault coins to vault in exchange for underlying assets
    let withdraw_msg = vault.withdraw_msg(&deps.querier, amount, force)?;

    // Updates coin balances for account after a vault withdraw has taken place
    let vault_info = vault.query_vault_info(&deps.querier)?;
    let denoms = vault_info
        .coins
        .iter()
        .map(|v| v.denom.as_str())
        .collect::<Vec<_>>();
    let previous_balances =
        query_balances(deps.as_ref(), &env.contract.address, denoms.as_slice())?;
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
