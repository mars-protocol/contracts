use cosmwasm_std::testing::{MockApi, MockStorage};
use cosmwasm_std::{Coin, Decimal, DepsMut, Event, OwnedDeps, StdResult, Uint128};

use mars_outpost::asset::Asset;
use mars_outpost::red_bank::interest_rate_models::update_market_interest_rates_with_model;
use mars_outpost::red_bank::msg::{CreateOrUpdateConfig, InstantiateMsg};
use mars_outpost::red_bank::{GlobalState, Market};

use mars_testing::{
    mock_dependencies, mock_env, mock_env_at_block_time, mock_info, MarsMockQuerier, MockEnvParams,
};

use crate::contract::instantiate;
use crate::interest_rates::{
    calculate_applied_linear_interest_rate, compute_scaled_amount, compute_underlying_amount,
    ScalingOperation,
};
use crate::state::{
    GLOBAL_STATE, MARKETS, MARKET_REFERENCES_BY_INDEX, MARKET_REFERENCES_BY_MA_TOKEN,
};

pub(super) fn th_setup(
    contract_balances: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier> {
    let mut deps = mock_dependencies(contract_balances);
    let env = mock_env(MockEnvParams::default());
    let info = mock_info("owner");
    let config = CreateOrUpdateConfig {
        owner: Some("owner".to_string()),
        address_provider_address: Some("address_provider".to_string()),
        ma_token_code_id: Some(1u64),
        close_factor: Some(Decimal::from_ratio(1u128, 2u128)),
        base_asset: Some(Asset::Native {
            denom: "uusd".to_string(),
        }),
    };
    let msg = InstantiateMsg {
        config,
    };
    instantiate(deps.as_mut(), env, info, msg).unwrap();

    let asset = Asset::Native {
        denom: "uusd".to_string(),
    };
    deps.querier.set_oracle_price(asset.get_reference(), Decimal::one());

    deps
}

pub(super) fn th_init_market(deps: DepsMut, key: &[u8], market: &Market) -> Market {
    let mut index = 0;

    GLOBAL_STATE
        .update(deps.storage, |mut mm: GlobalState| -> StdResult<GlobalState> {
            index = mm.market_count;
            mm.market_count += 1;
            Ok(mm)
        })
        .unwrap();

    let new_market = Market {
        index,
        ..market.clone()
    };

    MARKETS.save(deps.storage, key, &new_market).unwrap();

    MARKET_REFERENCES_BY_INDEX.save(deps.storage, index, &key.to_vec()).unwrap();

    MARKET_REFERENCES_BY_MA_TOKEN
        .save(deps.storage, &new_market.ma_token_address, &key.to_vec())
        .unwrap();

    new_market
}

#[derive(Default, Debug)]
pub(super) struct TestInterestResults {
    pub borrow_index: Decimal,
    pub liquidity_index: Decimal,
    pub borrow_rate: Decimal,
    pub liquidity_rate: Decimal,
    pub protocol_rewards_to_distribute: Uint128,
    pub less_debt_scaled: Uint128,
}

pub(super) fn th_build_interests_updated_event(label: &str, ir: &TestInterestResults) -> Event {
    Event::new("interests_updated")
        .add_attribute("asset", label)
        .add_attribute("borrow_index", ir.borrow_index.to_string())
        .add_attribute("liquidity_index", ir.liquidity_index.to_string())
        .add_attribute("borrow_rate", ir.borrow_rate.to_string())
        .add_attribute("liquidity_rate", ir.liquidity_rate.to_string())
}

/// Deltas to be using in expected indices/rates results
#[derive(Default, Debug)]
pub(super) struct TestUtilizationDeltaInfo {
    pub less_liquidity: Uint128,
    pub more_debt: Uint128,
    pub less_debt: Uint128,
    /// Used when passing less debt to compute deltas in the actual scaled amount
    pub user_current_debt_scaled: Uint128,
}

/// Takes a market before an action (ie: a borrow) among some test parameters
/// used in that action and computes the expected indices and rates after that action.
pub(super) fn th_get_expected_indices_and_rates(
    market: &Market,
    block_time: u64,
    initial_liquidity: Uint128,
    delta_info: TestUtilizationDeltaInfo,
) -> TestInterestResults {
    if delta_info.more_debt > Uint128::zero() && delta_info.less_debt > Uint128::zero() {
        panic!("Cannot have more debt and less debt at the same time");
    }

    if delta_info.less_debt > Uint128::zero()
        && delta_info.user_current_debt_scaled == Uint128::zero()
    {
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
    let less_debt_scaled = if delta_info.less_debt > Uint128::zero() {
        let user_current_debt = compute_underlying_amount(
            delta_info.user_current_debt_scaled,
            expected_indices.borrow,
            ScalingOperation::Ceil,
        )
        .unwrap();

        let user_new_debt = if Uint128::from(delta_info.less_debt) >= user_current_debt {
            Uint128::zero()
        } else {
            user_current_debt - Uint128::from(delta_info.less_debt)
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
    let contract_current_balance = Uint128::from(initial_liquidity);
    let liquidity_taken = Uint128::from(delta_info.less_liquidity);
    let dec_liquidity_total = contract_current_balance - liquidity_taken;
    let expected_utilization_rate =
        Decimal::from_ratio(dec_debt_total, dec_liquidity_total + dec_debt_total);

    // interest rates (make a copy and update those values to get the expeted irs)
    let mut market_copy = market.clone();
    update_market_interest_rates_with_model(
        &mock_env_at_block_time(block_time),
        &mut market_copy,
        expected_utilization_rate,
    )
    .unwrap();

    TestInterestResults {
        borrow_index: expected_indices.borrow,
        liquidity_index: expected_indices.liquidity,
        borrow_rate: market_copy.borrow_rate,
        liquidity_rate: market_copy.liquidity_rate,
        protocol_rewards_to_distribute: expected_protocol_rewards_to_distribute,
        less_debt_scaled: less_debt_scaled,
    }
}

/// Compute protocol income to be distributed (using values up to the instant
/// before the contract call is made)
pub(super) fn th_get_expected_protocol_rewards(
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
pub(super) struct TestExpectedIndices {
    pub liquidity: Decimal,
    pub borrow: Decimal,
}

pub(super) fn th_get_expected_indices(market: &Market, block_time: u64) -> TestExpectedIndices {
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
