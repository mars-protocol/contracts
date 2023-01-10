use cosmwasm_std::{Coin, Deps, StdResult, Storage, Uint128};

use crate::{
    contract::STARTING_LP_POOL_TOKENS,
    error::ContractError,
    state::{COIN_BALANCES, COIN_CONFIG, LP_TOKEN_SUPPLY, ORACLE},
};

pub fn estimate_provide_liquidity(
    deps: &Deps,
    lp_token_out: &str,
    coins_in: Vec<Coin>,
) -> Result<Uint128, ContractError> {
    let total_supply = LP_TOKEN_SUPPLY.load(deps.storage, lp_token_out).unwrap_or(Uint128::zero());

    let lp_tokens_estimate = if total_supply.is_zero() {
        STARTING_LP_POOL_TOKENS
    } else {
        let coins = coins_in
            .iter()
            .map(|c| {
                let balance = COIN_BALANCES.load(deps.storage, (lp_token_out, &c.denom))?;
                Ok(Coin {
                    denom: c.denom.clone(),
                    amount: balance,
                })
            })
            .collect::<StdResult<Vec<_>>>()?;
        let oracle = ORACLE.load(deps.storage)?;
        let total_underlying_value = oracle.query_total_value(&deps.querier, &coins)?;
        let given_value = oracle.query_total_value(&deps.querier, &coins_in)?;
        total_supply.checked_multiply_ratio(given_value, total_underlying_value)?
    };
    Ok(lp_tokens_estimate)
}

pub fn estimate_withdraw_liquidity(
    storage: &dyn Storage,
    lp_token: &Coin,
) -> Result<Vec<Coin>, ContractError> {
    let total_supply = LP_TOKEN_SUPPLY.load(storage, &lp_token.denom)?;
    if total_supply.is_zero() {
        return Ok(vec![]);
    }

    let underlying = COIN_CONFIG.load(storage, &lp_token.denom)?;
    let estimate = underlying
        .into_iter()
        .map(|denom| {
            let balance = COIN_BALANCES.load(storage, (&lp_token.denom, &denom))?;
            Ok(Coin {
                denom,
                amount: balance.multiply_ratio(lp_token.amount, total_supply),
            })
        })
        .collect::<StdResult<Vec<_>>>()?
        .into_iter()
        .filter(|c| !c.amount.is_zero())
        .collect::<Vec<_>>();

    Ok(estimate)
}
