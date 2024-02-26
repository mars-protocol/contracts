#![allow(dead_code)]

use std::{collections::HashMap, fmt::Display, str::FromStr};

use anyhow::Result as AnyResult;
use cosmwasm_schema::serde;
use cosmwasm_std::{
    from_json,
    testing::{MockApi, MockStorage},
    Addr, Coin, Decimal, Deps, DepsMut, Event, OwnedDeps, Uint128,
};
use cw_multi_test::AppResponse;
use mars_interest_rate::{
    calculate_applied_linear_interest_rate, compute_scaled_amount, compute_underlying_amount,
    ScalingOperation,
};
use mars_red_bank::{
    contract::{instantiate, query},
    error::ContractError,
    state::{COLLATERALS, DEBTS, MARKETS},
};
use mars_testing::{mock_dependencies, mock_env, mock_info, MarsMockQuerier, MockEnvParams};
use mars_types::{
    keys::{UserId, UserIdKey},
    params::{AssetParams, CmSettings, LiquidationBonus, RedBankSettings},
    red_bank::{
        Collateral, CreateOrUpdateConfig, Debt, InitOrUpdateAssetParams, InstantiateMsg,
        InterestRateModel, Market, QueryMsg, UserCollateralResponse, UserDebtResponse,
        UserHealthStatus, UserPositionResponse,
    },
};

pub fn set_collateral(
    deps: DepsMut,
    user_addr: &Addr,
    denom: &str,
    amount_scaled: Uint128,
    enabled: bool,
) {
    let user_id = UserId::credit_manager(user_addr.clone(), "".to_string());
    let user_id_key: UserIdKey = user_id.try_into().unwrap();
    let collateral = Collateral {
        amount_scaled,
        enabled,
    };
    COLLATERALS.save(deps.storage, (&user_id_key, denom), &collateral).unwrap();
}

pub fn unset_collateral(deps: DepsMut, user_addr: &Addr, denom: &str) {
    let user_id = UserId::credit_manager(user_addr.clone(), "".to_string());
    let user_id_key: UserIdKey = user_id.try_into().unwrap();
    COLLATERALS.remove(deps.storage, (&user_id_key, denom));
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
    let user_id = UserId::credit_manager(user_addr.clone(), "".to_string());
    let user_id_key: UserIdKey = user_id.try_into().unwrap();
    COLLATERALS.may_load(deps.storage, (&user_id_key, denom)).unwrap().is_some()
}

/// Find whether a user has a collateral position AND has it enabled in the specified asset
pub fn has_collateral_enabled(deps: Deps, user_addr: &Addr, denom: &str) -> bool {
    let user_id = UserId::credit_manager(user_addr.clone(), "".to_string());
    let user_id_key: UserIdKey = user_id.try_into().unwrap();
    COLLATERALS
        .may_load(deps.storage, (&user_id_key, denom))
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
    };
    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        config,
    };
    instantiate(deps.as_mut(), env, info, msg).unwrap();

    deps.querier.set_oracle_price("uusd", Decimal::one());

    deps.querier.set_target_health_factor(Decimal::from_ratio(12u128, 10u128));

    deps
}

pub fn th_query<T: serde::de::DeserializeOwned>(deps: Deps, msg: QueryMsg) -> T {
    from_json(query(deps, mock_env(MockEnvParams::default()), msg).unwrap()).unwrap()
}

pub fn th_init_market(deps: DepsMut, denom: &str, market: &Market) -> Market {
    let new_market = Market {
        denom: denom.to_string(),
        ..market.clone()
    };

    MARKETS.save(deps.storage, denom, &new_market).unwrap();

    new_market
}

