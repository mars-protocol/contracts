use cosmwasm_std::{Addr, Deps, Env, Order, StdError, StdResult, Uint128};
use cw_storage_plus::Bound;

use mars_outpost::address_provider::{self, MarsContract};
use mars_outpost::error::MarsError;
use mars_outpost::red_bank::{
    ConfigResponse, Market, UserAssetCollateralResponse, UserAssetDebtResponse,
    UserCollateralResponse, UserDebtResponse, UserPositionResponse,
};

use crate::accounts::get_user_position;
use crate::helpers::get_bit;
use crate::interest_rates::{
    get_scaled_debt_amount, get_scaled_liquidity_amount, get_underlying_debt_amount,
    get_underlying_liquidity_amount,
};
use crate::state::{
    CONFIG, DEBTS, GLOBAL_STATE, MARKETS, MARKET_DENOMS_BY_MA_TOKEN, UNCOLLATERALIZED_LOAN_LIMITS,
    USERS,
};

const DEFAULT_LIMIT: u32 = 5;
const MAX_LIMIT: u32 = 10;

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    let money_market = GLOBAL_STATE.load(deps.storage)?;

    Ok(ConfigResponse {
        owner: config.owner,
        address_provider_address: config.address_provider_address,
        ma_token_code_id: config.ma_token_code_id,
        market_count: money_market.market_count,
        close_factor: config.close_factor,
    })
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

pub fn query_user_debt(deps: Deps, env: Env, user_address: Addr) -> StdResult<UserDebtResponse> {
    let user = USERS.may_load(deps.storage, &user_address)?.unwrap_or_default();

    let debts: StdResult<Vec<_>> = MARKETS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (denom, market) = item?;

            let is_borrowing_asset = get_bit(user.borrowed_assets, market.index)?;
            let (amount_scaled, amount) = if is_borrowing_asset {
                let debt = DEBTS.load(deps.storage, (&denom, &user_address))?;
                let amount_scaled = debt.amount_scaled;
                let amount =
                    get_underlying_debt_amount(amount_scaled, &market, env.block.time.seconds())?;
                (amount_scaled, amount)
            } else {
                (Uint128::zero(), Uint128::zero())
            };

            Ok(UserAssetDebtResponse {
                denom,
                amount_scaled,
                amount,
            })
        })
        .collect();

    Ok(UserDebtResponse {
        debts: debts?,
    })
}

pub fn query_user_asset_debt(
    deps: Deps,
    env: Env,
    user_address: Addr,
    denom: String,
) -> StdResult<UserAssetDebtResponse> {
    let market = MARKETS.load(deps.storage, &denom)?;

    let (amount_scaled, amount) = match DEBTS.may_load(deps.storage, (&denom, &user_address))? {
        Some(debt) => {
            let amount_scaled = debt.amount_scaled;
            let amount =
                get_underlying_debt_amount(amount_scaled, &market, env.block.time.seconds())?;
            (amount_scaled, amount)
        }

        None => (Uint128::zero(), Uint128::zero()),
    };

    Ok(UserAssetDebtResponse {
        denom,
        amount_scaled,
        amount,
    })
}

pub fn query_user_collateral(deps: Deps, address: Addr) -> StdResult<UserCollateralResponse> {
    let user = USERS.may_load(deps.storage, &address)?.unwrap_or_default();

    let collateral: StdResult<Vec<_>> = MARKETS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (denom, market) = item?;
            Ok(UserAssetCollateralResponse {
                denom,
                enabled: get_bit(user.collateral_assets, market.index)?,
            })
        })
        .collect();

    Ok(UserCollateralResponse {
        collateral: collateral?,
    })
}

pub fn query_uncollateralized_loan_limit(
    deps: Deps,
    user_address: Addr,
    denom: String,
) -> StdResult<Uint128> {
    let uncollateralized_loan_limit =
        UNCOLLATERALIZED_LOAN_LIMITS.load(deps.storage, (&denom, &user_address));

    match uncollateralized_loan_limit {
        Ok(limit) => Ok(limit),
        Err(_) => Err(StdError::not_found(format!(
            "No uncollateralized loan approved for user_address: {} on asset: {}",
            user_address, denom
        ))),
    }
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
    ma_token_address: String,
    amount_scaled: Uint128,
) -> StdResult<Uint128> {
    let ma_token_address = deps.api.addr_validate(&ma_token_address)?;
    let denom = MARKET_DENOMS_BY_MA_TOKEN.load(deps.storage, &ma_token_address)?;
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
    let global_state = GLOBAL_STATE.load(deps.storage)?;
    let user = USERS.may_load(deps.storage, &address)?.unwrap_or_default();
    let oracle_address = address_provider::helpers::query_address(
        deps,
        &config.address_provider_address,
        MarsContract::Oracle,
    )?;
    let user_position = get_user_position(
        deps,
        env.block.time.seconds(),
        &address,
        &oracle_address,
        &user,
        global_state.market_count,
    )?;

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
