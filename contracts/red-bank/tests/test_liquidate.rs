use std::cmp::min;

use cosmwasm_std::{
    attr, coin, coins,
    testing::{mock_info, MockApi, MockStorage},
    to_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal, Deps, OwnedDeps, StdError, StdResult,
    SubMsg, Uint128, WasmMsg,
};
use cw_utils::PaymentError;
use helpers::{
    has_collateral_position, set_collateral, th_build_interests_updated_event,
    th_get_expected_indices, th_get_expected_indices_and_rates, th_init_market, th_setup,
    TestUtilizationDeltaInfo,
};
use mars_red_bank::{
    contract::execute,
    error::ContractError,
    execute::liquidation_compute_amounts,
    interest_rates::{
        compute_scaled_amount, compute_underlying_amount, get_scaled_liquidity_amount,
        ScalingOperation, SCALING_FACTOR,
    },
    state::{COLLATERALS, CONFIG, DEBTS, MARKETS},
};
use mars_red_bank_types::{
    address_provider::MarsAddressType,
    incentives,
    red_bank::{Collateral, Debt, ExecuteMsg, InterestRateModel, Market},
};
use mars_testing::{mock_env, mock_env_at_block_time, MarsMockQuerier, MockEnvParams};
use mars_utils::math;

use crate::helpers::{set_debt, TestInterestResults};

mod helpers;

struct TestSuite {
    deps: OwnedDeps<MockStorage, MockApi, MarsMockQuerier>,
    collateral_coin: Coin,
    debt_coin: Coin,
    uncollateralized_denom: &'static str,
    collateral_price: Decimal,
    debt_price: Decimal,
    close_factor: Decimal,
    collateral_market: Market,
    debt_market: Market,
}

fn setup_test() -> TestSuite {
    let initial_collateral_coin = coin(1_000_000_000u128, "collateral");
    let initial_debt_coin = coin(2_000_000_000u128, "debt");
    let mut deps = th_setup(&[initial_collateral_coin.clone(), initial_debt_coin.clone()]);

    let close_factor = Decimal::from_ratio(1u128, 2u128);
    CONFIG
        .update(deps.as_mut().storage, |mut config| -> StdResult<_> {
            config.close_factor = close_factor;
            Ok(config)
        })
        .unwrap();

    let collateral_price = Decimal::from_ratio(2_u128, 1_u128);
    let debt_price = Decimal::from_ratio(11_u128, 10_u128);
    let uncollateralized_debt_price = Decimal::from_ratio(15_u128, 10_u128);
    let uncollateralized_denom = "uncollateralized_debt";
    deps.querier.set_oracle_price(&initial_collateral_coin.denom, collateral_price);
    deps.querier.set_oracle_price(&initial_debt_coin.denom, debt_price);
    deps.querier.set_oracle_price(uncollateralized_denom, uncollateralized_debt_price);

    // for the test to pass, we need an interest rate model that gives non-zero rates
    let mock_ir_model = InterestRateModel {
        optimal_utilization_rate: Decimal::percent(80),
        base: Decimal::percent(5),
        slope_1: Decimal::zero(),
        slope_2: Decimal::zero(),
    };

    let collateral_market = Market {
        max_loan_to_value: Decimal::from_ratio(5u128, 10u128),
        liquidation_threshold: Decimal::from_ratio(6u128, 10u128),
        liquidation_bonus: Decimal::from_ratio(1u128, 10u128),
        collateral_total_scaled: Uint128::new(1_500_000_000) * SCALING_FACTOR,
        debt_total_scaled: Uint128::new(800_000_000) * SCALING_FACTOR,
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::one(),
        liquidity_rate: Decimal::from_ratio(2u128, 10u128),
        borrow_rate: Decimal::from_ratio(2u128, 10u128),
        interest_rate_model: mock_ir_model.clone(),
        reserve_factor: Decimal::from_ratio(2u128, 100u128),
        indexes_last_updated: 0,
        ..Default::default()
    };

    let debt_market = Market {
        max_loan_to_value: Decimal::from_ratio(6u128, 10u128),
        collateral_total_scaled: Uint128::new(3_500_000_000) * SCALING_FACTOR,
        debt_total_scaled: Uint128::new(1_800_000_000) * SCALING_FACTOR,
        liquidity_index: Decimal::from_ratio(12u128, 10u128),
        borrow_index: Decimal::from_ratio(14u128, 10u128),
        liquidity_rate: Decimal::from_ratio(2u128, 10u128),
        borrow_rate: Decimal::from_ratio(2u128, 10u128),
        interest_rate_model: mock_ir_model,
        reserve_factor: Decimal::from_ratio(3u128, 100u128),
        indexes_last_updated: 0,
        ..Default::default()
    };

    let uncollateralized_debt_market = Market {
        denom: uncollateralized_denom.to_string(),
        ..Default::default()
    };

    let collateral_market =
        th_init_market(deps.as_mut(), &initial_collateral_coin.denom, &collateral_market);
    let debt_market = th_init_market(deps.as_mut(), &initial_debt_coin.denom, &debt_market);
    th_init_market(deps.as_mut(), uncollateralized_denom, &uncollateralized_debt_market);

    TestSuite {
        deps,
        collateral_coin: initial_collateral_coin,
        debt_coin: initial_debt_coin,
        uncollateralized_denom,
        collateral_price,
        debt_price,
        close_factor,
        collateral_market,
        debt_market,
    }
}

fn rewards_collector_collateral(deps: Deps, denom: &str) -> Collateral {
    COLLATERALS
        .load(
            deps.storage,
            (&Addr::unchecked(MarsAddressType::RewardsCollector.to_string()), denom),
        )
        .unwrap()
}

struct TestExpectedAmountResults {
    user_debt_repayed: Uint128,
    user_debt_repayed_scaled: Uint128,
    expected_refund_amount: Uint128,
    expected_liquidated_collateral_amount: Uint128,
    expected_liquidated_collateral_amount_scaled: Uint128,
    expected_reward_amount_scaled: Uint128,
    expected_debt_rates: TestInterestResults,
}

