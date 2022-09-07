use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{attr, coin, coins, Addr, BankMsg, CosmosMsg, Decimal, SubMsg, Uint128};

use cw_utils::PaymentError;
use mars_outpost::math;
use mars_outpost::red_bank::{ExecuteMsg, Market, User};
use mars_testing::{mock_env, mock_env_at_block_time, MockEnvParams};

use mars_red_bank::contract::execute;
use mars_red_bank::error::ContractError;
use mars_red_bank::events::build_debt_position_changed_event;
use mars_red_bank::helpers::{get_bit, set_bit};
use mars_red_bank::interest_rates::{
    calculate_applied_linear_interest_rate, compute_scaled_amount, compute_underlying_amount,
    ScalingOperation, SCALING_FACTOR,
};
use mars_red_bank::state::{DEBTS, MARKETS, UNCOLLATERALIZED_LOAN_LIMITS, USERS};

use helpers::{
    th_build_interests_updated_event, th_get_expected_indices_and_rates, th_init_market, th_setup,
    TestUtilizationDeltaInfo,
};

mod helpers;

#[test]
fn test_borrow_and_repay() {
    // consider three assets: uatom, uosmo, uusd
    // the user deposits uatom collateral, and borrow uosmo, uusd loans
    //
    // NOTE: available liquidity stays fixed as the test environment does not get changes in
    // contract balances on subsequent calls. They would change from call to call in practice
    let available_liquidity_uosmo = Uint128::new(1_000_000_000);
    let available_liquidity_uusd = Uint128::new(2_000_000_000);

    let mut deps = th_setup(&[
        coin(available_liquidity_uosmo.u128(), "uosmo"),
        coin(available_liquidity_uusd.u128(), "uusd"),
    ]);

    deps.querier.set_oracle_price("uatom", Decimal::one());
    deps.querier.set_oracle_price("uosmo", Decimal::one());
    deps.querier.set_oracle_price("uusd", Decimal::one());

    let mock_market_1 = Market {
        ma_token_address: Addr::unchecked("ma-uosmo"),
        borrow_index: Decimal::from_ratio(12u128, 10u128),
        liquidity_index: Decimal::from_ratio(8u128, 10u128),
        borrow_rate: Decimal::from_ratio(20u128, 100u128),
        liquidity_rate: Decimal::from_ratio(10u128, 100u128),
        reserve_factor: Decimal::from_ratio(1u128, 100u128),
        debt_total_scaled: Uint128::zero(),
        indexes_last_updated: 10000000,
        ..Default::default()
    };
    let mock_market_2 = Market {
        ma_token_address: Addr::unchecked("ma-uusd"),
        borrow_index: Decimal::one(),
        liquidity_index: Decimal::one(),
        ..Default::default()
    };
    let mock_market_3 = Market {
        ma_token_address: Addr::unchecked("ma-uatom"),
        borrow_index: Decimal::one(),
        liquidity_index: Decimal::from_ratio(11u128, 10u128),
        max_loan_to_value: Decimal::from_ratio(7u128, 10u128),
        borrow_rate: Decimal::from_ratio(30u128, 100u128),
        reserve_factor: Decimal::from_ratio(3u128, 100u128),
        liquidity_rate: Decimal::from_ratio(20u128, 100u128),
        debt_total_scaled: Uint128::zero(),
        indexes_last_updated: 10000000,
        ..Default::default()
    };

    // should get index 0
    let market_1_initial = th_init_market(deps.as_mut(), "uosmo", &mock_market_1);
    // should get index 1
    let market_2_initial = th_init_market(deps.as_mut(), "uusd", &mock_market_2);
    // should get index 2
    let market_collateral = th_init_market(deps.as_mut(), "uatom", &mock_market_3);

    let borrower_addr = Addr::unchecked("borrower");

    // Set user as having the market_collateral deposited
    let mut user = User::default();

    set_bit(&mut user.collateral_assets, market_collateral.index).unwrap();
    USERS.save(deps.as_mut().storage, &borrower_addr, &user).unwrap();

    // Set the querier to return a certain collateral balance
    let deposit_coin_address = Addr::unchecked("ma-uatom");
    deps.querier.set_cw20_balances(
        deposit_coin_address,
        &[(borrower_addr.clone(), Uint128::new(10000) * SCALING_FACTOR)],
    );

    // *
    // Borrow uosmo
    // *
    let block_time = mock_market_1.indexes_last_updated + 10000u64;
    let borrow_amount = Uint128::from(2400u128);

    let msg = ExecuteMsg::Borrow {
        denom: "uosmo".to_string(),
        amount: borrow_amount,
        recipient: None,
    };

    let env = mock_env_at_block_time(block_time);
    let info = mock_info("borrower", &[]);

    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    let expected_params_uosmo = th_get_expected_indices_and_rates(
        &market_1_initial,
        block_time,
        available_liquidity_uosmo,
        TestUtilizationDeltaInfo {
            less_liquidity: borrow_amount,
            more_debt: borrow_amount,
            ..Default::default()
        },
    );

    // check correct messages and logging
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: borrower_addr.to_string(),
            amount: coins(borrow_amount.u128(), "uosmo")
        }))]
    );
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "outposts/red-bank/borrow"),
            attr("denom", "uosmo"),
            attr("user", "borrower"),
            attr("recipient", "borrower"),
            attr("amount", borrow_amount.to_string()),
        ]
    );
    assert_eq!(
        res.events,
        vec![
            build_debt_position_changed_event("uosmo", true, "borrower".to_string()),
            th_build_interests_updated_event("uosmo", &expected_params_uosmo)
        ]
    );

    let user = USERS.load(&deps.storage, &borrower_addr).unwrap();
    assert!(get_bit(user.borrowed_assets, 0).unwrap());
    assert!(!get_bit(user.borrowed_assets, 1).unwrap());

    let debt = DEBTS.load(&deps.storage, (&borrower_addr, "uosmo")).unwrap();
    let expected_debt_scaled_1_after_borrow = compute_scaled_amount(
        borrow_amount,
        expected_params_uosmo.borrow_index,
        ScalingOperation::Ceil,
    )
    .unwrap();

    let market_1_after_borrow = MARKETS.load(&deps.storage, "uosmo").unwrap();

    assert_eq!(expected_debt_scaled_1_after_borrow, debt.amount_scaled);
    assert_eq!(expected_debt_scaled_1_after_borrow, market_1_after_borrow.debt_total_scaled);
    assert_eq!(expected_params_uosmo.borrow_rate, market_1_after_borrow.borrow_rate);
    assert_eq!(expected_params_uosmo.liquidity_rate, market_1_after_borrow.liquidity_rate);

    // *
    // Borrow uosmo (again)
    // *
    let borrow_amount = Uint128::from(1200u128);
    let block_time = market_1_after_borrow.indexes_last_updated + 20000u64;

    let msg = ExecuteMsg::Borrow {
        denom: "uosmo".to_string(),
        amount: borrow_amount,
        recipient: None,
    };

    let env = mock_env_at_block_time(block_time);
    let info = mock_info("borrower", &[]);

    execute(deps.as_mut(), env, info, msg).unwrap();

    let user = USERS.load(&deps.storage, &borrower_addr).unwrap();
    assert!(get_bit(user.borrowed_assets, 0).unwrap());
    assert!(!get_bit(user.borrowed_assets, 1).unwrap());

    let expected_params_uosmo = th_get_expected_indices_and_rates(
        &market_1_after_borrow,
        block_time,
        available_liquidity_uosmo,
        TestUtilizationDeltaInfo {
            less_liquidity: borrow_amount,
            more_debt: borrow_amount,
            ..Default::default()
        },
    );
    let debt = DEBTS.load(&deps.storage, (&borrower_addr, "uosmo")).unwrap();
    let market_1_after_borrow_again = MARKETS.load(&deps.storage, "uosmo").unwrap();

    let expected_debt_scaled_1_after_borrow_again = expected_debt_scaled_1_after_borrow
        + compute_scaled_amount(
            borrow_amount,
            expected_params_uosmo.borrow_index,
            ScalingOperation::Ceil,
        )
        .unwrap();
    assert_eq!(expected_debt_scaled_1_after_borrow_again, debt.amount_scaled);
    assert_eq!(
        expected_debt_scaled_1_after_borrow_again,
        market_1_after_borrow_again.debt_total_scaled
    );
    assert_eq!(expected_params_uosmo.borrow_rate, market_1_after_borrow_again.borrow_rate);
    assert_eq!(expected_params_uosmo.liquidity_rate, market_1_after_borrow_again.liquidity_rate);

    // *
    // Borrow uusd
    // *

    let borrow_amount = Uint128::from(4000u128);
    let block_time = market_1_after_borrow_again.indexes_last_updated + 3000u64;
    let env = mock_env_at_block_time(block_time);
    let info = mock_info("borrower", &[]);
    let msg = ExecuteMsg::Borrow {
        denom: String::from("uusd"),
        amount: borrow_amount,
        recipient: None,
    };
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    let user = USERS.load(&deps.storage, &borrower_addr).unwrap();
    assert!(get_bit(user.borrowed_assets, 0).unwrap());
    assert!(get_bit(user.borrowed_assets, 1).unwrap());

    let expected_params_uusd = th_get_expected_indices_and_rates(
        &market_2_initial,
        block_time,
        available_liquidity_uusd,
        TestUtilizationDeltaInfo {
            less_liquidity: borrow_amount,
            more_debt: borrow_amount,
            ..Default::default()
        },
    );

    // check correct messages and logging
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "borrower".to_string(),
            amount: coins(borrow_amount.u128(), "uusd")
        }))]
    );
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "outposts/red-bank/borrow"),
            attr("denom", "uusd"),
            attr("user", "borrower"),
            attr("recipient", "borrower"),
            attr("amount", borrow_amount.to_string()),
        ]
    );
    assert_eq!(
        res.events,
        vec![
            build_debt_position_changed_event("uusd", true, "borrower".to_string()),
            th_build_interests_updated_event("uusd", &expected_params_uusd)
        ]
    );

    let debt2 = DEBTS.load(&deps.storage, (&borrower_addr, "uusd")).unwrap();
    let market_2_after_borrow_2 = MARKETS.load(&deps.storage, "uusd").unwrap();

    let expected_debt_scaled_2_after_borrow_2 = compute_scaled_amount(
        borrow_amount,
        expected_params_uusd.borrow_index,
        ScalingOperation::Ceil,
    )
    .unwrap();
    assert_eq!(expected_debt_scaled_2_after_borrow_2, debt2.amount_scaled);
    assert_eq!(expected_debt_scaled_2_after_borrow_2, market_2_after_borrow_2.debt_total_scaled);
    assert_eq!(expected_params_uusd.borrow_rate, market_2_after_borrow_2.borrow_rate);
    assert_eq!(expected_params_uusd.liquidity_rate, market_2_after_borrow_2.liquidity_rate);

    // *
    // Borrow native coin again (should fail due to insufficient collateral)
    // *
    let env = mock_env(MockEnvParams::default());
    let info = mock_info("borrower", &[]);
    let msg = ExecuteMsg::Borrow {
        denom: String::from("uusd"),
        amount: Uint128::from(83968_u128),
        recipient: None,
    };
    let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(error_res, ContractError::BorrowAmountExceedsGivenCollateral {});

    // *
    // Repay zero uusd debt (should fail)
    // *
    let env = mock_env_at_block_time(block_time);
    let info = mock_info("borrower", &[]);
    let msg = ExecuteMsg::Repay {
        on_behalf_of: None,
    };
    let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(error_res, PaymentError::NoFunds {}.into());

    // *
    // Repay some uusd debt
    // *
    let repay_amount = Uint128::from(2000u128);
    let block_time = market_2_after_borrow_2.indexes_last_updated + 8000u64;
    let env = mock_env_at_block_time(block_time);
    let info = cosmwasm_std::testing::mock_info("borrower", &[coin(repay_amount.into(), "uusd")]);
    let msg = ExecuteMsg::Repay {
        on_behalf_of: None,
    };
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    let expected_params_uusd = th_get_expected_indices_and_rates(
        &market_2_after_borrow_2,
        block_time,
        available_liquidity_uusd,
        TestUtilizationDeltaInfo {
            less_debt: repay_amount,
            user_current_debt_scaled: expected_debt_scaled_2_after_borrow_2,
            ..Default::default()
        },
    );

    assert_eq!(res.messages, vec![]);
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "outposts/red-bank/repay"),
            attr("denom", "uusd"),
            attr("sender", "borrower"),
            attr("user", "borrower"),
            attr("amount", repay_amount.to_string()),
        ]
    );
    assert_eq!(res.events, vec![th_build_interests_updated_event("uusd", &expected_params_uusd)]);

    let user = USERS.load(&deps.storage, &borrower_addr).unwrap();
    assert!(get_bit(user.borrowed_assets, 0).unwrap());
    assert!(get_bit(user.borrowed_assets, 1).unwrap());

    let debt2 = DEBTS.load(&deps.storage, (&borrower_addr, "uusd")).unwrap();
    let market_2_after_repay_some_2 = MARKETS.load(&deps.storage, "uusd").unwrap();

    let expected_debt_scaled_2_after_repay_some_2 = expected_debt_scaled_2_after_borrow_2
        - compute_scaled_amount(
            repay_amount,
            expected_params_uusd.borrow_index,
            ScalingOperation::Ceil,
        )
        .unwrap();
    assert_eq!(expected_debt_scaled_2_after_repay_some_2, debt2.amount_scaled);
    assert_eq!(
        expected_debt_scaled_2_after_repay_some_2,
        market_2_after_repay_some_2.debt_total_scaled
    );
    assert_eq!(expected_params_uusd.borrow_rate, market_2_after_repay_some_2.borrow_rate);
    assert_eq!(expected_params_uusd.liquidity_rate, market_2_after_repay_some_2.liquidity_rate);

    // *
    // Repay all uusd debt
    // *
    let block_time = market_2_after_repay_some_2.indexes_last_updated + 10000u64;
    // need this to compute the repay amount
    let expected_params_uusd = th_get_expected_indices_and_rates(
        &market_2_after_repay_some_2,
        block_time,
        available_liquidity_uusd,
        TestUtilizationDeltaInfo {
            less_debt: Uint128::from(9999999999999_u128), // hack: Just do a big number to repay all debt,
            user_current_debt_scaled: expected_debt_scaled_2_after_repay_some_2,
            ..Default::default()
        },
    );

    let repay_amount: u128 = compute_underlying_amount(
        expected_debt_scaled_2_after_repay_some_2,
        expected_params_uusd.borrow_index,
        ScalingOperation::Ceil,
    )
    .unwrap()
    .into();

    let env = mock_env_at_block_time(block_time);
    let info = cosmwasm_std::testing::mock_info("borrower", &[coin(repay_amount, "uusd")]);
    let msg = ExecuteMsg::Repay {
        on_behalf_of: None,
    };
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "outposts/red-bank/repay"),
            attr("denom", "uusd"),
            attr("sender", "borrower"),
            attr("user", "borrower"),
            attr("amount", repay_amount.to_string()),
        ]
    );
    assert_eq!(
        res.events,
        vec![
            th_build_interests_updated_event("uusd", &expected_params_uusd),
            build_debt_position_changed_event("uusd", false, "borrower".to_string()),
        ]
    );

    let user = USERS.load(&deps.storage, &borrower_addr).unwrap();
    assert!(get_bit(user.borrowed_assets, 0).unwrap());
    assert!(!get_bit(user.borrowed_assets, 1).unwrap());

    let debt2 = DEBTS.load(&deps.storage, (&borrower_addr, "uusd")).unwrap();
    let market_2_after_repay_all_2 = MARKETS.load(&deps.storage, "uusd").unwrap();

    assert_eq!(Uint128::zero(), debt2.amount_scaled);
    assert_eq!(Uint128::zero(), market_2_after_repay_all_2.debt_total_scaled);

    // *
    // Repay more uusd debt (should fail)
    // *
    let env = mock_env(MockEnvParams::default());
    let info = cosmwasm_std::testing::mock_info("borrower", &[coin(2000, "uusd")]);
    let msg = ExecuteMsg::Repay {
        on_behalf_of: None,
    };
    let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(error_res, ContractError::CannotRepayZeroDebt {});

    // *
    // Repay all uosmo debt (and then some)
    // *
    let block_time = market_2_after_repay_all_2.indexes_last_updated + 5000u64;
    let repay_amount = Uint128::from(4800u128);

    let expected_params_uosmo = th_get_expected_indices_and_rates(
        &market_1_after_borrow_again,
        block_time,
        available_liquidity_uosmo,
        TestUtilizationDeltaInfo {
            less_debt: repay_amount,
            user_current_debt_scaled: expected_debt_scaled_1_after_borrow_again,
            ..Default::default()
        },
    );

    let env = mock_env_at_block_time(block_time);
    let info = cosmwasm_std::testing::mock_info("borrower", &[coin(repay_amount.u128(), "uosmo")]);
    let msg = ExecuteMsg::Repay {
        on_behalf_of: None,
    };
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    let expected_refund_amount = repay_amount
        - compute_underlying_amount(
            expected_debt_scaled_1_after_borrow_again,
            expected_params_uosmo.borrow_index,
            ScalingOperation::Ceil,
        )
        .unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: borrower_addr.to_string(),
            amount: coins(expected_refund_amount.u128(), "uosmo")
        }))]
    );
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "outposts/red-bank/repay"),
            attr("denom", "uosmo"),
            attr("sender", "borrower"),
            attr("user", "borrower"),
            attr("amount", (repay_amount - expected_refund_amount).to_string()),
        ]
    );
    assert_eq!(
        res.events,
        vec![
            th_build_interests_updated_event("uosmo", &expected_params_uosmo),
            build_debt_position_changed_event("uosmo", false, "borrower".to_string()),
        ]
    );
    let user = USERS.load(&deps.storage, &borrower_addr).unwrap();
    assert!(!get_bit(user.borrowed_assets, 0).unwrap());
    assert!(!get_bit(user.borrowed_assets, 1).unwrap());

    let debt1 = DEBTS.load(&deps.storage, (&borrower_addr, "uosmo")).unwrap();
    let market_1_after_repay_1 = MARKETS.load(&deps.storage, "uosmo").unwrap();
    assert_eq!(Uint128::zero(), debt1.amount_scaled);
    assert_eq!(Uint128::zero(), market_1_after_repay_1.debt_total_scaled);
}

