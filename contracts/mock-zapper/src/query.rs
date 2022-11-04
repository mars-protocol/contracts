use cosmwasm_std::{Coin, Deps, Storage, Uint128};

use crate::contract::STARTING_LP_POOL_TOKENS;
use crate::error::ContractError;
use crate::state::{COIN_BALANCES, LP_TOKEN_SUPPLY, ORACLE};

pub fn estimate_provide_liquidity(
    deps: &Deps,
    lp_token_out: &str,
    coins_in: Vec<Coin>,
) -> Result<Uint128, ContractError> {
    let total_supply = LP_TOKEN_SUPPLY
        .load(deps.storage, lp_token_out)
        .unwrap_or(Uint128::zero());

    let lp_tokens_estimate = if total_supply.is_zero() {
        STARTING_LP_POOL_TOKENS
    } else {
        let (coin0, coin1) = COIN_BALANCES.load(deps.storage, lp_token_out)?;
        let oracle = ORACLE.load(deps.storage)?;
        let total_underlying_value = oracle.query_total_value(&deps.querier, &[coin0, coin1])?;
        let given_value = oracle.query_total_value(&deps.querier, &coins_in)?;
        total_supply
            .checked_multiply_ratio(given_value.atomics(), total_underlying_value.atomics())?
    };
    Ok(lp_tokens_estimate)
}

pub fn estimate_withdraw_liquidity(
    storage: &dyn Storage,
    lp_token: &Coin,
) -> Result<Vec<Coin>, ContractError> {
    let total_supply = LP_TOKEN_SUPPLY.load(storage, &lp_token.denom)?;
    let (coin0, coin1) = COIN_BALANCES.load(storage, &lp_token.denom)?;

    if total_supply.is_zero() {
        Ok(vec![coin0, coin1])
    } else {
        Ok(vec![
            Coin {
                denom: coin0.denom,
                amount: coin0.amount.multiply_ratio(lp_token.amount, total_supply),
            },
            Coin {
                denom: coin1.denom,
                amount: coin1.amount.multiply_ratio(lp_token.amount, total_supply),
            },
        ])
    }
}