fn expected_amounts(
    block_time: u64,
    user_debt_scaled: Uint128,
    repay_amount: Uint128,
    test_suite: &TestSuite,
) -> TestExpectedAmountResults {
    let expected_debt_indices = th_get_expected_indices(&test_suite.debt_market, block_time);
    let user_debt = compute_underlying_amount(
        user_debt_scaled,
        expected_debt_indices.borrow,
        ScalingOperation::Ceil,
    )
    .unwrap();

    let max_repayable_debt = user_debt * test_suite.close_factor;
    let amount_to_repay = min(repay_amount, max_repayable_debt);
    let expected_refund_amount = if amount_to_repay < repay_amount {
        repay_amount - amount_to_repay
    } else {
        Uint128::zero()
    };

    let expected_debt_rates = th_get_expected_indices_and_rates(
        &test_suite.debt_market,
        block_time,
        TestUtilizationDeltaInfo {
            less_debt: amount_to_repay,
            user_current_debt_scaled: user_debt_scaled,
            less_liquidity: expected_refund_amount,
            ..Default::default()
        },
    );

    let expected_liquidated_collateral_amount = math::divide_uint128_by_decimal(
        amount_to_repay
            * test_suite.debt_price
            * (Decimal::one() + test_suite.collateral_market.liquidation_bonus),
        test_suite.collateral_price,
    )
    .unwrap();

    let expected_collateral_rates = th_get_expected_indices_and_rates(
        &test_suite.collateral_market,
        block_time,
        TestUtilizationDeltaInfo {
            less_liquidity: expected_liquidated_collateral_amount,
            ..Default::default()
        },
    );

    let expected_liquidated_collateral_amount_scaled = compute_scaled_amount(
        expected_liquidated_collateral_amount,
        expected_collateral_rates.liquidity_index,
        ScalingOperation::Truncate,
    )
    .unwrap();

    let expected_reward_amount_scaled = compute_scaled_amount(
        expected_debt_rates.protocol_rewards_to_distribute,
        expected_debt_rates.liquidity_index,
        ScalingOperation::Truncate,
    )
    .unwrap();

    TestExpectedAmountResults {
        user_debt_repayed: amount_to_repay,
        user_debt_repayed_scaled: expected_debt_rates.less_debt_scaled,
        expected_refund_amount,
        expected_liquidated_collateral_amount,
        expected_liquidated_collateral_amount_scaled,
        expected_reward_amount_scaled,
        expected_debt_rates,
    }
}

// recipient - can be liquidator or another address which can receive collateral
fn expected_messages(
    user_addr: &Addr,
    recipient_addr: &Addr,
    user_collateral_scaled: Uint128,
    recipient_collateral_scaled: Uint128,
    collateral_market: &Market,
    debt_market: &Market,
) -> Vec<SubMsg> {
    // there should be up to three messages updating indices at the incentives contract, in the
    // order:
    // - collateral denom, user
    // - collateral denom, liquidator
    // - debt denom, rewards collector (if rewards accrued > 0)
    //
    // NOTE that we don't expect a message to update rewards collector's index of the
    // **collateral** asset, because the liquidation action does NOT change the collateral
    // asset's utilization rate, it's interest rate does not need to be updated.
    vec![
        SubMsg::new(WasmMsg::Execute {
            contract_addr: MarsAddressType::Incentives.to_string(),
            msg: to_binary(&incentives::ExecuteMsg::BalanceChange {
                user_addr: user_addr.clone(),
                denom: collateral_market.denom.clone(),
                user_amount_scaled_before: user_collateral_scaled,
                total_amount_scaled_before: collateral_market.collateral_total_scaled,
            })
            .unwrap(),
            funds: vec![],
        }),
        SubMsg::new(WasmMsg::Execute {
            contract_addr: MarsAddressType::Incentives.to_string(),
            msg: to_binary(&incentives::ExecuteMsg::BalanceChange {
                user_addr: recipient_addr.clone(),
                denom: collateral_market.denom.clone(),
                user_amount_scaled_before: recipient_collateral_scaled,
                total_amount_scaled_before: collateral_market.collateral_total_scaled,
            })
            .unwrap(),
            funds: vec![],
        }),
        SubMsg::new(WasmMsg::Execute {
            contract_addr: MarsAddressType::Incentives.to_string(),
            msg: to_binary(&incentives::ExecuteMsg::BalanceChange {
                user_addr: Addr::unchecked(MarsAddressType::RewardsCollector.to_string()),
                denom: debt_market.denom.clone(),
                user_amount_scaled_before: Uint128::zero(),
                total_amount_scaled_before: debt_market.collateral_total_scaled,
            })
            .unwrap(),
            funds: vec![],
        }),
    ]
}

#[test]
fn liquidate_if_no_coins_sent() {
    let TestSuite {
        mut deps,
        ..
    } = setup_test();

    let env = mock_env(MockEnvParams::default());
    let info = mock_info("liquidator", &[]);
    let msg = ExecuteMsg::Liquidate {
        user: "user".to_string(),
        collateral_denom: "collateral".to_string(),
        recipient: None,
    };
    let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(error_res, PaymentError::NoFunds {}.into());
}

#[test]
fn liquidate_if_many_coins_sent() {
    let TestSuite {
        mut deps,
        ..
    } = setup_test();

    let env = mock_env(MockEnvParams::default());
    let info = mock_info("liquidator", &[coin(100, "somecoin1"), coin(200, "somecoin2")]);
    let msg = ExecuteMsg::Liquidate {
        user: "user".to_string(),
        collateral_denom: "collateral".to_string(),
        recipient: None,
    };
    let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(error_res, PaymentError::MultipleDenoms {}.into());
}

#[test]
fn liquidate_if_no_collateral() {
    let TestSuite {
        mut deps,
        collateral_coin,
        debt_coin,
        ..
    } = setup_test();

    let liquidate_msg = ExecuteMsg::Liquidate {
        user: "user".to_string(),
        collateral_denom: collateral_coin.denom,
        recipient: None,
    };

    let env = mock_env(MockEnvParams::default());
    let info = mock_info("liquidator", &coins(400_000_u128, debt_coin.denom));
    let error_res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap_err();
    assert_eq!(error_res, ContractError::CannotLiquidateWhenNoCollateralBalance {});
}

#[test]
fn liquidate_if_only_uncollateralized_debt_exists() {
    let TestSuite {
        mut deps,
        collateral_coin,
        debt_coin,
        uncollateralized_denom,
        collateral_market,
        ..
    } = setup_test();

    let user_addr = Addr::unchecked("user");

    set_collateral(
        deps.as_mut(),
        &user_addr,
        &collateral_market.denom,
        Uint128::new(2_000_000),
        true,
    );
    set_debt(deps.as_mut(), &user_addr, uncollateralized_denom, Uint128::new(10_000), true);

    let liquidate_msg = ExecuteMsg::Liquidate {
        user: user_addr.to_string(),
        collateral_denom: collateral_coin.denom,
        recipient: None,
    };

    let env = mock_env(MockEnvParams::default());
    let info = mock_info("liquidator", &coins(400_000_u128, debt_coin.denom));
    // trying to liquidate user with zero outstanding debt should fail (uncollateralized has not impact)
    let error_res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap_err();
    assert_eq!(error_res, ContractError::CannotLiquidateWhenNoDebtBalance {});
}