pub fn th_default_asset_params() -> AssetParams {
    AssetParams {
        denom: "todo".to_string(),
        credit_manager: CmSettings {
            whitelisted: false,
            hls: None,
        },
        red_bank: RedBankSettings {
            deposit_enabled: true,
            borrow_enabled: true,
        },
        max_loan_to_value: Decimal::zero(),
        liquidation_threshold: Decimal::one(),
        liquidation_bonus: LiquidationBonus {
            starting_lb: Decimal::percent(0u64),
            slope: Decimal::one(),
            min_lb: Decimal::percent(0u64),
            max_lb: Decimal::percent(5u64),
        },
        protocol_liquidation_fee: Decimal::percent(2u64),
        deposit_cap: Uint128::MAX,
    }
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
    let more_debt_scaled = th_get_scaled_debt_amount(delta_info.more_debt, expected_indices.borrow);

    // When repaying, new computed index is used to get current debt and deduct amount
    let less_debt_scaled = if !delta_info.less_debt.is_zero() {
        let user_current_debt = th_get_underlying_debt_amount(
            delta_info.user_current_debt_scaled,
            expected_indices.borrow,
        );

        let user_new_debt =
            user_current_debt.checked_sub(delta_info.less_debt).unwrap_or(Uint128::zero());

        let user_new_debt_scaled =
            th_get_scaled_debt_amount(user_new_debt, expected_indices.borrow);

        delta_info.user_current_debt_scaled - user_new_debt_scaled
    } else {
        Uint128::zero()
    };

    // NOTE: Don't panic here so that the total repay of debt can be simulated
    // when less debt is greater than outstanding debt
    let new_debt_total_scaled = (market.debt_total_scaled + more_debt_scaled)
        .checked_sub(less_debt_scaled)
        .unwrap_or(Uint128::zero());
    let debt_total = th_get_underlying_debt_amount(new_debt_total_scaled, expected_indices.borrow);

    let total_collateral = th_get_underlying_liquidity_amount(
        market.collateral_total_scaled,
        expected_indices.liquidity,
    );

    // Total collateral increased by accured protocol rewards
    let total_collateral = total_collateral + expected_protocol_rewards_to_distribute;
    let expected_utilization_rate = if !total_collateral.is_zero() {
        Decimal::from_ratio(debt_total, total_collateral)
    } else {
        Decimal::zero()
    };

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
    let previous_debt_total =
        th_get_underlying_debt_amount(market.debt_total_scaled, previous_borrow_index);
    let current_debt_total =
        th_get_underlying_debt_amount(market.debt_total_scaled, expected_indices.borrow);
    let interest_accrued =
        current_debt_total.checked_sub(previous_debt_total).unwrap_or(Uint128::zero());
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

pub fn th_get_scaled_liquidity_amount(amount: Uint128, liquidity_index: Decimal) -> Uint128 {
    compute_scaled_amount(amount, liquidity_index, ScalingOperation::Truncate).unwrap()
}

pub fn th_get_scaled_debt_amount(amount: Uint128, borrow_index: Decimal) -> Uint128 {
    compute_scaled_amount(amount, borrow_index, ScalingOperation::Ceil).unwrap()
}

pub fn th_get_underlying_liquidity_amount(
    amount_scaled: Uint128,
    liquidity_index: Decimal,
) -> Uint128 {
    compute_underlying_amount(amount_scaled, liquidity_index, ScalingOperation::Truncate).unwrap()
}

pub fn th_get_underlying_debt_amount(amount_scaled: Uint128, borrow_index: Decimal) -> Uint128 {
    compute_underlying_amount(amount_scaled, borrow_index, ScalingOperation::Ceil).unwrap()
}

pub fn liq_threshold_hf(position: &UserPositionResponse) -> Decimal {
    match position.health_status {
        UserHealthStatus::Borrowing {
            liq_threshold_hf,
            ..
        } => liq_threshold_hf,
        _ => panic!("User is not borrowing"),
    }
}

// Merge collaterals and debts for users.
// Return total amount_scaled for collateral / debt and balance amounts for denoms.
pub fn merge_collaterals_and_debts(
    users_collaterals: &[&HashMap<String, UserCollateralResponse>],
    users_debts: &[&HashMap<String, UserDebtResponse>],
) -> (HashMap<String, Uint128>, HashMap<String, Uint128>, HashMap<String, Uint128>) {
    let mut balances: HashMap<String, Uint128> = HashMap::new();

    let mut merged_collaterals: HashMap<String, Uint128> = HashMap::new();

    for user_collaterals in users_collaterals {
        for (denom, collateral) in user_collaterals.iter() {
            merged_collaterals
                .entry(denom.clone())
                .and_modify(|v| {
                    *v += collateral.amount_scaled;
                })
                .or_insert(collateral.amount_scaled);
            balances
                .entry(denom.clone())
                .and_modify(|v| {
                    *v += collateral.amount;
                })
                .or_insert(collateral.amount);
        }
    }

    let mut merged_debts: HashMap<String, Uint128> = HashMap::new();

    for user_debts in users_debts {
        for (denom, debt) in user_debts.iter() {
            merged_debts
                .entry(denom.clone())
                .and_modify(|v| {
                    *v += debt.amount_scaled;
                })
                .or_insert(debt.amount_scaled);
            balances
                .entry(denom.clone())
                .and_modify(|v| {
                    *v -= debt.amount;
                })
                .or_insert(Uint128::zero()); // balance can't be negative
        }
    }

    (merged_collaterals, merged_debts, balances)
}

pub fn assert_err(res: AnyResult<AppResponse>, err: ContractError) {
    match res {
        Ok(_) => panic!("Result was not an error"),
        Err(generic_err) => {
            let contract_err: ContractError = generic_err.downcast().unwrap();
            assert_eq!(contract_err, err);
        }
    }
}

pub fn assert_err_with_str(res: AnyResult<AppResponse>, expected: impl Display) {
    match res {
        Ok(_) => panic!("Result was not an error"),
        Err(generic_err) => {
            let contract_err: ContractError = generic_err.downcast().unwrap();
            let msg = contract_err.to_string();
            println!("error: {}", msg); // print error for debugging
            assert!(msg.contains(&format!("{expected}")))
        }
    }
}

pub fn osmo_asset_params() -> (InitOrUpdateAssetParams, AssetParams) {
    default_asset_params_with("uosmo", Decimal::percent(70), Decimal::percent(78))
}

pub fn usdc_asset_params() -> (InitOrUpdateAssetParams, AssetParams) {
    default_asset_params_with("uusdc", Decimal::percent(90), Decimal::percent(96))
}

pub fn default_asset_params_with(
    denom: &str,
    max_loan_to_value: Decimal,
    liquidation_threshold: Decimal,
) -> (InitOrUpdateAssetParams, AssetParams) {
    let market_params = InitOrUpdateAssetParams {
        reserve_factor: Some(Decimal::percent(20)),
        interest_rate_model: Some(InterestRateModel {
            optimal_utilization_rate: Decimal::percent(10),
            base: Decimal::percent(30),
            slope_1: Decimal::percent(25),
            slope_2: Decimal::percent(30),
        }),
    };
    let asset_params = AssetParams {
        denom: denom.to_string(),
        credit_manager: CmSettings {
            whitelisted: false,
            hls: None,
        },
        red_bank: RedBankSettings {
            deposit_enabled: true,
            borrow_enabled: true,
        },
        max_loan_to_value,
        liquidation_threshold,
        liquidation_bonus: LiquidationBonus {
            starting_lb: Decimal::percent(1),
            slope: Decimal::from_str("2.0").unwrap(),
            min_lb: Decimal::percent(2),
            max_lb: Decimal::percent(10),
        },
        protocol_liquidation_fee: Decimal::percent(2),
        deposit_cap: Uint128::MAX,
    };
    (market_params, asset_params)
}
