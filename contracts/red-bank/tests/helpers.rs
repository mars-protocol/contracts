#![allow(dead_code)]

use cosmwasm_schema::serde;
use cosmwasm_std::testing::{MockApi, MockStorage};
use cosmwasm_std::{from_binary, Addr, Coin, Decimal, Deps, DepsMut, Event, OwnedDeps, Uint128};

use mars_outpost::red_bank::{
    Collateral, CreateOrUpdateConfig, Debt, InstantiateMsg, Market, QueryMsg,
};
use mars_testing::{mock_dependencies, mock_env, mock_info, MarsMockQuerier, MockEnvParams};

use mars_red_bank::contract::{instantiate, query};
use mars_red_bank::interest_rates::{
    calculate_applied_linear_interest_rate, compute_scaled_amount, compute_underlying_amount,
    ScalingOperation,
};
use mars_red_bank::state::{COLLATERALS, DEBTS, MARKETS};

pub fn set_collateral(
    deps: DepsMut,
    user_addr: &Addr,
    denom: &str,
    amount_scaled: Uint128,
    enabled: bool,
) {
    let collateral = Collateral {
        amount_scaled,
        enabled,
    };
    COLLATERALS.save(deps.storage, (user_addr, denom), &collateral).unwrap();
}

pub fn unset_collateral(deps: DepsMut, user_addr: &Addr, denom: &str) {
    COLLATERALS.remove(deps.storage, (user_addr, denom));
}

pub fn set_debt(
    deps: DepsMut,
    user_addr: &Addr,
    denom: &str,
    amount_scaled: impl Into<Uint128>,
    uncollateralized: bool,
) {
    let debt = Debt {
        amount_scaled: amount_scaled.into(),
        uncollateralized,
    };
    DEBTS.save(deps.storage, (user_addr, denom), &debt).unwrap();
}

/// Find if a user has a debt position in the specified asset
pub fn has_debt_position(deps: Deps, user_addr: &Addr, denom: &str) -> bool {
    DEBTS.may_load(deps.storage, (user_addr, denom)).unwrap().is_some()
}

/// Find if a user has a collateral position in the specified asset, regardless of whether enabled
pub fn has_collateral_position(deps: Deps, user_addr: &Addr, denom: &str) -> bool {
    COLLATERALS.may_load(deps.storage, (user_addr, denom)).unwrap().is_some()
}

/// Find whether a user has a collateral position AND has it enabled in the specified asset
pub fn has_collateral_enabled(deps: Deps, user_addr: &Addr, denom: &str) -> bool {
    COLLATERALS
        .may_load(deps.storage, (user_addr, denom))
        .unwrap()
        .map(|collateral| collateral.enabled)
        .unwrap_or(false)
}

pub fn th_setup(contract_balances: &[Coin]) -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier> {
    let mut deps = mock_dependencies(contract_balances);
    let env = mock_env(MockEnvParams::default());
    let info = mock_info("owner");
    let config = CreateOrUpdateConfig {
        address_provider: Some("address_provider".to_string()),
        close_factor: Some(Decimal::from_ratio(1u128, 2u128)),
    };
    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        config,
    };
    instantiate(deps.as_mut(), env, info, msg).unwrap();

    deps.querier.set_oracle_price("uusd", Decimal::one());

    deps
}

pub fn th_query<T: serde::de::DeserializeOwned>(deps: Deps, msg: QueryMsg) -> T {
    from_binary(&query(deps, mock_env(MockEnvParams::default()), msg).unwrap()).unwrap()
}

pub fn th_init_market(deps: DepsMut, denom: &str, market: &Market) -> Market {
    let new_market = Market {
        denom: denom.to_string(),
        ..market.clone()
    };

    MARKETS.save(deps.storage, denom, &new_market).unwrap();

    new_market
}

#[derive(Default, Debug)]
pub struct TestInterestResults {
    pub borrow_index: Decimal,
    pub liquidity_index: Decimal,
    pub borrow_rate: Decimal,
    pub liquidity_rate: Decimal,
    pub protocol_rewards_to_distribute: Uint128,
    pub less_debt_scaled: Uint128,
}

pub fn th_build_interests_updated_event(denom: &str, ir: &TestInterestResults) -> Event {
    Event::new("interests_updated")
        .add_attribute("denom", denom)
        .add_attribute("borrow_index", ir.borrow_index.to_string())
        .add_attribute("liquidity_index", ir.liquidity_index.to_string())
        .add_attribute("borrow_rate", ir.borrow_rate.to_string())
        .add_attribute("liquidity_rate", ir.liquidity_rate.to_string())
}

/// Deltas to be using in expected indices/rates results
#[derive(Default, Debug)]
pub struct TestUtilizationDeltaInfo {
    pub less_liquidity: Uint128,
    pub more_debt: Uint128,
    pub less_debt: Uint128,
    /// Used when passing less debt to compute deltas in the actual scaled amount
    pub user_current_debt_scaled: Uint128,
}