#[test]
fn liquidate_partially() {
    let mut ts = setup_test();

    let user_addr = Addr::unchecked("user");
    let liquidator_addr = Addr::unchecked("liquidator");

    let user_collateral_scaled_before = Uint128::from(2_000_000u64) * SCALING_FACTOR;
    let user_debt_scaled_before = compute_scaled_amount(
        Uint128::from(3_000_000u64),
        ts.debt_market.borrow_index,
        ScalingOperation::Ceil,
    )
    .unwrap();

    set_collateral(
        ts.deps.as_mut(),
        &user_addr,
        &ts.collateral_market.denom,
        user_collateral_scaled_before,
        true,
    );
    set_debt(ts.deps.as_mut(), &user_addr, &ts.debt_market.denom, user_debt_scaled_before, false);
    set_debt(
        ts.deps.as_mut(),
        &user_addr,
        ts.uncollateralized_denom,
        Uint128::new(10_000) * SCALING_FACTOR,
        true,
    );

    let liquidate_msg = ExecuteMsg::Liquidate {
        user: user_addr.to_string(),
        collateral_denom: ts.collateral_market.denom.clone(),
        recipient: None,
    };

    let debt_to_repay = Uint128::from(400_000_u64);
    let block_time = 15_000_000;
    let env = mock_env_at_block_time(block_time);
    let info = mock_info(
        liquidator_addr.as_str(),
        &coins(debt_to_repay.u128(), ts.debt_market.denom.clone()),
    );
    let res = execute(ts.deps.as_mut(), env, info, liquidate_msg).unwrap();

    let TestExpectedAmountResults {
        user_debt_repayed,
        user_debt_repayed_scaled,
        expected_liquidated_collateral_amount,
        expected_liquidated_collateral_amount_scaled,
        expected_reward_amount_scaled,
        expected_debt_rates,
        ..
    } = expected_amounts(block_time, user_debt_scaled_before, debt_to_repay, &ts);

    let expected_msgs = expected_messages(
        &user_addr,
        &liquidator_addr,
        user_collateral_scaled_before,
        Uint128::zero(),
        &ts.collateral_market,
        &ts.debt_market,
    );
    assert_eq!(res.messages, expected_msgs);

    mars_testing::assert_eq_vec(
        res.attributes,
        vec![
            attr("action", "liquidate"),
            attr("user", user_addr.as_str()),
            attr("liquidator", liquidator_addr.as_str()),
            attr("recipient", liquidator_addr.as_str()),
            attr("collateral_denom", ts.collateral_market.denom.as_str()),
            attr("collateral_amount", expected_liquidated_collateral_amount),
            attr("collateral_amount_scaled", expected_liquidated_collateral_amount_scaled),
            attr("debt_denom", ts.debt_market.denom.as_str()),
            attr("debt_amount", user_debt_repayed),
            attr("debt_amount_scaled", user_debt_repayed_scaled),
        ],
    );
    assert_eq!(
        res.events,
        vec![th_build_interests_updated_event(&ts.debt_market.denom, &expected_debt_rates)]
    );

    let debt_market_after = MARKETS.load(&ts.deps.storage, &ts.debt_market.denom).unwrap();

    // user's collateral scaled amount should have been correctly decreased
    let collateral = COLLATERALS
        .load(ts.deps.as_ref().storage, (&user_addr, &ts.collateral_market.denom))
        .unwrap();
    assert_eq!(
        collateral.amount_scaled,
        user_collateral_scaled_before - expected_liquidated_collateral_amount_scaled
    );

    // liquidator's collateral scaled amount should have been correctly increased
    let collateral = COLLATERALS
        .load(ts.deps.as_ref().storage, (&liquidator_addr, &ts.collateral_market.denom))
        .unwrap();
    assert_eq!(collateral.amount_scaled, expected_liquidated_collateral_amount_scaled);

    // check user's debt decreased by the appropriate amount
    let debt = DEBTS.load(&ts.deps.storage, (&user_addr, &ts.debt_market.denom)).unwrap();
    assert_eq!(debt.amount_scaled, user_debt_scaled_before - user_debt_repayed_scaled);

    // check global debt decreased by the appropriate amount
    assert_eq!(
        debt_market_after.debt_total_scaled,
        ts.debt_market.debt_total_scaled - user_debt_repayed_scaled
    );

    // rewards collector's collateral scaled amount **of the debt asset** should have been correctly increased
    let collateral = rewards_collector_collateral(ts.deps.as_ref(), &ts.debt_market.denom);
    assert_eq!(collateral.amount_scaled, expected_reward_amount_scaled);

    // global collateral scaled amount **of the debt asset** should have been correctly increased
    assert_eq!(
        debt_market_after.collateral_total_scaled,
        ts.debt_market.collateral_total_scaled + expected_reward_amount_scaled
    );
}