#[test]
fn test_repay_on_behalf_of() {
    let available_liquidity_native = Uint128::from(1000000000u128);
    let mut deps = th_setup(&[coin(available_liquidity_native.into(), "borrowedcoinnative")]);

    deps.querier.set_oracle_price("depositedcoinnative", Decimal::one());
    deps.querier.set_oracle_price("borrowedcoinnative", Decimal::one());

    let mock_market_1 = Market {
        ma_token_address: Addr::unchecked("matoken1"),
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::one(),
        max_loan_to_value: Decimal::from_ratio(50u128, 100u128),
        ..Default::default()
    };
    let mock_market_2 = Market {
        ma_token_address: Addr::unchecked("matoken2"),
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::one(),
        max_loan_to_value: Decimal::from_ratio(50u128, 100u128),
        ..Default::default()
    };

    let market_1_initial = th_init_market(deps.as_mut(), "depositedcoinnative", &mock_market_1); // collateral
    let market_2_initial = th_init_market(deps.as_mut(), "borrowedcoinnative", &mock_market_2);

    let borrower_addr = Addr::unchecked("borrower");
    let user_addr = Addr::unchecked("user");

    // Set user as having the market_1_initial (collateral) deposited
    let mut user = User::default();

    set_bit(&mut user.collateral_assets, market_1_initial.index).unwrap();
    USERS.save(deps.as_mut().storage, &borrower_addr, &user).unwrap();

    // Set the querier to return a certain collateral balance
    let deposit_coin_address = Addr::unchecked("matoken1");
    deps.querier.set_cw20_balances(
        deposit_coin_address,
        &[(borrower_addr.clone(), Uint128::new(10000) * SCALING_FACTOR)],
    );

    // *
    // 'borrower' borrows native coin
    // *
    let borrow_amount = 4000u128;
    let env = mock_env(MockEnvParams::default());
    let info = mock_info(borrower_addr.as_str(), &[]);
    let msg = ExecuteMsg::Borrow {
        denom: String::from("borrowedcoinnative"),
        amount: Uint128::from(borrow_amount),
        recipient: None,
    };
    let _res = execute(deps.as_mut(), env, info, msg).unwrap();

    let user = USERS.load(&deps.storage, &borrower_addr).unwrap();
    assert!(get_bit(user.borrowed_assets, market_2_initial.index).unwrap());

    // *
    // 'user' repays debt on behalf of 'borrower'
    // *
    let repay_amount = borrow_amount;
    let env = mock_env(MockEnvParams::default());
    let info = cosmwasm_std::testing::mock_info(
        user_addr.as_str(),
        &[coin(repay_amount, "borrowedcoinnative")],
    );
    let msg = ExecuteMsg::Repay {
        on_behalf_of: Some(borrower_addr.to_string()),
    };
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    // 'user' should not be saved
    let _user = USERS.load(&deps.storage, &user_addr).unwrap_err();

    // Debt for 'user' should not exist
    let debt = DEBTS.may_load(&deps.storage, (&user_addr, "borrowedcoinnative")).unwrap();
    assert!(debt.is_none());

    // Debt for 'borrower' should be repayed
    let debt = DEBTS.load(&deps.storage, (&borrower_addr, "borrowedcoinnative")).unwrap();
    assert_eq!(debt.amount_scaled, Uint128::zero());

    // 'borrower' should have unset bit for debt after full repay
    let user = USERS.load(&deps.storage, &borrower_addr).unwrap();
    assert!(!get_bit(user.borrowed_assets, market_2_initial.index).unwrap());

    // Check msgs and attributes
    assert_eq!(res.messages, vec![]);
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "outposts/red-bank/repay"),
            attr("denom", "borrowedcoinnative"),
            attr("sender", "user"),
            attr("user", "borrower"),
            attr("amount", repay_amount.to_string()),
        ]
    );
}

