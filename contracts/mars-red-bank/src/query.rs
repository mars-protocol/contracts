use cosmwasm_std::{Addr, Deps, Env, Order, StdError, StdResult, Uint128};
use cw_storage_plus::Bound;

use mars_outpost::address_provider::{self, MarsContract};
use mars_outpost::error::MarsError;
use mars_outpost::red_bank::{CoinScaled, Config, Market, UserPositionResponse};

use crate::accounts::get_user_position;
use crate::interest_rates::{
    get_scaled_debt_amount, get_scaled_liquidity_amount, get_underlying_debt_amount,
    get_underlying_liquidity_amount,
};
use crate::state::{COLLATERALS, CONFIG, DEBTS, MARKETS, UNCOLLATERALIZED_LOAN_LIMITS};

const DEFAULT_LIMIT: u32 = 5;
const MAX_LIMIT: u32 = 10;

pub fn query_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

pub fn query_market(deps: Deps, denom: String) -> StdResult<Market> {
    MARKETS
        .load(deps.storage, &denom)
        .map_err(|_| StdError::generic_err(format!("failed to load market for: {}", denom)))
}

pub fn query_markets(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<Market>> {
    let start = start_after.map(|denom| Bound::ExclusiveRaw(denom.into_bytes()));
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    MARKETS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, market) = item?;
            Ok(market)
        })
        .collect()
}

pub fn query_user_debt(deps: Deps, env: Env, user_address: Addr) -> StdResult<Vec<CoinScaled>> {
    DEBTS
        .prefix(&user_address)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (denom, debt) = item?;

            let market = MARKETS.load(deps.storage, &denom)?;
            let amount_scaled = debt.amount_scaled;
            let amount =
                get_underlying_debt_amount(amount_scaled, &market, env.block.time.seconds())?;

            Ok(CoinScaled {
                denom,
                amount_scaled,
                amount,
            })
        })
        .collect()
}

pub fn query_user_asset_debt(
    deps: Deps,
    env: Env,
    user_address: Addr,
    denom: String,
) -> StdResult<CoinScaled> {
    let market = MARKETS.load(deps.storage, &denom)?;

    let amount_scaled = DEBTS
        .may_load(deps.storage, (&user_address, &denom))?
        .map(|debt| debt.amount_scaled)
        .unwrap_or_else(Uint128::zero);

    let amount = get_underlying_debt_amount(amount_scaled, &market, env.block.time.seconds())?;

    Ok(CoinScaled {
        denom,
        amount_scaled,
        amount,
    })
}

pub fn query_user_collateral(
    deps: Deps,
    env: Env,
    user_address: Addr,
) -> StdResult<Vec<CoinScaled>> {
    COLLATERALS
        .prefix(&user_address)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (denom, amount_scaled) = item?;

            let market = MARKETS.load(deps.storage, &denom)?;
            let amount =
                get_underlying_debt_amount(amount_scaled, &market, env.block.time.seconds())?;

            Ok(CoinScaled {
                denom,
                amount_scaled,
                amount,
            })
        })
        .collect()
}

pub fn query_user_asset_collateral(
    deps: Deps,
    env: Env,
    user_address: Addr,
    denom: String,
) -> StdResult<CoinScaled> {
    let market = MARKETS.load(deps.storage, &denom)?;

    let amount_scaled =
        COLLATERALS.may_load(deps.storage, (&user_address, &denom))?.unwrap_or_else(Uint128::zero);

    let amount = get_underlying_liquidity_amount(amount_scaled, &market, env.block.time.seconds())?;

    Ok(CoinScaled {
        denom,
        amount_scaled,
        amount,
    })
}

pub fn query_uncollateralized_loan_limit(
    deps: Deps,
    user_address: Addr,
    denom: String,
) -> StdResult<Uint128> {
    UNCOLLATERALIZED_LOAN_LIMITS.may_load(deps.storage, (&user_address, &denom))?.ok_or_else(|| {
        StdError::not_found(format!(
            "No uncollateralized loan approved for user_address: {} on asset: {}",
            user_address, denom
        ))
    })
}

pub fn query_scaled_liquidity_amount(
    deps: Deps,
    env: Env,
    denom: String,
    amount: Uint128,
) -> StdResult<Uint128> {
    let market = MARKETS.load(deps.storage, &denom)?;
    get_scaled_liquidity_amount(amount, &market, env.block.time.seconds())
}

pub fn query_scaled_debt_amount(
    deps: Deps,
    env: Env,
    denom: String,
    amount: Uint128,
) -> StdResult<Uint128> {
    let market = MARKETS.load(deps.storage, &denom)?;
    get_scaled_debt_amount(amount, &market, env.block.time.seconds())
}

pub fn query_underlying_liquidity_amount(
    deps: Deps,
    env: Env,
    denom: String,
    amount_scaled: Uint128,
) -> StdResult<Uint128> {
    let market = MARKETS.load(deps.storage, &denom)?;
    get_underlying_liquidity_amount(amount_scaled, &market, env.block.time.seconds())
}

pub fn query_underlying_debt_amount(
    deps: Deps,
    env: Env,
    denom: String,
    amount_scaled: Uint128,
) -> StdResult<Uint128> {
    let market = MARKETS.load(deps.storage, &denom)?;
    get_underlying_debt_amount(amount_scaled, &market, env.block.time.seconds())
}

pub fn query_user_position(
    deps: Deps,
    env: Env,
    address: Addr,
) -> Result<UserPositionResponse, MarsError> {
    let config = CONFIG.load(deps.storage)?;
    let oracle_address = address_provider::helpers::query_address(
        deps,
        &config.address_provider_address,
        MarsContract::Oracle,
    )?;
    let user_position =
        get_user_position(deps, env.block.time.seconds(), &address, &oracle_address)?;

    Ok(UserPositionResponse {
        total_collateral_in_base_asset: user_position.total_collateral_in_base_asset,
        total_debt_in_base_asset: user_position.total_debt_in_base_asset,
        total_collateralized_debt_in_base_asset: user_position
            .total_collateralized_debt_in_base_asset,
        max_debt_in_base_asset: user_position.max_debt_in_base_asset,
        weighted_liquidation_threshold_in_base_asset: user_position
            .weighted_liquidation_threshold_in_base_asset,
        health_status: user_position.health_status,
    })
}
