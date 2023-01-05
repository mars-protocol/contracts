use cosmwasm_std::{Addr, BlockInfo, Deps, Env, Order, StdError, StdResult, Uint128};
use cw_storage_plus::Bound;
use mars_outpost::{
    address_provider::{self, MarsAddressType},
    error::MarsError,
    red_bank::{
        Collateral, ConfigResponse, Debt, Market, UncollateralizedLoanLimitResponse,
        UserCollateralResponse, UserDebtResponse, UserHealthStatus, UserPositionResponse,
    },
};

use crate::{
    health,
    interest_rates::{
        get_scaled_debt_amount, get_scaled_liquidity_amount, get_underlying_debt_amount,
        get_underlying_liquidity_amount,
    },
    state::{COLLATERALS, CONFIG, DEBTS, MARKETS, UNCOLLATERALIZED_LOAN_LIMITS},
};

const DEFAULT_LIMIT: u32 = 5;
const MAX_LIMIT: u32 = 10;

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: config.owner.to_string(),
        address_provider: config.address_provider.to_string(),
        close_factor: config.close_factor,
    })
}

pub fn query_market(deps: Deps, denom: String) -> StdResult<Market> {
    MARKETS
        .load(deps.storage, &denom)
        .map_err(|_| StdError::generic_err(format!("failed to load market for: {denom}")))
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

pub fn query_uncollateralized_loan_limit(
    deps: Deps,
    user_addr: Addr,
    denom: String,
) -> StdResult<UncollateralizedLoanLimitResponse> {
    let limit = UNCOLLATERALIZED_LOAN_LIMITS.may_load(deps.storage, (&user_addr, &denom))?;
    Ok(UncollateralizedLoanLimitResponse {
        denom,
        limit: limit.unwrap_or_else(Uint128::zero),
    })
}

pub fn query_uncollateralized_loan_limits(
    deps: Deps,
    user_addr: Addr,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<UncollateralizedLoanLimitResponse>> {
    let start = start_after.map(|denom| Bound::ExclusiveRaw(denom.into_bytes()));
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    UNCOLLATERALIZED_LOAN_LIMITS
        .prefix(&user_addr)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (denom, limit) = item?;
            Ok(UncollateralizedLoanLimitResponse {
                denom,
                limit,
            })
        })
        .collect()
}

pub fn query_user_debt(
    deps: Deps,
    block: &BlockInfo,
    user_addr: Addr,
    denom: String,
) -> StdResult<UserDebtResponse> {
    let Debt {
        amount_scaled,
        uncollateralized,
    } = DEBTS.may_load(deps.storage, (&user_addr, &denom))?.unwrap_or_default();

    let block_time = block.time.seconds();
    let market = MARKETS.load(deps.storage, &denom)?;
    let amount = get_underlying_debt_amount(amount_scaled, &market, block_time)?;

    Ok(UserDebtResponse {
        denom,
        amount_scaled,
        amount,
        uncollateralized,
    })
}

pub fn query_user_debts(
    deps: Deps,
    block: &BlockInfo,
    user_addr: Addr,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<UserDebtResponse>> {
    let block_time = block.time.seconds();

    let start = start_after.map(|denom| Bound::ExclusiveRaw(denom.into_bytes()));
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    DEBTS
        .prefix(&user_addr)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (denom, debt) = item?;

            let market = MARKETS.load(deps.storage, &denom)?;

            let amount_scaled = debt.amount_scaled;
            let amount = get_underlying_debt_amount(amount_scaled, &market, block_time)?;

            Ok(UserDebtResponse {
                denom,
                amount_scaled,
                amount,
                uncollateralized: debt.uncollateralized,
            })
        })
        .collect()
}

pub fn query_user_collateral(
    deps: Deps,
    block: &BlockInfo,
    user_addr: Addr,
    denom: String,
) -> StdResult<UserCollateralResponse> {
    let Collateral {
        amount_scaled,
        enabled,
    } = COLLATERALS.may_load(deps.storage, (&user_addr, &denom))?.unwrap_or_default();

    let block_time = block.time.seconds();
    let market = MARKETS.load(deps.storage, &denom)?;
    let amount = get_underlying_liquidity_amount(amount_scaled, &market, block_time)?;

    Ok(UserCollateralResponse {
        denom,
        amount_scaled,
        amount,
        enabled,
    })
}

pub fn query_user_collaterals(
    deps: Deps,
    block: &BlockInfo,
    user_addr: Addr,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<UserCollateralResponse>> {
    let block_time = block.time.seconds();

    let start = start_after.map(|denom| Bound::ExclusiveRaw(denom.into_bytes()));
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    COLLATERALS
        .prefix(&user_addr)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (denom, collateral) = item?;

            let market = MARKETS.load(deps.storage, &denom)?;

            let amount_scaled = collateral.amount_scaled;
            let amount = get_underlying_liquidity_amount(amount_scaled, &market, block_time)?;

            Ok(UserCollateralResponse {
                denom,
                amount_scaled,
                amount,
                enabled: collateral.enabled,
            })
        })
        .collect()
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
    user_addr: Addr,
) -> Result<UserPositionResponse, MarsError> {
    let config = CONFIG.load(deps.storage)?;
    let oracle_addr = address_provider::helpers::query_address(
        deps,
        &config.address_provider,
        MarsAddressType::Oracle,
    )?;

    let positions = health::get_user_positions_map(&deps, &env, &user_addr, &oracle_addr)?;
    let health = health::compute_position_health(&positions)?;

    let health_status = if let (Some(max_ltv_hf), Some(liq_threshold_hf)) =
        (health.max_ltv_health_factor, health.liquidation_health_factor)
    {
        UserHealthStatus::Borrowing {
            max_ltv_hf,
            liq_threshold_hf,
        }
    } else {
        UserHealthStatus::NotBorrowing
    };

    Ok(UserPositionResponse {
        total_enabled_collateral: health.total_collateral_value,
        total_collateralized_debt: health.total_debt_value,
        weighted_max_ltv_collateral: health.max_ltv_adjusted_collateral,
        weighted_liquidation_threshold_collateral: health.liquidation_threshold_adjusted_collateral,
        health_status,
    })
}
