use cosmwasm_std::{Addr, BlockInfo, Decimal, Deps, Env, Order, StdResult, Uint128};
use cw_paginate::{paginate_map_query, paginate_prefix_query, PaginationResponse};
use cw_storage_plus::Bound;
use mars_interest_rate::{
    get_scaled_debt_amount, get_scaled_liquidity_amount, get_underlying_debt_amount,
    get_underlying_liquidity_amount,
};
use mars_types::{
    address_provider::{self, MarsAddressType},
    keys::{UserId, UserIdKey},
    red_bank::{
        Collateral, ConfigResponse, Debt, Market, MarketV2Response,
        PaginatedUserCollateralResponse, UserCollateralResponse, UserDebtResponse,
        UserHealthStatus, UserPositionResponse,
    },
};

use crate::{
    error::{ContractError, ContractResult},
    health,
    state::{COLLATERALS, CONFIG, DEBTS, MARKETS, OWNER},
};

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let owner_state = OWNER.query(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: owner_state.owner,
        proposed_new_owner: owner_state.proposed,
        address_provider: config.address_provider.to_string(),
    })
}

pub fn query_market(deps: Deps, denom: String) -> StdResult<Option<Market>> {
    MARKETS.may_load(deps.storage, &denom)
}

pub fn query_market_v2(deps: Deps, env: Env, denom: String) -> StdResult<MarketV2Response> {
    let block_time = env.block.time.seconds();
    let market = MARKETS.load(deps.storage, &denom)?;

    let collateral_total_amount =
        get_underlying_liquidity_amount(market.collateral_total_scaled, &market, block_time)?;

    let debt_total_amount =
        get_underlying_debt_amount(market.debt_total_scaled, &market, block_time)?;

    let utilization_rate = if !collateral_total_amount.is_zero() {
        Decimal::from_ratio(debt_total_amount, collateral_total_amount)
    } else {
        Decimal::zero()
    };

    Ok(MarketV2Response {
        collateral_total_amount,
        debt_total_amount,
        utilization_rate,
        market,
    })
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

pub fn query_markets_v2(
    deps: Deps,
    env: Env,
    start_after: Option<String>,
    limit: Option<u32>,
) -> ContractResult<PaginationResponse<MarketV2Response>> {
    let block_time = env.block.time.seconds();
    let start = start_after.map(|denom| Bound::ExclusiveRaw(denom.into_bytes()));
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);

    paginate_map_query(&MARKETS, deps.storage, start, Some(limit), |_denom, market| {
        let collateral_total_amount =
            get_underlying_liquidity_amount(market.collateral_total_scaled, &market, block_time)?;
        let debt_total_amount =
            get_underlying_debt_amount(market.debt_total_scaled, &market, block_time)?;

        let utilization_rate = if !collateral_total_amount.is_zero() {
            Decimal::from_ratio(debt_total_amount, collateral_total_amount)
        } else {
            Decimal::zero()
        };

        Ok(MarketV2Response {
            debt_total_amount,
            collateral_total_amount,
            utilization_rate,
            market,
        })
    })
}

pub fn query_user_debt(
    deps: Deps,
    block: &BlockInfo,
    user_addr: Addr,
    denom: String,
) -> Result<UserDebtResponse, ContractError> {
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
) -> Result<Vec<UserDebtResponse>, ContractError> {
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
    account_id: Option<String>,
    denom: String,
) -> Result<UserCollateralResponse, ContractError> {
    let acc_id = account_id.unwrap_or("".to_string());

    let user_id = UserId::credit_manager(user_addr, acc_id);
    let user_id_key: UserIdKey = user_id.try_into()?;

    let Collateral {
        amount_scaled,
        enabled,
    } = COLLATERALS.may_load(deps.storage, (&user_id_key, &denom))?.unwrap_or_default();

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
    account_id: Option<String>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<UserCollateralResponse>, ContractError> {
    let res_v2: PaginationResponse<UserCollateralResponse> =
        query_user_collaterals_v2(deps, block, user_addr, account_id, start_after, limit)?;
    Ok(res_v2.data)
}

pub fn query_user_collaterals_v2(
    deps: Deps,
    block: &BlockInfo,
    user_addr: Addr,
    account_id: Option<String>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<PaginatedUserCollateralResponse, ContractError> {
    let block_time = block.time.seconds();

    let start = start_after.map(|denom| Bound::ExclusiveRaw(denom.into_bytes()));
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);

    let acc_id = account_id.unwrap_or("".to_string());

    let user_id: UserId = UserId::credit_manager(user_addr, acc_id);
    let user_id_key: UserIdKey = user_id.try_into()?;

    paginate_prefix_query(
        &COLLATERALS,
        deps.storage,
        &user_id_key,
        start,
        Some(limit),
        |denom, collateral| {
            let market = MARKETS.load(deps.storage, &denom)?;

            let amount_scaled = collateral.amount_scaled;
            let amount = get_underlying_liquidity_amount(amount_scaled, &market, block_time)?;

            Ok(UserCollateralResponse {
                denom: denom.to_string(),
                amount_scaled,
                amount,
                enabled: collateral.enabled,
            })
        },
    )
}

pub fn query_scaled_liquidity_amount(
    deps: Deps,
    env: Env,
    denom: String,
    amount: Uint128,
) -> Result<Uint128, ContractError> {
    let market = MARKETS.load(deps.storage, &denom)?;
    Ok(get_scaled_liquidity_amount(amount, &market, env.block.time.seconds())?)
}

pub fn query_scaled_debt_amount(
    deps: Deps,
    env: Env,
    denom: String,
    amount: Uint128,
) -> Result<Uint128, ContractError> {
    let market = MARKETS.load(deps.storage, &denom)?;
    Ok(get_scaled_debt_amount(amount, &market, env.block.time.seconds())?)
}

pub fn query_underlying_liquidity_amount(
    deps: Deps,
    env: Env,
    denom: String,
    amount_scaled: Uint128,
) -> Result<Uint128, ContractError> {
    let market = MARKETS.load(deps.storage, &denom)?;
    Ok(get_underlying_liquidity_amount(amount_scaled, &market, env.block.time.seconds())?)
}

pub fn query_underlying_debt_amount(
    deps: Deps,
    env: Env,
    denom: String,
    amount_scaled: Uint128,
) -> Result<Uint128, ContractError> {
    let market = MARKETS.load(deps.storage, &denom)?;
    Ok(get_underlying_debt_amount(amount_scaled, &market, env.block.time.seconds())?)
}

pub fn query_user_position(
    deps: Deps,
    env: Env,
    user_addr: Addr,
    account_id: Option<String>,
    liquidation_pricing: bool,
) -> Result<UserPositionResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let addresses = address_provider::helpers::query_contract_addrs(
        deps,
        &config.address_provider,
        vec![MarsAddressType::Oracle, MarsAddressType::Params],
    )?;
    let oracle_addr = &addresses[&MarsAddressType::Oracle];
    let params_addr = &addresses[&MarsAddressType::Params];

    let acc_id = account_id.unwrap_or("".to_string());
    let positions = health::get_user_positions_map(
        &deps,
        &env,
        &user_addr,
        &acc_id,
        oracle_addr,
        params_addr,
        liquidation_pricing,
    )?;
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
