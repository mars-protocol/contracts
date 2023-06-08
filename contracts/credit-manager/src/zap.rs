use cosmwasm_std::{Coin, Deps, DepsMut, Env, Response, Uint128};
use mars_rover::{
    error::{ContractError, ContractResult},
    msg::execute::{ActionAmount, ActionCoin},
    traits::{Denoms, Stringify},
};

use crate::{
    state::{COIN_BALANCES, ZAPPER},
    utils::{
        assert_coin_is_whitelisted, assert_coins_are_whitelisted, decrement_coin_balance,
        update_balance_msg, update_balances_msgs,
    },
};

pub fn provide_liquidity(
    mut deps: DepsMut,
    env: Env,
    account_id: &str,
    coins_in: Vec<ActionCoin>,
    lp_token_out: &str,
    minimum_receive: Uint128,
) -> ContractResult<Response> {
    assert_coin_is_whitelisted(&mut deps, lp_token_out)?;
    assert_coins_are_whitelisted(&mut deps, coins_in.to_denoms())?;

    // Decrement coin amounts in account for those sent to pool
    let mut updated_coins_in: Vec<Coin> = Vec::with_capacity(coins_in.len());
    for coin_in in coins_in {
        let coin_balance = COIN_BALANCES.load(deps.storage, (account_id, &coin_in.denom))?;
        let new_amount = match coin_in.amount {
            ActionAmount::Exact(amt) => amt,
            ActionAmount::AccountBalance => coin_balance,
        };
        let updated_coin = Coin {
            denom: coin_in.denom,
            amount: new_amount,
        };
        decrement_coin_balance(deps.storage, account_id, &updated_coin)?;
        updated_coins_in.push(updated_coin);
    }

    // After zap is complete, update account's LP token balance
    let zapper = ZAPPER.load(deps.storage)?;
    let zap_msg = zapper.provide_liquidity_msg(&updated_coins_in, lp_token_out, minimum_receive)?;
    let update_balance_msg =
        update_balance_msg(&deps.querier, &env.contract.address, account_id, lp_token_out)?;

    Ok(Response::new()
        .add_message(zap_msg)
        .add_message(update_balance_msg)
        .add_attribute("action", "provide_liquidity")
        .add_attribute("account_id", account_id)
        .add_attribute("coins_in", updated_coins_in.as_slice().to_string())
        .add_attribute("lp_token_out", lp_token_out))
}

pub fn withdraw_liquidity(
    mut deps: DepsMut,
    env: Env,
    account_id: &str,
    lp_token_action: &ActionCoin,
    minimum_receive: Vec<Coin>,
) -> ContractResult<Response> {
    assert_coin_is_whitelisted(&mut deps, &lp_token_action.denom)?;

    let lp_token = Coin {
        denom: lp_token_action.denom.clone(),
        amount: match lp_token_action.amount {
            ActionAmount::Exact(a) => a,
            ActionAmount::AccountBalance => COIN_BALANCES
                .may_load(deps.storage, (account_id, &lp_token_action.denom))?
                .unwrap_or(Uint128::zero()),
        },
    };

    if lp_token.amount.is_zero() {
        return Err(ContractError::NoAmount);
    }

    let zapper = ZAPPER.load(deps.storage)?;
    let coins_out = zapper.estimate_withdraw_liquidity(&deps.querier, &lp_token)?;
    assert_coins_are_whitelisted(&mut deps, coins_out.to_denoms())?;

    decrement_coin_balance(deps.storage, account_id, &lp_token)?;

    // After unzap is complete, update account's coin balances
    let zap_msg = zapper.withdraw_liquidity_msg(&lp_token, minimum_receive)?;
    let update_balances_msgs = update_balances_msgs(
        &deps.querier,
        &env.contract.address,
        account_id,
        coins_out.to_denoms(),
    )?;

    Ok(Response::new()
        .add_message(zap_msg)
        .add_messages(update_balances_msgs)
        .add_attribute("action", "withdraw_liquidity")
        .add_attribute("account_id", account_id)
        .add_attribute("coin_in", lp_token.to_string())
        .add_attribute("coins_out", coins_out.as_slice().to_string()))
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