#[test]
fn liquidate_up_to_close_factor_with_refund() {
    let mut ts = setup_test();

    let user_addr = Addr::unchecked("user");
    let liquidator_addr = Addr::unchecked("liquidator");

    let user_collateral_scaled_before = Uint128::from(2_000_000u64) * SCALING_FACTOR;
    let user_debt_scaled_before = compute_scaled_amount(
        Uint128::from(3_000_000u64),
        ts.debt_market.borrow_index,
        ScalingOperation::Ceil,
    )
    .unwrap();

    set_collateral(
        ts.deps.as_mut(),
        &user_addr,
        &ts.collateral_market.denom,
        user_collateral_scaled_before,
        true,
    );
    set_debt(ts.deps.as_mut(), &user_addr, &ts.debt_market.denom, user_debt_scaled_before, false);

    let liquidate_msg = ExecuteMsg::Liquidate {
        user: user_addr.to_string(),
        collateral_denom: ts.collateral_market.denom.clone(),
        recipient: None,
    };

    let debt_to_repay = Uint128::from(10_000_000_u64);
    let block_time = 16_000_000;
    let env = mock_env_at_block_time(block_time);
    let info = mock_info(
        liquidator_addr.as_str(),
        &coins(debt_to_repay.u128(), ts.debt_market.denom.clone()),
    );
    let res = execute(ts.deps.as_mut(), env, info, liquidate_msg).unwrap();

    let TestExpectedAmountResults {
        user_debt_repayed,
        user_debt_repayed_scaled,
        expected_refund_amount,
        expected_liquidated_collateral_amount,
        expected_liquidated_collateral_amount_scaled,
        expected_reward_amount_scaled,
        expected_debt_rates,
        ..
    } = expected_amounts(block_time, user_debt_scaled_before, debt_to_repay, &ts);

    let mut expected_msgs = expected_messages(
        &user_addr,
        &liquidator_addr,
        user_collateral_scaled_before,
        Uint128::zero(),
        &ts.collateral_market,
        &ts.debt_market,
    );
    expected_msgs.push(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
        to_address: liquidator_addr.to_string(),
        amount: coins(expected_refund_amount.u128(), ts.debt_market.denom.clone()),
    })));
    assert_eq!(res.messages, expected_msgs);

    mars_testing::assert_eq_vec(
        vec![
            attr("action", "liquidate"),
            attr("user", user_addr.as_str()),
            attr("liquidator", liquidator_addr.as_str()),
            attr("recipient", liquidator_addr.as_str()),
            attr("collateral_denom", ts.collateral_market.denom.as_str()),
            attr("collateral_amount", expected_liquidated_collateral_amount),
            attr("collateral_amount_scaled", expected_liquidated_collateral_amount_scaled),
            attr("debt_denom", ts.debt_market.denom.as_str()),
            attr("debt_amount", user_debt_repayed),
            attr("debt_amount_scaled", user_debt_repayed_scaled),
        ],
        res.attributes,
    );
    assert_eq!(
        res.events,
        vec![th_build_interests_updated_event(&ts.debt_market.denom, &expected_debt_rates)],
    );

    let debt_market_after = MARKETS.load(&ts.deps.storage, &ts.debt_market.denom).unwrap();

    // user's collateral scaled amount should have been correctly decreased
    let collateral = COLLATERALS
        .load(ts.deps.as_ref().storage, (&user_addr, &ts.collateral_market.denom))
        .unwrap();
    assert_eq!(
        collateral.amount_scaled,
        user_collateral_scaled_before - expected_liquidated_collateral_amount_scaled
    );

    // liquidator's collateral scaled amount should have been correctly increased
    let collateral = COLLATERALS
        .load(ts.deps.as_ref().storage, (&liquidator_addr, &ts.collateral_market.denom))
        .unwrap();
    assert_eq!(collateral.amount_scaled, expected_liquidated_collateral_amount_scaled);

    // check user's debt decreased by the appropriate amount
    let debt = DEBTS.load(&ts.deps.storage, (&user_addr, &ts.debt_market.denom)).unwrap();
    assert_eq!(debt.amount_scaled, user_debt_scaled_before - expected_debt_rates.less_debt_scaled);

    // check global debt decreased by the appropriate amount
    assert_eq!(
        debt_market_after.debt_total_scaled,
        ts.debt_market.debt_total_scaled - expected_debt_rates.less_debt_scaled
    );

    // rewards collector's collateral scaled amount **of the debt asset** should have been correctly increased
    let collateral = rewards_collector_collateral(ts.deps.as_ref(), &ts.debt_market.denom);
    assert_eq!(collateral.amount_scaled, expected_reward_amount_scaled);

    // global collateral scaled amount **of the debt asset** should have been correctly increased
    assert_eq!(
        debt_market_after.collateral_total_scaled,
        ts.debt_market.collateral_total_scaled + expected_reward_amount_scaled
    );
}

#[test]
fn liquidate_fully() {
    let TestSuite {
        mut deps,
        collateral_price,
        debt_price,
        collateral_market,
        debt_market,
        ..
    } = setup_test();

    let user_addr = Addr::unchecked("user");
    let liquidator_addr = Addr::unchecked("liquidator");

    let user_collateral_scaled_before = Uint128::new(100) * SCALING_FACTOR;
    let user_debt_scaled_before = Uint128::new(400) * SCALING_FACTOR;

    set_collateral(
        deps.as_mut(),
        &user_addr,
        &collateral_market.denom,
        user_collateral_scaled_before,
        true,
    );
    set_debt(deps.as_mut(), &user_addr, &debt_market.denom, user_debt_scaled_before, false);

    let liquidate_msg = ExecuteMsg::Liquidate {
        user: user_addr.to_string(),
        collateral_denom: collateral_market.denom.clone(),
        recipient: None,
    };

    let debt_to_repay = Uint128::from(300u128);
    let block_time = 16_000_000;
    let env = mock_env_at_block_time(block_time);
    let info = mock_info(
        liquidator_addr.as_str(),
        &coins(debt_to_repay.u128(), debt_market.denom.clone()),
    );
    let res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap();

    // get expected indices and rates for debt and collateral markets
    let expected_collateral_indices = th_get_expected_indices(&collateral_market, block_time);
    let user_collateral_balance = compute_underlying_amount(
        user_collateral_scaled_before,
        expected_collateral_indices.liquidity,
        ScalingOperation::Truncate,
    )
    .unwrap();

    // Since debt is being over_repayed, we expect to liquidate total collateral
    let expected_less_debt = math::divide_uint128_by_decimal(
        math::divide_uint128_by_decimal(collateral_price * user_collateral_balance, debt_price)
            .unwrap(),
        Decimal::one() + collateral_market.liquidation_bonus,
    )
    .unwrap();

    let expected_refund_amount = debt_to_repay - expected_less_debt;

    let expected_debt_rates = th_get_expected_indices_and_rates(
        &debt_market,
        block_time,
        TestUtilizationDeltaInfo {
            less_debt: expected_less_debt,
            user_current_debt_scaled: user_debt_scaled_before,
            less_liquidity: expected_refund_amount,
            ..Default::default()
        },
    );

    let debt_market_after = MARKETS.load(&deps.storage, &debt_market.denom).unwrap();

    // since this is a full liquidation, the full amount of user's collateral shares should have
    // been transferred to the liquidator
    let expected_liquidated_collateral_amount_scaled = user_collateral_scaled_before;

    let mut expected_msgs = expected_messages(
        &user_addr,
        &liquidator_addr,
        user_collateral_scaled_before,
        Uint128::zero(),
        &collateral_market,
        &debt_market,
    );
    expected_msgs.push(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
        to_address: liquidator_addr.to_string(),
        amount: coins(expected_refund_amount.u128(), debt_market.denom.clone()),
    })));
    assert_eq!(res.messages, expected_msgs);

    mars_testing::assert_eq_vec(
        vec![
            attr("action", "liquidate"),
            attr("user", user_addr.as_str()),
            attr("liquidator", liquidator_addr.as_str()),
            attr("recipient", liquidator_addr.as_str()),
            attr("collateral_denom", collateral_market.denom.as_str()),
            attr("collateral_amount", user_collateral_balance),
            attr("collateral_amount_scaled", expected_liquidated_collateral_amount_scaled),
            attr("debt_denom", debt_market.denom.as_str()),
            attr("debt_amount", expected_less_debt),
            attr("debt_amount_scaled", expected_debt_rates.less_debt_scaled),
        ],
        res.attributes,
    );
    assert_eq!(
        res.events,
        vec![th_build_interests_updated_event(&debt_market.denom, &expected_debt_rates)],
    );

    // since this is a full liquidation, the user's collateral position should have been deleted
    assert!(!has_collateral_position(deps.as_ref(), &user_addr, &collateral_market.denom));

    // liquidator's collateral scaled amount should have been correctly increased
    let collateral = COLLATERALS
        .load(deps.as_ref().storage, (&liquidator_addr, &collateral_market.denom))
        .unwrap();
    assert_eq!(collateral.amount_scaled, expected_liquidated_collateral_amount_scaled);

    // check user's debt decreased by the appropriate amount
    let debt = DEBTS.load(&deps.storage, (&user_addr, &debt_market.denom)).unwrap();
    assert_eq!(debt.amount_scaled, user_debt_scaled_before - expected_debt_rates.less_debt_scaled);

    // check global debt decreased by the appropriate amount
    assert_eq!(
        debt_market_after.debt_total_scaled,
        debt_market.debt_total_scaled - expected_debt_rates.less_debt_scaled
    );
}

