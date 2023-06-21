use cosmwasm_std::{
    attr, coin, coins,
    testing::{mock_env, mock_info, MockApi, MockStorage},
    to_binary, Addr, BankMsg, CosmosMsg, Decimal, OwnedDeps, SubMsg, Uint128, WasmMsg,
};
use helpers::{
    has_collateral_position, set_collateral, th_build_interests_updated_event,
    th_get_expected_indices_and_rates, th_setup, TestUtilizationDeltaInfo,
};
use mars_params::types::AssetParams;
use mars_red_bank::{
    contract::execute,
    error::ContractError,
    interest_rates::{
        compute_scaled_amount, compute_underlying_amount, get_scaled_liquidity_amount,
        get_updated_borrow_index, get_updated_liquidity_index, ScalingOperation, SCALING_FACTOR,
    },
    state::{COLLATERALS, DEBTS, MARKETS},
};
use mars_red_bank_types::{
    address_provider::MarsAddressType,
    incentives,
    red_bank::{Collateral, Debt, ExecuteMsg, Market},
};
use mars_testing::{mock_env_at_block_time, MarsMockQuerier};
use mars_utils::math;

use crate::helpers::th_default_asset_params;

mod helpers;

struct TestSuite {
    deps: OwnedDeps<MockStorage, MockApi, MarsMockQuerier>,
    denom: &'static str,
    withdrawer_addr: Addr,
    initial_market: Market,
}

fn setup_test() -> TestSuite {
    let denom = "uosmo";
    let initial_liquidity = Uint128::new(12_000_000);

    let mut deps = th_setup(&[coin(initial_liquidity.u128(), denom)]);

    let market = Market {
        denom: denom.to_string(),
        reserve_factor: Decimal::from_ratio(1u128, 10u128),
        borrow_index: Decimal::from_ratio(2u128, 1u128),
        liquidity_index: Decimal::from_ratio(15u128, 10u128),
        borrow_rate: Decimal::from_ratio(20u128, 100u128),
        liquidity_rate: Decimal::from_ratio(10u128, 100u128),
        indexes_last_updated: 10000000,
        collateral_total_scaled: Uint128::new(2_000_000) * SCALING_FACTOR,
        debt_total_scaled: Uint128::new(10_000_000) * SCALING_FACTOR,
        ..Default::default()
    };

    MARKETS.save(deps.as_mut().storage, denom, &market).unwrap();

    TestSuite {
        deps,
        denom,
        withdrawer_addr: Addr::unchecked("larry"),
        initial_market: market,
    }
}

#[test]
fn withdrawing_more_than_balance() {
    let TestSuite {
        mut deps,
        denom,
        withdrawer_addr,
        ..
    } = setup_test();

    // give withdrawer a small collateral position
    set_collateral(deps.as_mut(), &withdrawer_addr, denom, Uint128::new(200), false);

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(withdrawer_addr.as_str(), &[]),
        ExecuteMsg::Withdraw {
            denom: denom.to_string(),
            amount: Some(Uint128::from(2000u128)),
            recipient: None,
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidWithdrawAmount {
            denom: denom.to_string()
        }
    );
}