/// Takes a market before an action (ie: a borrow) among some test parameters
/// used in that action and computes the expected indices and rates after that action.
pub fn th_get_expected_indices_and_rates(
    market: &Market,
    block_time: u64,
    initial_liquidity: Uint128,
    delta_info: TestUtilizationDeltaInfo,
) -> TestInterestResults {
    if !delta_info.more_debt.is_zero() && !delta_info.less_debt.is_zero() {
        panic!("Cannot have more debt and less debt at the same time");
    }

    if !delta_info.less_debt.is_zero() && delta_info.user_current_debt_scaled.is_zero() {
        panic!("Cannot have less debt with 0 current user debt scaled");
    }

    let expected_indices = th_get_expected_indices(market, block_time);

    let expected_protocol_rewards_to_distribute =
        th_get_expected_protocol_rewards(market, &expected_indices);

    // When borrowing, new computed index is used for scaled amount
    let more_debt_scaled = compute_scaled_amount(
        delta_info.more_debt,
        expected_indices.borrow,
        ScalingOperation::Ceil,
    )
    .unwrap();

    // When repaying, new computed index is used to get current debt and deduct amount
    let less_debt_scaled = if !delta_info.less_debt.is_zero() {
        let user_current_debt = compute_underlying_amount(
            delta_info.user_current_debt_scaled,
            expected_indices.borrow,
            ScalingOperation::Ceil,
        )
        .unwrap();

        let user_new_debt = if delta_info.less_debt >= user_current_debt {
            Uint128::zero()
        } else {
            user_current_debt - delta_info.less_debt
        };

        let user_new_debt_scaled =
            compute_scaled_amount(user_new_debt, expected_indices.borrow, ScalingOperation::Ceil)
                .unwrap();

        delta_info.user_current_debt_scaled - user_new_debt_scaled
    } else {
        Uint128::zero()
    };

    // NOTE: Don't panic here so that the total repay of debt can be simulated
    // when less debt is greater than outstanding debt
    let new_debt_total_scaled = if (market.debt_total_scaled + more_debt_scaled) > less_debt_scaled
    {
        market.debt_total_scaled + more_debt_scaled - less_debt_scaled
    } else {
        Uint128::zero()
    };
    let dec_debt_total = compute_underlying_amount(
        new_debt_total_scaled,
        expected_indices.borrow,
        ScalingOperation::Ceil,
    )
    .unwrap();
    let contract_current_balance = initial_liquidity;
    let liquidity_taken = delta_info.less_liquidity;
    let dec_liquidity_total = contract_current_balance - liquidity_taken;
    let expected_utilization_rate =
        Decimal::from_ratio(dec_debt_total, dec_liquidity_total + dec_debt_total);

    // interest rates (make a copy and update those values to get the expeted irs)
    let mut market_copy = market.clone();
    market_copy.update_interest_rates(expected_utilization_rate).unwrap();

    TestInterestResults {
        borrow_index: expected_indices.borrow,
        liquidity_index: expected_indices.liquidity,
        borrow_rate: market_copy.borrow_rate,
        liquidity_rate: market_copy.liquidity_rate,
        protocol_rewards_to_distribute: expected_protocol_rewards_to_distribute,
        less_debt_scaled,
    }
}

/// Compute protocol income to be distributed (using values up to the instant
/// before the contract call is made)
pub fn th_get_expected_protocol_rewards(
    market: &Market,
    expected_indices: &TestExpectedIndices,
) -> Uint128 {
    let previous_borrow_index = market.borrow_index;
    let previous_debt_total = compute_underlying_amount(
        market.debt_total_scaled,
        previous_borrow_index,
        ScalingOperation::Ceil,
    )
    .unwrap();
    let current_debt_total = compute_underlying_amount(
        market.debt_total_scaled,
        expected_indices.borrow,
        ScalingOperation::Ceil,
    )
    .unwrap();
    let interest_accrued = if current_debt_total > previous_debt_total {
        current_debt_total - previous_debt_total
    } else {
        Uint128::zero()
    };
    interest_accrued * market.reserve_factor
}

/// Expected results for applying accumulated interest
pub struct TestExpectedIndices {
    pub liquidity: Decimal,
    pub borrow: Decimal,
}

pub fn th_get_expected_indices(market: &Market, block_time: u64) -> TestExpectedIndices {
    let seconds_elapsed = block_time - market.indexes_last_updated;
    // market indices
    let expected_liquidity_index = calculate_applied_linear_interest_rate(
        market.liquidity_index,
        market.liquidity_rate,
        seconds_elapsed,
    )
    .unwrap();

    let expected_borrow_index = calculate_applied_linear_interest_rate(
        market.borrow_index,
        market.borrow_rate,
        seconds_elapsed,
    )
    .unwrap();

    TestExpectedIndices {
        liquidity: expected_liquidity_index,
        borrow: expected_borrow_index,
    }
}