/// FIXME: new clippy version warns to remove clone() from "collateral_market.clone()" but then it breaks compilation
#[allow(clippy::redundant_clone)]
#[test]
fn liquidate_partially_if_same_asset_for_debt_and_collateral() {
    let TestSuite {
        mut deps,
        collateral_price,
        collateral_market,
        ..
    } = setup_test();
    let debt_price = collateral_price;
    let debt_market = collateral_market.clone();

    let user_addr = Addr::unchecked("user");
    let liquidator_addr = Addr::unchecked("liquidator");

    let user_collateral_scaled_before = Uint128::from(2_000_000u64) * SCALING_FACTOR;
    let user_debt_scaled_before = compute_scaled_amount(
        Uint128::from(3_000_000u64),
        debt_market.borrow_index,
        ScalingOperation::Ceil,
    )
    .unwrap();

    set_collateral(
        deps.as_mut(),
        &user_addr,
        &collateral_market.denom,
        user_collateral_scaled_before,
        true,
    );
    set_debt(deps.as_mut(), &user_addr, &debt_market.denom, user_debt_scaled_before, false);

    let liquidate_msg = ExecuteMsg::Liquidate {
        user: user_addr.to_string(),
        collateral_denom: collateral_market.denom.clone(),
        recipient: None,
    };

    let debt_to_repay = Uint128::from(400_000_u64);
    let block_time = 15_000_000;
    let env = mock_env_at_block_time(block_time);
    let info = mock_info(
        liquidator_addr.as_str(),
        &coins(debt_to_repay.u128(), debt_market.denom.clone()),
    );
    let res = execute(deps.as_mut(), env.clone(), info, liquidate_msg).unwrap();

    // get expected indices and rates for debt market
    let expected_debt_rates = th_get_expected_indices_and_rates(
        &debt_market,
        block_time,
        TestUtilizationDeltaInfo {
            less_debt: debt_to_repay,
            user_current_debt_scaled: user_debt_scaled_before,
            ..Default::default()
        },
    );

    let collateral_market_after = MARKETS.load(&deps.storage, &collateral_market.denom).unwrap();
    let debt_market_after = MARKETS.load(&deps.storage, &debt_market.denom).unwrap();

    let expected_liquidated_collateral_amount = math::divide_uint128_by_decimal(
        debt_to_repay * debt_price * (Decimal::one() + collateral_market.liquidation_bonus),
        collateral_price,
    )
    .unwrap();

    let expected_liquidated_collateral_amount_scaled = get_scaled_liquidity_amount(
        expected_liquidated_collateral_amount,
        &collateral_market_after,
        env.block.time.seconds(),
    )
    .unwrap();

    let expected_reward_amount_scaled = compute_scaled_amount(
        expected_debt_rates.protocol_rewards_to_distribute,
        expected_debt_rates.liquidity_index,
        ScalingOperation::Truncate,
    )
    .unwrap();

    let expected_msgs = expected_messages(
        &user_addr,
        &liquidator_addr,
        user_collateral_scaled_before,
        Uint128::zero(),
        &collateral_market,
        &debt_market,
    );
    assert_eq!(res.messages, expected_msgs);

    mars_testing::assert_eq_vec(
        res.attributes,
        vec![
            attr("action", "liquidate"),
            attr("user", user_addr.as_str()),
            attr("liquidator", liquidator_addr.as_str()),
            attr("recipient", liquidator_addr.as_str()),
            attr("collateral_denom", collateral_market.denom.as_str()),
            attr("collateral_amount", expected_liquidated_collateral_amount),
            attr("collateral_amount_scaled", expected_liquidated_collateral_amount_scaled),
            attr("debt_denom", debt_market.denom.as_str()),
            attr("debt_amount", debt_to_repay),
            attr("debt_amount_scaled", expected_debt_rates.less_debt_scaled),
        ],
    );
    assert_eq!(
        res.events,
        vec![th_build_interests_updated_event(&debt_market.denom, &expected_debt_rates)]
    );

    // user's collateral scaled amount should have been correctly decreased
    let collateral =
        COLLATERALS.load(deps.as_ref().storage, (&user_addr, &collateral_market.denom)).unwrap();
    assert_eq!(
        collateral.amount_scaled,
        user_collateral_scaled_before - expected_liquidated_collateral_amount_scaled
    );

    // liquidator's collateral scaled amount should have been correctly increased
    let collateral = COLLATERALS
        .load(deps.as_ref().storage, (&liquidator_addr, &collateral_market.denom))
        .unwrap();
    assert_eq!(collateral.amount_scaled, expected_liquidated_collateral_amount_scaled);

    // check user's debt decreased by the appropriate amount
    let debt = DEBTS.load(&deps.storage, (&user_addr, &debt_market.denom)).unwrap();
    assert_eq!(debt.amount_scaled, user_debt_scaled_before - expected_debt_rates.less_debt_scaled);

    // check global debt decreased by the appropriate amount
    assert_eq!(
        debt_market_after.debt_total_scaled,
        debt_market.debt_total_scaled - expected_debt_rates.less_debt_scaled
    );

    // rewards collector's collateral scaled amount **of the debt asset** should have been correctly increased
    let collateral = rewards_collector_collateral(deps.as_ref(), &debt_market.denom);
    assert_eq!(collateral.amount_scaled, expected_reward_amount_scaled);

    // global collateral scaled amount **of the debt asset** should have been correctly increased
    assert_eq!(
        debt_market_after.collateral_total_scaled,
        debt_market.collateral_total_scaled + expected_reward_amount_scaled
    );
}