#[test]
fn withdrawing_partially() {
    let TestSuite {
        mut deps,
        denom,
        withdrawer_addr,
        initial_market,
    } = setup_test();

    let block_time = initial_market.indexes_last_updated + 2000;
    let withdraw_amount = Uint128::new(20_000);

    // create a collateral position for the user
    // for this test, we assume the user has NOT enabled the asset as collateral
    // the health factor check should have been skipped (no need to set mock oracle price)
    let initial_deposit_amount_scaled = initial_market.collateral_total_scaled;
    set_collateral(
        deps.as_mut(),
        &withdrawer_addr,
        &initial_market.denom,
        initial_deposit_amount_scaled,
        false,
    );

    let res = execute(
        deps.as_mut(),
        mock_env_at_block_time(block_time),
        mock_info(withdrawer_addr.as_str(), &[]),
        ExecuteMsg::Withdraw {
            denom: denom.to_string(),
            amount: Some(withdraw_amount),
            recipient: None,
        },
    )
    .unwrap();

    let market = MARKETS.load(deps.as_ref().storage, denom).unwrap();

    // compute expected market parameters
    let expected_params = th_get_expected_indices_and_rates(
        &initial_market,
        block_time,
        TestUtilizationDeltaInfo {
            less_liquidity: withdraw_amount,
            ..Default::default()
        },
    );

    let expected_deposit_balance = compute_underlying_amount(
        initial_deposit_amount_scaled,
        expected_params.liquidity_index,
        ScalingOperation::Truncate,
    )
    .unwrap();

    let expected_withdraw_amount_remaining = expected_deposit_balance - withdraw_amount;

    let expected_withdraw_amount_scaled_remaining = compute_scaled_amount(
        expected_withdraw_amount_remaining,
        expected_params.liquidity_index,
        ScalingOperation::Truncate,
    )
    .unwrap();

    let expected_burn_amount =
        initial_deposit_amount_scaled - expected_withdraw_amount_scaled_remaining;

    let expected_rewards_amount_scaled = compute_scaled_amount(
        expected_params.protocol_rewards_to_distribute,
        market.liquidity_index,
        ScalingOperation::Truncate,
    )
    .unwrap();

    let expected_total_collateral_amount_scaled = initial_market.collateral_total_scaled
        - expected_burn_amount
        + expected_rewards_amount_scaled;

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(WasmMsg::Execute {
                contract_addr: MarsAddressType::Incentives.to_string(),
                msg: to_binary(&incentives::ExecuteMsg::BalanceChange {
                    user_addr: Addr::unchecked(MarsAddressType::RewardsCollector.to_string()),
                    denom: denom.to_string(),
                    user_amount_scaled_before: Uint128::zero(),
                    total_amount_scaled_before: initial_market.collateral_total_scaled,
                })
                .unwrap(),
                funds: vec![],
            }),
            SubMsg::new(WasmMsg::Execute {
                contract_addr: MarsAddressType::Incentives.to_string(),
                msg: to_binary(&incentives::ExecuteMsg::BalanceChange {
                    user_addr: withdrawer_addr.clone(),
                    denom: denom.to_string(),
                    user_amount_scaled_before: initial_deposit_amount_scaled,
                    total_amount_scaled_before: initial_market.collateral_total_scaled
                        + expected_rewards_amount_scaled,
                })
                .unwrap(),
                funds: vec![],
            }),
            SubMsg::new(BankMsg::Send {
                to_address: withdrawer_addr.to_string(),
                amount: coins(withdraw_amount.u128(), denom)
            })
        ]
    );
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "withdraw"),
            attr("sender", &withdrawer_addr),
            attr("recipient", &withdrawer_addr),
            attr("denom", denom),
            attr("amount", withdraw_amount),
            attr("amount_scaled", expected_burn_amount),
        ]
    );
    assert_eq!(res.events, vec![th_build_interests_updated_event(denom, &expected_params)]);

    // market parameters should have been updated
    assert_eq!(market.borrow_index, expected_params.borrow_index);
    assert_eq!(market.liquidity_index, expected_params.liquidity_index);
    assert_eq!(market.borrow_rate, expected_params.borrow_rate);
    assert_eq!(market.liquidity_rate, expected_params.liquidity_rate);

    // the market's total collateral scaled amount should have been decreased
    assert_eq!(market.collateral_total_scaled, expected_total_collateral_amount_scaled);

    // the user's collateral scaled amount should have been decreased
    let collateral = COLLATERALS.load(deps.as_ref().storage, (&withdrawer_addr, denom)).unwrap();
    assert_eq!(collateral.amount_scaled, expected_withdraw_amount_scaled_remaining);

    // the reward collector's collateral scaled amount should have been increased
    let rewards_addr = Addr::unchecked(MarsAddressType::RewardsCollector.to_string());
    let collateral = COLLATERALS.load(deps.as_ref().storage, (&rewards_addr, denom)).unwrap();
    assert_eq!(collateral.amount_scaled, expected_rewards_amount_scaled);
}

