use cosmwasm_std::{Coin, Decimal, DepsMut, Env, Response, Uint128};
use mars_rover::{
    error::{ContractError, ContractResult},
    msg::execute::{ActionAmount, ActionCoin, ChangeExpected},
};

use crate::{
    state::{COIN_BALANCES, SWAPPER},
    utils::{assert_coin_is_whitelisted, decrement_coin_balance, update_balance_msg},
};

pub fn swap_exact_in(
    mut deps: DepsMut,
    env: Env,
    account_id: &str,
    coin_in: &ActionCoin,
    denom_out: &str,
    slippage: Decimal,
) -> ContractResult<Response> {
    assert_coin_is_whitelisted(&mut deps, denom_out)?;

    let coin_in_to_trade = Coin {
        denom: coin_in.denom.clone(),
        amount: match coin_in.amount {
            ActionAmount::Exact(a) => a,
            ActionAmount::AccountBalance => COIN_BALANCES
                .may_load(deps.storage, (account_id, &coin_in.denom))?
                .unwrap_or(Uint128::zero()),
        },
    };

    if coin_in_to_trade.amount.is_zero() {
        return Err(ContractError::NoAmount);
    }

    decrement_coin_balance(deps.storage, account_id, &coin_in_to_trade)?;

    // Updates coin balances for account after the swap has taken place
    let update_coin_balance_msg = update_balance_msg(
        &deps.querier,
        &env.contract.address,
        account_id,
        denom_out,
        ChangeExpected::Increase,
    )?;

    let swapper = SWAPPER.load(deps.storage)?;

    Ok(Response::new()
        .add_message(swapper.swap_exact_in_msg(&coin_in_to_trade, denom_out, slippage)?)
        .add_message(update_coin_balance_msg)
        .add_attribute("action", "swapper")
        .add_attribute("account_id", account_id)
        .add_attribute("coin_in", coin_in_to_trade.to_string())
        .add_attribute("denom_out", denom_out))
}