/// FIXME: new clippy version warns to remove clone() from "collateral_market.clone()" but then it breaks compilation
#[allow(clippy::redundant_clone)]
#[test]
fn liquidate_with_refund_if_same_asset_for_debt_and_collateral() {
    let TestSuite {
        mut deps,
        collateral_price,
        close_factor,
        collateral_market,
        ..
    } = setup_test();
    let debt_price = collateral_price;
    let debt_market = collateral_market.clone();

    let user_addr = Addr::unchecked("user");
    let liquidator_addr = Addr::unchecked("liquidator");

    let user_collateral_scaled_before = Uint128::from(2_000_000u64) * SCALING_FACTOR;
    let user_debt_scaled_before = compute_scaled_amount(
        Uint128::from(3_000_000u64),
        debt_market.borrow_index,
        ScalingOperation::Ceil,
    )
    .unwrap();

    set_collateral(
        deps.as_mut(),
        &user_addr,
        &collateral_market.denom,
        user_collateral_scaled_before,
        true,
    );
    set_debt(deps.as_mut(), &user_addr, &debt_market.denom, user_debt_scaled_before, false);

    let liquidate_msg = ExecuteMsg::Liquidate {
        user: user_addr.to_string(),
        collateral_denom: collateral_market.denom.clone(),
        recipient: None,
    };

    let debt_to_repay = Uint128::from(10_000_000_u64);
    let block_time = 16_000_000;
    let env = mock_env_at_block_time(block_time);
    let info = mock_info(
        liquidator_addr.as_str(),
        &coins(debt_to_repay.u128(), debt_market.denom.clone()),
    );
    let res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap();

    // get expected indices and rates for debt and collateral markets
    let expected_debt_indices = th_get_expected_indices(&debt_market, block_time);
    let user_debt_asset_total_debt = compute_underlying_amount(
        user_debt_scaled_before,
        expected_debt_indices.borrow,
        ScalingOperation::Ceil,
    )
    .unwrap();
    // since debt is being over_repayed, we expect to max out the liquidatable debt
    let expected_less_debt = user_debt_asset_total_debt * close_factor;

    let expected_refund_amount = debt_to_repay - expected_less_debt;

    let expected_debt_rates = th_get_expected_indices_and_rates(
        &debt_market,
        block_time,
        TestUtilizationDeltaInfo {
            less_debt: expected_less_debt,
            user_current_debt_scaled: user_debt_scaled_before,
            less_liquidity: expected_refund_amount,
            ..Default::default()
        },
    );

    let expected_liquidated_collateral_amount = math::divide_uint128_by_decimal(
        expected_less_debt * debt_price * (Decimal::one() + collateral_market.liquidation_bonus),
        collateral_price,
    )
    .unwrap();

    let expected_collateral_rates = th_get_expected_indices_and_rates(
        &collateral_market,
        block_time,
        TestUtilizationDeltaInfo {
            less_liquidity: expected_liquidated_collateral_amount,
            ..Default::default()
        },
    );

    let debt_market_after = MARKETS.load(&deps.storage, &debt_market.denom).unwrap();

    let expected_liquidated_collateral_amount_scaled = compute_scaled_amount(
        expected_liquidated_collateral_amount,
        expected_collateral_rates.liquidity_index,
        ScalingOperation::Truncate,
    )
    .unwrap();

    let expected_reward_amount_scaled = compute_scaled_amount(
        expected_debt_rates.protocol_rewards_to_distribute,
        expected_debt_rates.liquidity_index,
        ScalingOperation::Truncate,
    )
    .unwrap();

    let mut expected_msgs = expected_messages(
        &user_addr,
        &liquidator_addr,
        user_collateral_scaled_before,
        Uint128::zero(),
        &collateral_market,
        &debt_market,
    );
    expected_msgs.push(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
        to_address: liquidator_addr.to_string(),
        amount: coins(expected_refund_amount.u128(), debt_market.denom.clone()),
    })));
    assert_eq!(res.messages, expected_msgs);

    mars_testing::assert_eq_vec(
        vec![
            attr("action", "liquidate"),
            attr("user", user_addr.as_str()),
            attr("liquidator", liquidator_addr.as_str()),
            attr("recipient", liquidator_addr.as_str()),
            attr("collateral_denom", collateral_market.denom.as_str()),
            attr("collateral_amount", expected_liquidated_collateral_amount),
            attr("collateral_amount_scaled", expected_liquidated_collateral_amount_scaled),
            attr("debt_denom", debt_market.denom.as_str()),
            attr("debt_amount", expected_less_debt),
            attr("debt_amount_scaled", expected_debt_rates.less_debt_scaled),
        ],
        res.attributes,
    );
    assert_eq!(
        res.events,
        vec![th_build_interests_updated_event(&debt_market.denom, &expected_debt_rates)],
    );

    // user's collateral scaled amount should have been correctly decreased
    let collateral =
        COLLATERALS.load(deps.as_ref().storage, (&user_addr, &collateral_market.denom)).unwrap();
    assert_eq!(
        collateral.amount_scaled,
        user_collateral_scaled_before - expected_liquidated_collateral_amount_scaled
    );

    // liquidator's collateral scaled amount should have been correctly increased
    let collateral = COLLATERALS
        .load(deps.as_ref().storage, (&liquidator_addr, &collateral_market.denom))
        .unwrap();
    assert_eq!(collateral.amount_scaled, expected_liquidated_collateral_amount_scaled);

    // check user's debt decreased by the appropriate amount
    let debt = DEBTS.load(&deps.storage, (&user_addr, &debt_market.denom)).unwrap();
    assert_eq!(debt.amount_scaled, user_debt_scaled_before - expected_debt_rates.less_debt_scaled);

    // check global debt decreased by the appropriate amount
    assert_eq!(
        debt_market_after.debt_total_scaled,
        debt_market.debt_total_scaled - expected_debt_rates.less_debt_scaled
    );

    // rewards collector's collateral scaled amount **of the debt asset** should have been correctly increased
    let collateral = rewards_collector_collateral(deps.as_ref(), &debt_market.denom);
    assert_eq!(collateral.amount_scaled, expected_reward_amount_scaled);

    // global collateral scaled amount **of the debt asset** should have been correctly increased
    assert_eq!(
        debt_market_after.collateral_total_scaled,
        debt_market.collateral_total_scaled + expected_reward_amount_scaled
    );
}