#[test]
fn withdrawing_completely() {
    let TestSuite {
        mut deps,
        denom,
        withdrawer_addr,
        initial_market,
    } = setup_test();

    let block_time = initial_market.indexes_last_updated + 2000;

    // create a collateral position for the withdrawer
    let withdrawer_balance_scaled = Uint128::new(123_456) * SCALING_FACTOR;
    set_collateral(deps.as_mut(), &withdrawer_addr, denom, withdrawer_balance_scaled, true);

    let res = execute(
        deps.as_mut(),
        mock_env_at_block_time(block_time),
        mock_info(withdrawer_addr.as_str(), &[]),
        ExecuteMsg::Withdraw {
            denom: denom.to_string(),
            amount: None,
            recipient: None,
        },
    )
    .unwrap();

    let market = MARKETS.load(&deps.storage, denom).unwrap();

    let withdrawer_balance = compute_underlying_amount(
        withdrawer_balance_scaled,
        get_updated_liquidity_index(&initial_market, block_time).unwrap(),
        ScalingOperation::Truncate,
    )
    .unwrap();

    let expected_params = th_get_expected_indices_and_rates(
        &initial_market,
        block_time,
        TestUtilizationDeltaInfo {
            less_liquidity: withdrawer_balance,
            ..Default::default()
        },
    );

    let expected_rewards_amount_scaled = compute_scaled_amount(
        expected_params.protocol_rewards_to_distribute,
        market.liquidity_index,
        ScalingOperation::Truncate,
    )
    .unwrap();

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(WasmMsg::Execute {
                contract_addr: MarsAddressType::Incentives.to_string(),
                msg: to_binary(&incentives::ExecuteMsg::BalanceChange {
                    user_addr: Addr::unchecked(MarsAddressType::RewardsCollector.to_string()),
                    denom: denom.to_string(),
                    user_amount_scaled_before: Uint128::zero(),
                    total_amount_scaled_before: initial_market.collateral_total_scaled,
                })
                .unwrap(),
                funds: vec![],
            }),
            SubMsg::new(WasmMsg::Execute {
                contract_addr: MarsAddressType::Incentives.to_string(),
                msg: to_binary(&incentives::ExecuteMsg::BalanceChange {
                    user_addr: withdrawer_addr.clone(),
                    denom: denom.to_string(),
                    user_amount_scaled_before: withdrawer_balance_scaled,
                    total_amount_scaled_before: initial_market.collateral_total_scaled
                        + expected_rewards_amount_scaled,
                })
                .unwrap(),
                funds: vec![],
            }),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: withdrawer_addr.to_string(),
                amount: coins(withdrawer_balance.u128(), denom)
            })),
        ]
    );
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "withdraw"),
            attr("sender", &withdrawer_addr),
            attr("recipient", &withdrawer_addr),
            attr("denom", denom),
            attr("amount", withdrawer_balance.to_string()),
            attr("amount_scaled", withdrawer_balance_scaled.to_string()),
        ]
    );
    assert_eq!(res.events, vec![th_build_interests_updated_event(denom, &expected_params)]);

    assert_eq!(market.borrow_index, expected_params.borrow_index);
    assert_eq!(market.liquidity_index, expected_params.liquidity_index);
    assert_eq!(market.borrow_rate, expected_params.borrow_rate);
    assert_eq!(market.liquidity_rate, expected_params.liquidity_rate);

    // withdrawer's collateral position should have been deleted after full withdraw
    assert!(!has_collateral_position(deps.as_ref(), &withdrawer_addr, denom));
}

