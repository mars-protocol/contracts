use cosmwasm_std::{Coin, DepsMut, Env, Response, Uint128};
use mars_types::{
    credit_manager::{ActionAmount, ActionCoin, ChangeExpected},
    swapper::SwapperRoute,
};

use crate::{
    error::{ContractError, ContractResult},
    state::{COIN_BALANCES, REWARDS_COLLECTOR, SWAPPER, SWAP_FEE},
    utils::{decrement_coin_balance, increment_coin_balance, update_balance_msg},
};

pub fn swap_exact_in(
    deps: DepsMut,
    env: Env,
    account_id: &str,
    coin_in: &ActionCoin,
    denom_out: &str,
    min_receive: Uint128,
    route: Option<SwapperRoute>,
) -> ContractResult<Response> {
    let mut coin_in_to_trade = Coin {
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

    // Deduct the swap fee
    let swap_fee = SWAP_FEE.load(deps.storage)?;
    let swap_fee_amount = coin_in_to_trade.amount.checked_mul_floor(swap_fee)?;
    coin_in_to_trade.amount = coin_in_to_trade.amount.checked_sub(swap_fee_amount)?;

    // Send to Rewards collector
    let rc_coin = Coin {
        denom: coin_in.denom.clone(),
        amount: swap_fee_amount,
    };
    let rewards_collector_account = REWARDS_COLLECTOR.load(deps.storage)?.account_id;
    increment_coin_balance(deps.storage, &rewards_collector_account, &rc_coin)?;

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
        .add_message(swapper.swap_exact_in_msg(&coin_in_to_trade, denom_out, min_receive, route)?)
        .add_message(update_coin_balance_msg)
        .add_attribute("action", "swapper")
        .add_attribute("account_id", account_id)
        .add_attribute("coin_in", coin_in_to_trade.to_string())
        .add_attribute("denom_out", denom_out))
}