#[test]
fn liquidate_with_recipient_for_underlying_collateral() {
    let mut ts = setup_test();

    let user_addr = Addr::unchecked("user");
    let liquidator_addr = Addr::unchecked("liquidator");
    let recipient_addr = Addr::unchecked("recipient");

    let user_collateral_scaled_before = Uint128::from(2_000_000u64) * SCALING_FACTOR;
    let user_debt_scaled_before = compute_scaled_amount(
        Uint128::from(3_000_000u64),
        ts.debt_market.borrow_index,
        ScalingOperation::Ceil,
    )
    .unwrap();

    set_collateral(
        ts.deps.as_mut(),
        &user_addr,
        &ts.collateral_market.denom,
        user_collateral_scaled_before,
        true,
    );
    set_debt(ts.deps.as_mut(), &user_addr, &ts.debt_market.denom, user_debt_scaled_before, false);

    let liquidate_msg = ExecuteMsg::Liquidate {
        user: user_addr.to_string(),
        collateral_denom: ts.collateral_market.denom.clone(),
        recipient: Some(recipient_addr.to_string()),
    };

    let debt_to_repay = Uint128::from(10_000_000_u64);
    let block_time = 16_000_000;
    let env = mock_env_at_block_time(block_time);
    let info = mock_info(
        liquidator_addr.as_str(),
        &coins(debt_to_repay.u128(), ts.debt_market.denom.clone()),
    );
    let res = execute(ts.deps.as_mut(), env, info, liquidate_msg).unwrap();

    let TestExpectedAmountResults {
        user_debt_repayed,
        user_debt_repayed_scaled,
        expected_refund_amount,
        expected_liquidated_collateral_amount,
        expected_liquidated_collateral_amount_scaled,
        expected_reward_amount_scaled,
        expected_debt_rates,
        ..
    } = expected_amounts(block_time, user_debt_scaled_before, debt_to_repay, &ts);

    let mut expected_msgs = expected_messages(
        &user_addr,
        &recipient_addr,
        user_collateral_scaled_before,
        Uint128::zero(),
        &ts.collateral_market,
        &ts.debt_market,
    );
    expected_msgs.push(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
        to_address: liquidator_addr.to_string(),
        amount: coins(expected_refund_amount.u128(), ts.debt_market.denom.clone()),
    })));
    assert_eq!(res.messages, expected_msgs);

    mars_testing::assert_eq_vec(
        vec![
            attr("action", "liquidate"),
            attr("user", user_addr.as_str()),
            attr("liquidator", liquidator_addr.as_str()),
            attr("recipient", recipient_addr.as_str()),
            attr("collateral_denom", ts.collateral_market.denom.as_str()),
            attr("collateral_amount", expected_liquidated_collateral_amount),
            attr("collateral_amount_scaled", expected_liquidated_collateral_amount_scaled),
            attr("debt_denom", ts.debt_market.denom.as_str()),
            attr("debt_amount", user_debt_repayed),
            attr("debt_amount_scaled", user_debt_repayed_scaled),
        ],
        res.attributes,
    );
    assert_eq!(
        res.events,
        vec![th_build_interests_updated_event(&ts.debt_market.denom, &expected_debt_rates)],
    );

    let debt_market_after = MARKETS.load(&ts.deps.storage, &ts.debt_market.denom).unwrap();

    // user's collateral scaled amount should have been correctly decreased
    let collateral = COLLATERALS
        .load(ts.deps.as_ref().storage, (&user_addr, &ts.collateral_market.denom))
        .unwrap();
    assert_eq!(
        collateral.amount_scaled,
        user_collateral_scaled_before - expected_liquidated_collateral_amount_scaled
    );

    // liquidator's collateral should be empty
    COLLATERALS
        .load(ts.deps.as_ref().storage, (&liquidator_addr, &ts.collateral_market.denom))
        .unwrap_err();

    // recipient's collateral scaled amount should have been correctly increased
    let collateral = COLLATERALS
        .load(ts.deps.as_ref().storage, (&recipient_addr, &ts.collateral_market.denom))
        .unwrap();
    assert_eq!(collateral.amount_scaled, expected_liquidated_collateral_amount_scaled);

    // check user's debt decreased by the appropriate amount
    let debt = DEBTS.load(&ts.deps.storage, (&user_addr, &ts.debt_market.denom)).unwrap();
    assert_eq!(debt.amount_scaled, user_debt_scaled_before - expected_debt_rates.less_debt_scaled);

    // check global debt decreased by the appropriate amount
    assert_eq!(
        debt_market_after.debt_total_scaled,
        ts.debt_market.debt_total_scaled - expected_debt_rates.less_debt_scaled
    );

    // rewards collector's collateral scaled amount **of the debt asset** should have been correctly increased
    let collateral = rewards_collector_collateral(ts.deps.as_ref(), &ts.debt_market.denom);
    assert_eq!(collateral.amount_scaled, expected_reward_amount_scaled);

    // global collateral scaled amount **of the debt asset** should have been correctly increased
    assert_eq!(
        debt_market_after.collateral_total_scaled,
        ts.debt_market.collateral_total_scaled + expected_reward_amount_scaled
    );
}

