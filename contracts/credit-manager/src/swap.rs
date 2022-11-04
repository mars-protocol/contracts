use cosmwasm_std::{Coin, Decimal, DepsMut, Env, Response};

use rover::error::{ContractError, ContractResult};

use crate::state::SWAPPER;
use crate::utils::{assert_coins_are_whitelisted, decrement_coin_balance, update_balance_msg};

pub fn swap_exact_in(
    deps: DepsMut,
    env: Env,
    account_id: &str,
    coin_in: Coin,
    denom_out: &str,
    slippage: Decimal,
) -> ContractResult<Response> {
    assert_coins_are_whitelisted(deps.storage, vec![coin_in.denom.as_str(), denom_out])?;

    if coin_in.amount.is_zero() {
        return Err(ContractError::NoAmount);
    }

    decrement_coin_balance(deps.storage, account_id, &coin_in)?;

    // Updates coin balances for account after the swap has taken place
    let update_coin_balance_msg =
        update_balance_msg(&deps.querier, &env.contract.address, account_id, denom_out)?;

    let swapper = SWAPPER.load(deps.storage)?;

    Ok(Response::new()
        .add_message(swapper.swap_exact_in_msg(&coin_in, denom_out, slippage)?)
        .add_message(update_coin_balance_msg)
        .add_attribute("action", "rover/credit-manager/swapper"))
}
