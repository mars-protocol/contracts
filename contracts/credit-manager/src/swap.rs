use cosmwasm_std::{to_binary, Coin, CosmosMsg, Decimal, DepsMut, Env, Response, WasmMsg};

use rover::error::{ContractError, ContractResult};
use rover::msg::execute::CallbackMsg;
use rover::msg::ExecuteMsg;
use rover::NftTokenId;

use crate::state::SWAPPER;
use crate::update_coin_balances::query_balances;
use crate::utils::{assert_coins_are_whitelisted, decrement_coin_balance};

pub fn swap_exact_in(
    deps: DepsMut,
    env: Env,
    token_id: NftTokenId,
    coin_in: Coin,
    denom_out: &str,
    slippage: Decimal,
) -> ContractResult<Response> {
    assert_coins_are_whitelisted(deps.storage, vec![coin_in.denom.as_str(), denom_out])?;

    if coin_in.amount.is_zero() {
        return Err(ContractError::NoAmount);
    }

    decrement_coin_balance(deps.storage, token_id, &coin_in)?;

    // Updates coin balances for account after the swap has taken place
    let previous_balances = query_balances(deps.as_ref(), &env.contract.address, &[denom_out])?;
    let update_coin_balance_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        funds: vec![],
        msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::UpdateCoinBalances {
            token_id: token_id.to_string(),
            previous_balances,
        }))?,
    });

    let swapper = SWAPPER.load(deps.storage)?;

    Ok(Response::new()
        .add_message(swapper.swap_exact_in_msg(&coin_in, denom_out, slippage)?)
        .add_message(update_coin_balance_msg)
        .add_attribute("action", "rover/credit_manager/swapper"))
}
