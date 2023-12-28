use cosmwasm_std::{Coin, Decimal, DepsMut, Env, Response, Uint128};
use mars_types::{
    credit_manager::{ActionAmount, ActionCoin, ChangeExpected},
    swapper_v2::SwapperRoute,
};

use crate::{
    error::{ContractError, ContractResult},
    state::COIN_BALANCES,
    utils::{
        assert_coin_is_whitelisted, assert_slippage, decrement_coin_balance, update_balance_msg,
    },
};

pub fn swap_exact_in(
    mut deps: DepsMut,
    env: Env,
    account_id: &str,
    coin_in: &ActionCoin,
    denom_out: &str,
    slippage: Decimal,
    route: SwapperRoute,
) -> ContractResult<Response> {
    assert_slippage(deps.storage, slippage)?;

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

    // TODO remove swapper addr
    // let swapper = SWAPPER.load(deps.storage)?;
    let swap_msg = route.swap_msg(
        &deps.querier,
        &env,
        coin_in_to_trade.clone(),
        denom_out.to_string(),
        slippage,
    )?;

    Ok(Response::new()
        .add_message(swap_msg)
        .add_message(update_coin_balance_msg)
        .add_attribute("action", "swapper")
        .add_attribute("account_id", account_id)
        .add_attribute("coin_in", coin_in_to_trade.to_string())
        .add_attribute("denom_out", denom_out))
}