#[test]
fn test_repay_uncollateralized_loan_on_behalf_of() {
    let mut deps = th_setup(&[]);

    let repayer_addr = Addr::unchecked("repayer");
    let another_user_addr = Addr::unchecked("another_user");

    UNCOLLATERALIZED_LOAN_LIMITS
        .save(deps.as_mut().storage, (&another_user_addr, "somecoin"), &Uint128::new(1000u128))
        .unwrap();

    let env = mock_env(MockEnvParams::default());
    let info = cosmwasm_std::testing::mock_info(repayer_addr.as_str(), &[coin(110000, "somecoin")]);
    let msg = ExecuteMsg::Repay {
        on_behalf_of: Some(another_user_addr.to_string()),
    };
    let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(error_res, ContractError::CannotRepayUncollateralizedLoanOnBehalfOf {});
}

#[test]
fn test_borrow_uusd() {
    let initial_liquidity = 10000000;
    let mut deps = th_setup(&[coin(initial_liquidity, "uusd")]);
    let block_time = 1;

    let borrower_addr = Addr::unchecked("borrower");
    let ltv = Decimal::from_ratio(7u128, 10u128);

    let mock_market = Market {
        ma_token_address: Addr::unchecked("matoken"),
        liquidity_index: Decimal::one(),
        max_loan_to_value: ltv,
        borrow_index: Decimal::from_ratio(20u128, 10u128),
        borrow_rate: Decimal::one(),
        liquidity_rate: Decimal::one(),
        debt_total_scaled: Uint128::zero(),
        indexes_last_updated: block_time,
        ..Default::default()
    };
    let market = th_init_market(deps.as_mut(), "uusd", &mock_market);

    // Set user as having the market_collateral deposited
    let deposit_amount_scaled = Uint128::new(110_000) * SCALING_FACTOR;
    let mut user = User::default();
    set_bit(&mut user.collateral_assets, market.index).unwrap();
    USERS.save(deps.as_mut().storage, &borrower_addr, &user).unwrap();

    // Set the querier to return collateral balance
    let deposit_coin_address = Addr::unchecked("matoken");
    deps.querier
        .set_cw20_balances(deposit_coin_address, &[(borrower_addr.clone(), deposit_amount_scaled)]);

    // borrow with insufficient collateral, should fail
    let new_block_time = 120u64;
    let time_elapsed = new_block_time - market.indexes_last_updated;
    let liquidity_index = calculate_applied_linear_interest_rate(
        market.liquidity_index,
        market.liquidity_rate,
        time_elapsed,
    )
    .unwrap();
    let collateral = compute_underlying_amount(
        deposit_amount_scaled,
        liquidity_index,
        ScalingOperation::Truncate,
    )
    .unwrap();
    let max_to_borrow = collateral * ltv;
    let msg = ExecuteMsg::Borrow {
        denom: "uusd".to_string(),
        amount: max_to_borrow + Uint128::from(1u128),
        recipient: None,
    };
    let env = mock_env_at_block_time(new_block_time);
    let info = mock_info("borrower", &[]);
    let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(error_res, ContractError::BorrowAmountExceedsGivenCollateral {});

    let valid_amount = max_to_borrow - Uint128::from(1000u128);
    let msg = ExecuteMsg::Borrow {
        denom: "uusd".to_string(),
        amount: valid_amount,
        recipient: None,
    };
    let env = mock_env_at_block_time(block_time);
    let info = mock_info("borrower", &[]);
    execute(deps.as_mut(), env, info, msg).unwrap();

    let market_after_borrow = MARKETS.load(&deps.storage, "uusd").unwrap();

    let user = USERS.load(&deps.storage, &borrower_addr).unwrap();
    assert!(get_bit(user.borrowed_assets, 0).unwrap());

    let debt = DEBTS.load(&deps.storage, (&borrower_addr, "uusd")).unwrap();

    assert_eq!(
        valid_amount,
        compute_underlying_amount(
            debt.amount_scaled,
            market_after_borrow.borrow_index,
            ScalingOperation::Ceil
        )
        .unwrap()
    );
}