#[test]
fn withdrawing_to_another_user() {
    let TestSuite {
        mut deps,
        denom,
        withdrawer_addr,
        initial_market,
    } = setup_test();

    let block_time = initial_market.indexes_last_updated + 2000;
    let recipient_addr = Addr::unchecked("jake");

    // create a collateral position for the withdrawer
    let withdrawer_balance_scaled = Uint128::new(123_456) * SCALING_FACTOR;
    set_collateral(deps.as_mut(), &withdrawer_addr, denom, withdrawer_balance_scaled, true);

    let res = execute(
        deps.as_mut(),
        mock_env_at_block_time(block_time),
        mock_info(withdrawer_addr.as_str(), &[]),
        ExecuteMsg::Withdraw {
            denom: denom.to_string(),
            amount: None,
            recipient: Some(recipient_addr.to_string()),
        },
    )
    .unwrap();

    let market = MARKETS.load(deps.as_ref().storage, denom).unwrap();

    let withdraw_amount = compute_underlying_amount(
        withdrawer_balance_scaled,
        market.liquidity_index,
        ScalingOperation::Truncate,
    )
    .unwrap();

    let expected_params = th_get_expected_indices_and_rates(
        &initial_market,
        block_time,
        TestUtilizationDeltaInfo {
            less_liquidity: withdraw_amount,
            ..Default::default()
        },
    );

    let expected_rewards_amount_scaled = compute_scaled_amount(
        expected_params.protocol_rewards_to_distribute,
        market.liquidity_index,
        ScalingOperation::Truncate,
    )
    .unwrap();

    // check if the withdrew funds are properly sent to the designated recipient
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(WasmMsg::Execute {
                contract_addr: MarsAddressType::Incentives.to_string(),
                msg: to_binary(&incentives::ExecuteMsg::BalanceChange {
                    user_addr: Addr::unchecked(MarsAddressType::RewardsCollector.to_string()),
                    denom: denom.to_string(),
                    user_amount_scaled_before: Uint128::zero(),
                    total_amount_scaled_before: initial_market.collateral_total_scaled,
                })
                .unwrap(),
                funds: vec![],
            }),
            SubMsg::new(WasmMsg::Execute {
                contract_addr: MarsAddressType::Incentives.to_string(),
                msg: to_binary(&incentives::ExecuteMsg::BalanceChange {
                    user_addr: withdrawer_addr.clone(),
                    denom: denom.to_string(),
                    user_amount_scaled_before: withdrawer_balance_scaled,
                    total_amount_scaled_before: initial_market.collateral_total_scaled
                        + expected_rewards_amount_scaled,
                })
                .unwrap(),
                funds: vec![],
            }),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: recipient_addr.to_string(),
                amount: coins(withdraw_amount.u128(), denom)
            }))
        ]
    );
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "withdraw"),
            attr("sender", &withdrawer_addr),
            attr("recipient", &recipient_addr),
            attr("denom", denom.to_string()),
            attr("amount", withdraw_amount.to_string()),
            attr("amount_scaled", withdrawer_balance_scaled.to_string()),
        ]
    );

    // withdrawer's collateral position should have been deleted after full withdraw
    assert!(!has_collateral_position(deps.as_ref(), &withdrawer_addr, denom));
}

