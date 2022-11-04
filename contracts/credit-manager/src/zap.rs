use cosmwasm_std::{Coin, Deps, DepsMut, Env, Response, Uint128};

use mars_rover::error::ContractResult;
use mars_rover::traits::Denoms;

use crate::state::ZAPPER;
use crate::utils::{
    assert_coin_is_whitelisted, assert_coins_are_whitelisted, decrement_coin_balance,
    update_balance_msg, update_balances_msgs,
};

pub fn provide_liquidity(
    deps: DepsMut,
    env: Env,
    account_id: &str,
    coins_in: Vec<Coin>,
    lp_token_out: &str,
    minimum_receive: Uint128,
) -> ContractResult<Response> {
    assert_coin_is_whitelisted(deps.storage, lp_token_out)?;
    assert_coins_are_whitelisted(deps.storage, coins_in.to_denoms())?;

    // Decrement coin amounts in account for those sent to pool
    for coin_in in &coins_in {
        decrement_coin_balance(deps.storage, account_id, coin_in)?;
    }

    // After zap is complete, update account's LP token balance
    let zapper = ZAPPER.load(deps.storage)?;
    let zap_msg = zapper.provide_liquidity_msg(&coins_in, lp_token_out, minimum_receive)?;
    let update_balance_msg = update_balance_msg(
        &deps.querier,
        &env.contract.address,
        account_id,
        lp_token_out,
    )?;

    Ok(Response::new()
        .add_message(zap_msg)
        .add_message(update_balance_msg)
        .add_attribute("action", "rover/credit-manager/provide_liquidity"))
}

pub fn withdraw_liquidity(
    deps: DepsMut,
    env: Env,
    account_id: &str,
    lp_token: Coin,
) -> ContractResult<Response> {
    assert_coin_is_whitelisted(deps.storage, &lp_token.denom)?;

    let zapper = ZAPPER.load(deps.storage)?;
    let coins_out = zapper.estimate_withdraw_liquidity(&deps.querier, &lp_token)?;
    assert_coins_are_whitelisted(deps.storage, coins_out.to_denoms())?;

    decrement_coin_balance(deps.storage, account_id, &lp_token)?;

    // After unzap is complete, update account's coin balances
    let zap_msg = zapper.withdraw_liquidity_msg(&lp_token)?;
    let update_balances_msgs = update_balances_msgs(
        &deps.querier,
        &env.contract.address,
        account_id,
        coins_out.to_denoms(),
    )?;

    Ok(Response::new()
        .add_message(zap_msg)
        .add_messages(update_balances_msgs)
        .add_attribute("action", "rover/credit-manager/withdraw_liquidity"))
}

pub fn estimate_provide_liquidity(
    deps: Deps,
    lp_token_out: &str,
    coins_in: Vec<Coin>,
) -> ContractResult<Uint128> {
    let zapper = ZAPPER.load(deps.storage)?;
    let estimate = zapper.estimate_provide_liquidity(&deps.querier, lp_token_out, &coins_in)?;
    Ok(estimate)
}

pub fn estimate_withdraw_liquidity(deps: Deps, lp_token: Coin) -> ContractResult<Vec<Coin>> {
    let zapper = ZAPPER.load(deps.storage)?;
    let estimate = zapper.estimate_withdraw_liquidity(&deps.querier, &lp_token)?;
    Ok(estimate)
}