#[test]
fn test_borrow_full_liquidity_and_then_repay() {
    let initial_liquidity = 50000;
    let mut deps = th_setup(&[coin(initial_liquidity, "uusd")]);
    let info = mock_info("borrower", &[]);
    let borrower_addr = Addr::unchecked("borrower");
    let block_time = 1;
    let ltv = Decimal::one();

    let mock_market = Market {
        ma_token_address: Addr::unchecked("matoken"),
        liquidity_index: Decimal::one(),
        max_loan_to_value: ltv,
        borrow_index: Decimal::one(),
        borrow_rate: Decimal::one(),
        liquidity_rate: Decimal::one(),
        debt_total_scaled: Uint128::zero(),
        reserve_factor: Decimal::from_ratio(12u128, 100u128),
        indexes_last_updated: block_time,
        ..Default::default()
    };
    let market = th_init_market(deps.as_mut(), "uusd", &mock_market);

    // User should have amount of collateral more than initial liquidity in order to borrow full liquidity
    let deposit_amount = initial_liquidity + 1000u128;
    let mut user = User::default();
    set_bit(&mut user.collateral_assets, market.index).unwrap();
    USERS.save(deps.as_mut().storage, &borrower_addr, &user).unwrap();

    // Set the querier to return collateral balance
    let deposit_coin_address = Addr::unchecked("matoken");
    deps.querier.set_cw20_balances(
        deposit_coin_address,
        &[(borrower_addr, Uint128::new(deposit_amount) * SCALING_FACTOR)],
    );

    // Borrow full liquidity
    {
        let env = mock_env_at_block_time(block_time);
        let msg = ExecuteMsg::Borrow {
            denom: "uusd".to_string(),
            amount: initial_liquidity.into(),
            recipient: None,
        };
        let _res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();

        let market_after_borrow = MARKETS.load(&deps.storage, "uusd").unwrap();
        let debt_total = compute_underlying_amount(
            market_after_borrow.debt_total_scaled,
            market_after_borrow.borrow_index,
            ScalingOperation::Ceil,
        )
        .unwrap();
        assert_eq!(debt_total, initial_liquidity.into());
    }

    let new_block_time = 12000u64;
    // We need to update balance after borrowing
    deps.querier.set_contract_balances(&[coin(0, "uusd")]);

    // Try to borrow more than available liquidity
    {
        let env = mock_env_at_block_time(new_block_time);
        let msg = ExecuteMsg::Borrow {
            denom: "uusd".to_string(),
            amount: 100u128.into(),
            recipient: None,
        };
        let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(error_res, ContractError::OperationExceedsAvailableLiquidity {});
    }

    // Repay part of the debt
    {
        let env = mock_env_at_block_time(new_block_time);
        let info = cosmwasm_std::testing::mock_info("borrower", &[coin(2000, "uusd")]);
        let msg = ExecuteMsg::Repay {
            on_behalf_of: None,
        };
        // check that repay succeeds
        execute(deps.as_mut(), env, info, msg).unwrap();
    }
}