struct HealthCheckTestSuite {
    deps: OwnedDeps<MockStorage, MockApi, MarsMockQuerier>,
    denoms: [&'static str; 3],
    markets: [Market; 3],
    asset_params: [AssetParams; 3],
    prices: [Decimal; 3],
    collaterals: [Collateral; 3],
    debts: [Debt; 3],
    withdrawer_addr: Addr,
}

fn setup_health_check_test() -> HealthCheckTestSuite {
    let denoms = ["uatom", "uosmo", "umars"];
    let initial_liquidity = Uint128::from(10000000u128);

    let mut deps = th_setup(&[coin(initial_liquidity.into(), denoms[2])]);

    let withdrawer_addr = Addr::unchecked("withdrawer");

    let markets = [
        Market {
            denom: denoms[0].to_string(),
            liquidity_index: Decimal::one(),
            borrow_index: Decimal::one(),
            collateral_total_scaled: Uint128::new(100_000) * SCALING_FACTOR,
            ..Default::default()
        },
        Market {
            denom: denoms[1].to_string(),
            liquidity_index: Decimal::one(),
            borrow_index: Decimal::one(),
            collateral_total_scaled: Uint128::new(100_000) * SCALING_FACTOR,
            ..Default::default()
        },
        Market {
            denom: denoms[2].to_string(),
            liquidity_index: Decimal::one(),
            borrow_index: Decimal::one(),
            collateral_total_scaled: Uint128::new(100_000) * SCALING_FACTOR,
            ..Default::default()
        },
    ];

    let asset_params = [
        AssetParams {
            max_loan_to_value: Decimal::from_ratio(40u128, 100u128),
            liquidation_threshold: Decimal::from_ratio(60u128, 100u128),
            ..th_default_asset_params()
        },
        AssetParams {
            max_loan_to_value: Decimal::from_ratio(50u128, 100u128),
            liquidation_threshold: Decimal::from_ratio(80u128, 100u128),
            ..th_default_asset_params()
        },
        AssetParams {
            max_loan_to_value: Decimal::from_ratio(20u128, 100u128),
            liquidation_threshold: Decimal::from_ratio(40u128, 100u128),
            ..th_default_asset_params()
        },
    ];

    let prices = [
        Decimal::from_ratio(3u128, 1u128),
        Decimal::from_ratio(2u128, 1u128),
        Decimal::from_ratio(1u128, 1u128),
    ];

    let collaterals = [
        Collateral {
            amount_scaled: Uint128::new(100_000) * SCALING_FACTOR,
            enabled: true,
        },
        Collateral {
            amount_scaled: Uint128::zero(),
            enabled: false,
        },
        Collateral {
            amount_scaled: Uint128::new(600_000) * SCALING_FACTOR,
            enabled: true,
        },
    ];

    let debts = [
        Debt {
            amount_scaled: Uint128::zero(),
            uncollateralized: false,
        },
        Debt {
            amount_scaled: Uint128::new(200_000) * SCALING_FACTOR,
            uncollateralized: false,
        },
        Debt {
            amount_scaled: Uint128::new(200_000) * SCALING_FACTOR,
            uncollateralized: true,
        },
    ];

    denoms
        .iter()
        .zip(markets.iter())
        .try_for_each(|(denom, market)| MARKETS.save(deps.as_mut().storage, denom, market))
        .unwrap();

    denoms
        .iter()
        .zip(asset_params.iter())
        .for_each(|(denom, ap)| deps.querier.set_redbank_params(denom, ap.clone()));

    denoms
        .iter()
        .zip(prices.iter())
        .for_each(|(denom, price)| deps.querier.set_oracle_price(denom, *price));

    denoms.iter().zip(collaterals.iter()).for_each(|(denom, collateral)| {
        if !collateral.amount_scaled.is_zero() {
            COLLATERALS.save(deps.as_mut().storage, (&withdrawer_addr, denom), collateral).unwrap();
        }
    });

    denoms.iter().zip(debts.iter()).for_each(|(denom, debt)| {
        if !debt.amount_scaled.is_zero() {
            DEBTS.save(deps.as_mut().storage, (&withdrawer_addr, denom), debt).unwrap();
        }
    });

    HealthCheckTestSuite {
        deps,
        denoms,
        markets,
        asset_params,
        prices,
        collaterals,
        debts,
        withdrawer_addr,
    }
}

/// Calculate how much to withdraw to have health factor equal to one
fn how_much_to_withdraw(suite: &HealthCheckTestSuite, block_time: u64) -> Uint128 {
    let HealthCheckTestSuite {
        markets,
        asset_params,
        prices,
        collaterals,
        debts,
        ..
    } = suite;

    let token_1_weighted_lt_in_base_asset = compute_underlying_amount(
        collaterals[0].amount_scaled,
        get_updated_liquidity_index(&markets[0], block_time).unwrap(),
        ScalingOperation::Truncate,
    )
    .unwrap()
        * asset_params[0].liquidation_threshold
        * prices[0];

    let token_3_weighted_lt_in_base_asset = compute_underlying_amount(
        collaterals[2].amount_scaled,
        get_updated_liquidity_index(&markets[2], block_time).unwrap(),
        ScalingOperation::Truncate,
    )
    .unwrap()
        * asset_params[2].liquidation_threshold
        * prices[2];

    let weighted_liquidation_threshold_in_base_asset =
        token_1_weighted_lt_in_base_asset + token_3_weighted_lt_in_base_asset;

    let total_collateralized_debt_in_base_asset = compute_underlying_amount(
        debts[1].amount_scaled,
        get_updated_borrow_index(&markets[1], block_time).unwrap(),
        ScalingOperation::Ceil,
    )
    .unwrap()
        * prices[1];

    // How much to withdraw in base asset to have health factor equal to one
    let how_much_to_withdraw_in_base_asset = math::divide_uint128_by_decimal(
        weighted_liquidation_threshold_in_base_asset - total_collateralized_debt_in_base_asset,
        asset_params[2].liquidation_threshold,
    )
    .unwrap();

    math::divide_uint128_by_decimal(how_much_to_withdraw_in_base_asset, prices[2]).unwrap()
}

#[test]
fn withdrawing_if_health_factor_not_met() {
    let suite = setup_health_check_test();

    let env = mock_env();
    let block_time = env.block.time.seconds();

    let max_withdraw_amount = how_much_to_withdraw(&suite, block_time);

    let HealthCheckTestSuite {
        mut deps,
        denoms,
        withdrawer_addr,
        ..
    } = suite;

    // withdraw token3 with failure
    // the withdraw amount needs to be a little bit greater to have health factor less than one
    let withdraw_amount = max_withdraw_amount + Uint128::from(10u128);

    let err = execute(
        deps.as_mut(),
        env,
        mock_info(withdrawer_addr.as_str(), &[]),
        ExecuteMsg::Withdraw {
            denom: denoms[2].to_string(),
            amount: Some(withdraw_amount),
            recipient: None,
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::InvalidHealthFactorAfterWithdraw {});
}

#[test]
fn withdrawing_if_health_factor_met() {
    let suite = setup_health_check_test();

    let env = mock_env();
    let block_time = env.block.time.seconds();

    let max_withdraw_amount = how_much_to_withdraw(&suite, block_time);

    let HealthCheckTestSuite {
        mut deps,
        denoms,
        markets,
        collaterals,
        withdrawer_addr,
        ..
    } = suite;

    // withdraw token3 with success
    // the withdraw amount needs to be a little bit smaller to have health factor greater than one
    let withdraw_amount = max_withdraw_amount - Uint128::from(10u128);

    let res = execute(
        deps.as_mut(),
        env,
        mock_info(withdrawer_addr.as_str(), &[]),
        ExecuteMsg::Withdraw {
            denom: denoms[2].to_string(),
            amount: Some(withdraw_amount),
            recipient: None,
        },
    )
    .unwrap();

    // NOTE: For this particular test, we have set the borrow interest rate at zero, so there no
    // protocol reward accrued, and hence no message to update the reward collector's index at the
    // incentives contract.
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(WasmMsg::Execute {
                contract_addr: MarsAddressType::Incentives.to_string(),
                msg: to_binary(&incentives::ExecuteMsg::BalanceChange {
                    user_addr: withdrawer_addr.clone(),
                    denom: denoms[2].to_string(),
                    user_amount_scaled_before: collaterals[2].amount_scaled,
                    // NOTE: Protocol rewards accrued is zero, so here it's initial total supply
                    total_amount_scaled_before: markets[2].collateral_total_scaled,
                })
                .unwrap(),
                funds: vec![],
            }),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: withdrawer_addr.to_string(),
                amount: coins(withdraw_amount.u128(), denoms[2])
            }))
        ],
    );

    let expected_withdraw_amount_scaled =
        get_scaled_liquidity_amount(withdraw_amount, &markets[2], block_time).unwrap();
    let expected_withdrawer_balance_after =
        collaterals[2].amount_scaled - expected_withdraw_amount_scaled;
    let expected_collateral_total_amount_scaled_after =
        markets[2].collateral_total_scaled - expected_withdraw_amount_scaled;

    let col = COLLATERALS.load(deps.as_ref().storage, (&withdrawer_addr, denoms[2])).unwrap();
    assert_eq!(col.amount_scaled, expected_withdrawer_balance_after);

    let market = MARKETS.load(deps.as_ref().storage, denoms[2]).unwrap();
    assert_eq!(market.collateral_total_scaled, expected_collateral_total_amount_scaled_after);
}
