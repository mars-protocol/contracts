use cosmwasm_std::{
    CheckedMultiplyFractionError, Coin, Decimal, Deps, DepsMut, Env, Response, Uint128,
};
use mars_types::{
    credit_manager::{ActionAmount, ActionCoin, ChangeExpected},
    traits::{Denoms, Stringify},
};

use crate::{
    error::{ContractError, ContractResult},
    state::{COIN_BALANCES, ZAPPER},
    utils::{
        assert_coin_is_whitelisted, assert_coins_are_whitelisted, assert_slippage,
        decrement_coin_balance, update_balance_msg, update_balances_msgs,
    },
};

pub fn provide_liquidity(
    mut deps: DepsMut,
    env: Env,
    account_id: &str,
    coins_in: Vec<ActionCoin>,
    lp_token_out: &str,
    slippage: Decimal,
) -> ContractResult<Response> {
    assert_slippage(deps.storage, slippage)?;

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

    let zapper = ZAPPER.load(deps.storage)?;

    // Estimate how much LP token will be received from zapper with applied slippage
    let estimated_min_receive =
        zapper.estimate_provide_liquidity(&deps.querier, lp_token_out, &updated_coins_in)?;
    let estimated_min_receive_slippage =
        estimated_min_receive.checked_mul_floor(Decimal::one() - slippage)?;

    let zap_msg = zapper.provide_liquidity_msg(
        &updated_coins_in,
        lp_token_out,
        estimated_min_receive_slippage,
    )?;

    // After zap is complete, update account's LP token balance
    let update_balance_msg = update_balance_msg(
        &deps.querier,
        &env.contract.address,
        account_id,
        lp_token_out,
        ChangeExpected::Increase,
    )?;

    Ok(Response::new()
        .add_message(zap_msg)
        .add_message(update_balance_msg)
        .add_attribute("action", "provide_liquidity")
        .add_attribute("account_id", account_id)
        .add_attribute("coins_in", updated_coins_in.as_slice().to_string())
        .add_attribute("lp_token_out", lp_token_out))
}

pub fn withdraw_liquidity(
    deps: DepsMut,
    env: Env,
    account_id: &str,
    lp_token_action: &ActionCoin,
    slippage: Decimal,
) -> ContractResult<Response> {
    assert_slippage(deps.storage, slippage)?;

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
    decrement_coin_balance(deps.storage, account_id, &lp_token)?;

    // Estimate how much coins will be received from zapper with applied slippage
    let estimated_coins_out = zapper.estimate_withdraw_liquidity(&deps.querier, &lp_token)?;
    let estimated_coins_out_slippage = estimated_coins_out
        .iter()
        .map(|c| {
            let amount = c.amount.checked_mul_floor(Decimal::one() - slippage)?;
            Ok(Coin {
                denom: c.denom.clone(),
                amount,
            })
        })
        .collect::<Result<Vec<Coin>, CheckedMultiplyFractionError>>()?;

    let unzap_msg = zapper.withdraw_liquidity_msg(&lp_token, estimated_coins_out_slippage)?;

    // After unzap is complete, update account's coin balances
    let update_balances_msgs = update_balances_msgs(
        &deps.querier,
        &env.contract.address,
        account_id,
        estimated_coins_out.to_denoms(),
        ChangeExpected::Increase,
    )?;

    Ok(Response::new()
        .add_message(unzap_msg)
        .add_messages(update_balances_msgs)
        .add_attribute("action", "withdraw_liquidity")
        .add_attribute("account_id", account_id)
        .add_attribute("coin_in", lp_token.to_string())
        .add_attribute("coins_out", estimated_coins_out.as_slice().to_string()))
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
