use cosmwasm_std::{to_binary, CosmosMsg, DepsMut, Env, Response, Uint128, WasmMsg};
use mars_rover::{
    adapters::vault::{UpdateType, Vault, VaultPositionUpdate},
    error::ContractResult,
    msg::{
        execute::{CallbackMsg, ChangeExpected},
        ExecuteMsg as RoverExecuteMsg,
    },
};

use crate::vault::utils::{query_withdraw_denom_balance, update_vault_position};

pub fn exit_vault(
    deps: DepsMut,
    env: Env,
    account_id: &str,
    vault: Vault,
    amount: Uint128,
) -> ContractResult<Response> {
    // Force indicates that the vault is one with a required lockup that needs to be broken
    // In this case, we'll need to withdraw from the locked bucket
    update_vault_position(
        deps.storage,
        account_id,
        &vault.address,
        VaultPositionUpdate::Unlocked(UpdateType::Decrement(amount)),
    )?;

    // Sends vault coins to vault in exchange for underlying assets
    let withdraw_msg = vault.withdraw_msg(&deps.querier, amount)?;

    // Updates coin balances for account after a vault withdraw has taken place
    let previous_balance =
        query_withdraw_denom_balance(deps.as_ref(), &env.contract.address, &vault)?;
    let update_coin_balance_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        funds: vec![],
        msg: to_binary(&RoverExecuteMsg::Callback(CallbackMsg::UpdateCoinBalance {
            account_id: account_id.to_string(),
            previous_balance,
            change: ChangeExpected::Increase,
        }))?,
    });

    Ok(Response::new()
        .add_message(withdraw_msg)
        .add_message(update_coin_balance_msg)
        .add_attribute("action", "vault/exit")
        .add_attribute("account_id", account_id)
        .add_attribute("vault_addr", vault.address.to_string())
        .add_attribute("amount_withdrawn", amount.to_string()))
}