#[test]
fn liquidation_health_factor_check() {
    // initialize collateral and debt markets
    let available_liquidity_collateral = Uint128::from(1000000000u128);
    let available_liquidity_debt = Uint128::from(2000000000u128);
    let mut deps = th_setup(&[
        coin(available_liquidity_collateral.into(), "collateral"),
        coin(available_liquidity_debt.into(), "debt"),
    ]);

    deps.querier.set_oracle_price("collateral", Decimal::one());
    deps.querier.set_oracle_price("debt", Decimal::one());
    deps.querier.set_oracle_price("uncollateralized_debt", Decimal::one());

    let collateral_ltv = Decimal::from_ratio(5u128, 10u128);
    let collateral_liquidation_threshold = Decimal::from_ratio(7u128, 10u128);
    let collateral_liquidation_bonus = Decimal::from_ratio(1u128, 10u128);

    let collateral_market = Market {
        max_loan_to_value: collateral_ltv,
        liquidation_threshold: collateral_liquidation_threshold,
        liquidation_bonus: collateral_liquidation_bonus,
        debt_total_scaled: Uint128::zero(),
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::one(),
        ..Default::default()
    };
    let debt_market = Market {
        max_loan_to_value: Decimal::from_ratio(6u128, 10u128),
        debt_total_scaled: Uint128::new(20_000_000) * SCALING_FACTOR,
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::one(),
        ..Default::default()
    };
    let uncollateralized_debt_market = Market {
        denom: "uncollateralized_debt".to_string(),
        ..Default::default()
    };

    // initialize markets
    th_init_market(deps.as_mut(), "collateral", &collateral_market);
    th_init_market(deps.as_mut(), "debt", &debt_market);
    th_init_market(deps.as_mut(), "uncollateralized_debt", &uncollateralized_debt_market);

    // test health factor check
    let healthy_user_addr = Addr::unchecked("healthy_user");

    // set initial collateral and debt balances for user
    let healthy_user_collateral_balance_scaled = Uint128::new(10_000_000) * SCALING_FACTOR;
    set_collateral(
        deps.as_mut(),
        &healthy_user_addr,
        "collateral",
        healthy_user_collateral_balance_scaled,
        true,
    );

    let healthy_user_debt_amount_scaled =
        Uint128::new(healthy_user_collateral_balance_scaled.u128())
            * collateral_liquidation_threshold;
    let healthy_user_debt = Debt {
        amount_scaled: healthy_user_debt_amount_scaled,
        uncollateralized: false,
    };
    let uncollateralized_debt = Debt {
        amount_scaled: Uint128::new(10_000) * SCALING_FACTOR,
        uncollateralized: true,
    };
    DEBTS.save(deps.as_mut().storage, (&healthy_user_addr, "debt"), &healthy_user_debt).unwrap();
    DEBTS
        .save(
            deps.as_mut().storage,
            (&healthy_user_addr, "uncollateralized_debt"),
            &uncollateralized_debt,
        )
        .unwrap();

    // perform liquidation (should fail because health factor is > 1)
    let liquidator_addr = Addr::unchecked("liquidator");
    let debt_to_cover = Uint128::from(1_000_000u64);

    let liquidate_msg = ExecuteMsg::Liquidate {
        user: healthy_user_addr.to_string(),
        collateral_denom: "collateral".to_string(),
        recipient: None,
    };

    let env = mock_env(MockEnvParams::default());
    let info = mock_info(liquidator_addr.as_str(), &coins(debt_to_cover.u128(), "debt"));
    let error_res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap_err();
    assert_eq!(error_res, ContractError::CannotLiquidateHealthyPosition {});
}

#[test]
fn liquidate_if_collateral_disabled() {
    // initialize collateral and debt markets
    let mut deps = th_setup(&[]);

    let collateral_market_1 = Market {
        ..Default::default()
    };
    let collateral_market_2 = Market {
        ..Default::default()
    };
    let debt_market = Market {
        ..Default::default()
    };

    // initialize markets
    th_init_market(deps.as_mut(), "collateral1", &collateral_market_1);
    th_init_market(deps.as_mut(), "collateral2", &collateral_market_2);
    th_init_market(deps.as_mut(), "debt", &debt_market);

    // Set user as having collateral and debt in respective markets
    let user_addr = Addr::unchecked("user");
    set_collateral(deps.as_mut(), &user_addr, "collateral1", Uint128::new(123), true);
    set_collateral(deps.as_mut(), &user_addr, "collateral2", Uint128::new(123), false);

    // perform liquidation (should fail because collateral2 isn't set as collateral for user)
    let liquidator_addr = Addr::unchecked("liquidator");
    let debt_to_cover = Uint128::from(1_000_000u64);

    let liquidate_msg = ExecuteMsg::Liquidate {
        user: user_addr.to_string(),
        collateral_denom: "collateral2".to_string(),
        recipient: None,
    };

    let env = mock_env(MockEnvParams::default());
    let info = mock_info(liquidator_addr.as_str(), &coins(debt_to_cover.u128(), "debt"));
    let error_res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap_err();
    assert_eq!(
        error_res,
        ContractError::CannotLiquidateWhenCollateralUnset {
            denom: "collateral2".to_string()
        }
    );
}

#[test]
fn liquidator_cannot_receive_collaterals_without_spending_coins() {
    let market = Market {
        liquidity_index: Decimal::one(),
        liquidation_bonus: Decimal::from_ratio(1u128, 10u128),
        ..Default::default()
    };
    let res_err = liquidation_compute_amounts(
        Uint128::new(320000000),
        Uint128::new(800),
        Uint128::new(2),
        &market,
        Decimal::one(),
        Decimal::from_ratio(300u128, 1u128),
        0,
        Decimal::from_ratio(1u128, 2u128),
    )
    .unwrap_err();
    assert_eq!(res_err, StdError::generic_err("Can't process liquidation. Invalid collateral_amount_to_liquidate (320) and debt_amount_to_repay (0)"))
}

#[test]
fn cannot_liquidate_without_receiving_collaterals() {
    let market = Market {
        liquidity_index: Decimal::one(),
        liquidation_bonus: Decimal::from_ratio(1u128, 10u128),
        ..Default::default()
    };
    let res_err = liquidation_compute_amounts(
        Uint128::new(320000000),
        Uint128::new(20),
        Uint128::new(30),
        &market,
        Decimal::from_ratio(12u128, 1u128),
        Decimal::one(),
        0,
        Decimal::from_ratio(1u128, 2u128),
    )
    .unwrap_err();
    assert_eq!(res_err, StdError::generic_err("Can't process liquidation. Invalid collateral_amount_to_liquidate (0) and debt_amount_to_repay (10)"))
}