#[test]
fn test_borrow_collateral_check() {
    // NOTE: available liquidity stays fixed as the test environment does not get changes in
    // contract balances on subsequent calls. They would change from call to call in practice
    let available_liquidity_1 = Uint128::from(1000000000u128);
    let available_liquidity_2 = Uint128::from(2000000000u128);
    let available_liquidity_3 = Uint128::from(3000000000u128);
    let mut deps = th_setup(&[
        coin(available_liquidity_1.into(), "uatom"),
        coin(available_liquidity_2.into(), "uosmo"),
        coin(available_liquidity_3.into(), "uusd"),
    ]);

    let exchange_rate_1 = Decimal::one();
    let exchange_rate_2 = Decimal::from_ratio(15u128, 4u128);
    let exchange_rate_3 = Decimal::one();

    deps.querier.set_oracle_price("uatom", exchange_rate_1);
    deps.querier.set_oracle_price("uosmo", exchange_rate_2);
    // NOTE: base asset price (asset3) should be set to 1 by the oracle helper

    let mock_market_1 = Market {
        ma_token_address: Addr::unchecked("matoken1"),
        max_loan_to_value: Decimal::from_ratio(8u128, 10u128),
        debt_total_scaled: Uint128::zero(),
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::from_ratio(1u128, 2u128),
        ..Default::default()
    };
    let mock_market_2 = Market {
        ma_token_address: Addr::unchecked("matoken2"),
        max_loan_to_value: Decimal::from_ratio(6u128, 10u128),
        debt_total_scaled: Uint128::zero(),
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::from_ratio(1u128, 2u128),
        ..Default::default()
    };
    let mock_market_3 = Market {
        ma_token_address: Addr::unchecked("matoken3"),
        max_loan_to_value: Decimal::from_ratio(4u128, 10u128),
        debt_total_scaled: Uint128::zero(),
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::from_ratio(1u128, 2u128),
        ..Default::default()
    };

    // should get index 0
    let market_1_initial = th_init_market(deps.as_mut(), "uatom", &mock_market_1);
    // should get index 1
    let market_2_initial = th_init_market(deps.as_mut(), "uosmo", &mock_market_2);
    // should get index 2
    let market_3_initial = th_init_market(deps.as_mut(), "uusd", &mock_market_3);

    let borrower_addr = Addr::unchecked("borrower");

    // Set user as having all the markets as collateral
    let mut user = User::default();

    set_bit(&mut user.collateral_assets, market_1_initial.index).unwrap();
    set_bit(&mut user.collateral_assets, market_2_initial.index).unwrap();
    set_bit(&mut user.collateral_assets, market_3_initial.index).unwrap();

    USERS.save(deps.as_mut().storage, &borrower_addr, &user).unwrap();

    let ma_token_address_1 = Addr::unchecked("matoken1");
    let ma_token_address_2 = Addr::unchecked("matoken2");
    let ma_token_address_3 = Addr::unchecked("matoken3");

    let balance_1 = Uint128::new(4_000_000) * SCALING_FACTOR;
    let balance_2 = Uint128::new(7_000_000) * SCALING_FACTOR;
    let balance_3 = Uint128::new(3_000_000) * SCALING_FACTOR;

    // Set the querier to return a certain collateral balance
    deps.querier.set_cw20_balances(ma_token_address_1, &[(borrower_addr.clone(), balance_1)]);
    deps.querier.set_cw20_balances(ma_token_address_2, &[(borrower_addr.clone(), balance_2)]);
    deps.querier.set_cw20_balances(ma_token_address_3, &[(borrower_addr, balance_3)]);

    let max_borrow_allowed_in_base_asset = (market_1_initial.max_loan_to_value
        * compute_underlying_amount(
            balance_1,
            market_1_initial.liquidity_index,
            ScalingOperation::Truncate,
        )
        .unwrap()
        * exchange_rate_1)
        + (market_2_initial.max_loan_to_value
            * compute_underlying_amount(
                balance_2,
                market_2_initial.liquidity_index,
                ScalingOperation::Truncate,
            )
            .unwrap()
            * exchange_rate_2)
        + (market_3_initial.max_loan_to_value
            * compute_underlying_amount(
                balance_3,
                market_3_initial.liquidity_index,
                ScalingOperation::Truncate,
            )
            .unwrap()
            * exchange_rate_3);
    let exceeding_borrow_amount =
        math::divide_uint128_by_decimal(max_borrow_allowed_in_base_asset, exchange_rate_2).unwrap()
            + Uint128::from(100_u64);
    let permissible_borrow_amount =
        math::divide_uint128_by_decimal(max_borrow_allowed_in_base_asset, exchange_rate_2).unwrap()
            - Uint128::from(100_u64);

    // borrow above the allowed amount given current collateral, should fail
    let borrow_msg = ExecuteMsg::Borrow {
        denom: "uosmo".to_string(),
        amount: exceeding_borrow_amount,
        recipient: None,
    };
    let env = mock_env(MockEnvParams::default());
    let info = mock_info("borrower", &[]);
    let error_res = execute(deps.as_mut(), env.clone(), info.clone(), borrow_msg).unwrap_err();
    assert_eq!(error_res, ContractError::BorrowAmountExceedsGivenCollateral {});

    // borrow permissible amount given current collateral, should succeed
    let borrow_msg = ExecuteMsg::Borrow {
        denom: "uosmo".to_string(),
        amount: permissible_borrow_amount,
        recipient: None,
    };
    execute(deps.as_mut(), env, info, borrow_msg).unwrap();
}

#[test]
fn test_cannot_borrow_if_market_not_enabled() {
    let mut deps = th_setup(&[]);

    let mock_market = Market {
        ma_token_address: Addr::unchecked("ma_somecoin"),
        borrow_enabled: false,
        ..Default::default()
    };
    th_init_market(deps.as_mut(), "somecoin", &mock_market);

    // Check error when borrowing not allowed on market
    let env = mock_env(MockEnvParams::default());
    let info = cosmwasm_std::testing::mock_info("borrower", &[coin(110000, "somecoin")]);
    let msg = ExecuteMsg::Borrow {
        denom: "somecoin".to_string(),
        amount: Uint128::new(1000),
        recipient: None,
    };
    let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(
        error_res,
        ContractError::BorrowNotEnabled {
            denom: "somecoin".to_string()
        }
    );
}

#[test]
fn test_borrow_and_send_funds_to_another_user() {
    let initial_liquidity = 10000000;
    let mut deps = th_setup(&[coin(initial_liquidity, "uusd")]);

    let borrower_addr = Addr::unchecked("borrower");
    let another_user_addr = Addr::unchecked("another_user");

    let mock_market = Market {
        ma_token_address: Addr::unchecked("matoken"),
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::one(),
        max_loan_to_value: Decimal::from_ratio(5u128, 10u128),
        debt_total_scaled: Uint128::zero(),
        ..Default::default()
    };
    let market = th_init_market(deps.as_mut(), "uusd", &mock_market);

    // Set user as having the market_collateral deposited
    let deposit_amount_scaled = Uint128::new(100_000) * SCALING_FACTOR;
    let mut user = User::default();
    set_bit(&mut user.collateral_assets, market.index).unwrap();
    USERS.save(deps.as_mut().storage, &borrower_addr, &user).unwrap();

    // Set the querier to return collateral balance
    let deposit_coin_address = Addr::unchecked("matoken");
    deps.querier
        .set_cw20_balances(deposit_coin_address, &[(borrower_addr.clone(), deposit_amount_scaled)]);

    let borrow_amount = Uint128::from(1000u128);
    let msg = ExecuteMsg::Borrow {
        denom: "uusd".to_string(),
        amount: borrow_amount,
        recipient: Some(another_user_addr.to_string()),
    };
    let env = mock_env(MockEnvParams::default());
    let info = mock_info("borrower", &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    let market_after_borrow = MARKETS.load(&deps.storage, "uusd").unwrap();

    // 'borrower' has bit set for the borrowed asset of the market
    let user = USERS.load(&deps.storage, &borrower_addr).unwrap();
    assert!(get_bit(user.borrowed_assets, market.index).unwrap());

    // Debt for 'borrower' should exist
    let debt = DEBTS.load(&deps.storage, (&borrower_addr, "uusd")).unwrap();
    assert_eq!(
        borrow_amount,
        compute_underlying_amount(
            debt.amount_scaled,
            market_after_borrow.borrow_index,
            ScalingOperation::Ceil
        )
        .unwrap()
    );

    // Debt for 'another_user' should not exist
    let debt = DEBTS.may_load(&deps.storage, (&another_user_addr, "uusd")).unwrap();
    assert!(debt.is_none());

    // Check msgs and attributes (funds should be sent to 'another_user')
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: another_user_addr.to_string(),
            amount: coins(borrow_amount.u128(), "uusd")
        }))]
    );
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "outposts/red-bank/borrow"),
            attr("denom", "uusd"),
            attr("user", borrower_addr),
            attr("recipient", another_user_addr),
            attr("amount", borrow_amount.to_string()),
        ]
    );
}
