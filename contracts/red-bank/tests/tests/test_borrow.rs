use cosmwasm_std::{
    attr, coin, coins, testing::mock_info, Addr, BankMsg, CosmosMsg, Decimal, SubMsg, Uint128,
};
use cw_utils::PaymentError;
use mars_interest_rate::{
    calculate_applied_linear_interest_rate, compute_scaled_amount, compute_underlying_amount,
    ScalingOperation, SCALING_FACTOR,
};
use mars_red_bank::{
    contract::execute,
    error::ContractError,
    state::{DEBTS, MARKETS},
};
use mars_testing::{mock_env, mock_env_at_block_time, MockEnvParams};
use mars_types::{
    address_provider::MarsAddressType,
    params::{AssetParams, CmSettings, RedBankSettings},
    red_bank::{ExecuteMsg, Market},
};

use super::helpers::{
    has_collateral_position, has_debt_position, set_collateral, th_build_interests_updated_event,
    th_default_asset_params, th_get_expected_indices_and_rates, th_init_market, th_setup,
    TestUtilizationDeltaInfo,
};

#[test]
fn borrow_and_repay() {
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
        borrow_index: Decimal::from_ratio(12u128, 10u128),
        liquidity_index: Decimal::from_ratio(8u128, 10u128),
        borrow_rate: Decimal::from_ratio(20u128, 100u128),
        liquidity_rate: Decimal::from_ratio(10u128, 100u128),
        reserve_factor: Decimal::from_ratio(1u128, 100u128),
        collateral_total_scaled: Uint128::new(1_000_000_000_000u128),
        debt_total_scaled: Uint128::zero(),
        indexes_last_updated: 10000000,
        ..Default::default()
    };
    let mock_market_2 = Market {
        borrow_index: Decimal::one(),
        liquidity_index: Decimal::one(),
        collateral_total_scaled: Uint128::new(1_000_000_000_000u128),
        ..Default::default()
    };
    let mock_market_3 = Market {
        borrow_index: Decimal::one(),
        liquidity_index: Decimal::from_ratio(11u128, 10u128),
        borrow_rate: Decimal::from_ratio(30u128, 100u128),
        reserve_factor: Decimal::from_ratio(3u128, 100u128),
        liquidity_rate: Decimal::from_ratio(20u128, 100u128),
        debt_total_scaled: Uint128::zero(),
        indexes_last_updated: 10000000,
        ..Default::default()
    };

    let market_1_initial = th_init_market(deps.as_mut(), "uosmo", &mock_market_1);
    let market_2_initial = th_init_market(deps.as_mut(), "uusd", &mock_market_2);
    th_init_market(deps.as_mut(), "uatom", &mock_market_3);

    deps.querier.set_redbank_params("uosmo", th_default_asset_params());
    deps.querier.set_redbank_params("uusd", th_default_asset_params());
    deps.querier.set_redbank_params(
        "uatom",
        AssetParams {
            max_loan_to_value: Decimal::from_ratio(7u128, 10u128),
            ..th_default_asset_params()
        },
    );

    let borrower_addr = Addr::unchecked("borrower");

    // Set user as having the market_collateral deposited
    set_collateral(
        deps.as_mut(),
        &borrower_addr,
        "uatom",
        Uint128::new(10000) * SCALING_FACTOR,
        true,
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
        TestUtilizationDeltaInfo {
            less_liquidity: borrow_amount,
            more_debt: borrow_amount,
            ..Default::default()
        },
    );

    let expected_debt_scaled_1_after_borrow = compute_scaled_amount(
        borrow_amount,
        expected_params_uosmo.borrow_index,
        ScalingOperation::Ceil,
    )
    .unwrap();

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
            attr("action", "borrow"),
            attr("sender", "borrower"),
            attr("recipient", "borrower"),
            attr("denom", "uosmo"),
            attr("amount", borrow_amount.to_string()),
            attr("amount_scaled", expected_debt_scaled_1_after_borrow),
        ]
    );
    assert_eq!(res.events, vec![th_build_interests_updated_event("uosmo", &expected_params_uosmo)]);

    // user should have a debt position in `uosmo` but not in `uusd`
    assert!(has_debt_position(deps.as_ref(), &borrower_addr, "uosmo"));
    assert!(!has_debt_position(deps.as_ref(), &borrower_addr, "uusd"));

    let debt = DEBTS.load(&deps.storage, (&borrower_addr, "uosmo")).unwrap();
    assert_eq!(expected_debt_scaled_1_after_borrow, debt.amount_scaled);

    let market_1_after_borrow = MARKETS.load(&deps.storage, "uosmo").unwrap();
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

    // user should have a debt position in `uosmo` but not in `uusd`
    assert!(has_debt_position(deps.as_ref(), &borrower_addr, "uosmo"));
    assert!(!has_debt_position(deps.as_ref(), &borrower_addr, "uusd"));

    let expected_params_uosmo = th_get_expected_indices_and_rates(
        &market_1_after_borrow,
        block_time,
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

    // user should have debt positions in both `uosmo` and `uusd`
    assert!(has_debt_position(deps.as_ref(), &borrower_addr, "uosmo"));
    assert!(has_debt_position(deps.as_ref(), &borrower_addr, "uusd"));

    let expected_params_uusd = th_get_expected_indices_and_rates(
        &market_2_initial,
        block_time,
        TestUtilizationDeltaInfo {
            less_liquidity: borrow_amount,
            more_debt: borrow_amount,
            ..Default::default()
        },
    );

    let expected_debt_scaled_2_after_borrow_2 = compute_scaled_amount(
        borrow_amount,
        expected_params_uusd.borrow_index,
        ScalingOperation::Ceil,
    )
    .unwrap();

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
            attr("action", "borrow"),
            attr("sender", "borrower"),
            attr("recipient", "borrower"),
            attr("denom", "uusd"),
            attr("amount", borrow_amount.to_string()),
            attr("amount_scaled", expected_debt_scaled_2_after_borrow_2),
        ]
    );
    assert_eq!(res.events, vec![th_build_interests_updated_event("uusd", &expected_params_uusd)]);

    let debt2 = DEBTS.load(&deps.storage, (&borrower_addr, "uusd")).unwrap();
    assert_eq!(expected_debt_scaled_2_after_borrow_2, debt2.amount_scaled);

    let market_2_after_borrow_2 = MARKETS.load(&deps.storage, "uusd").unwrap();
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
        TestUtilizationDeltaInfo {
            less_debt: repay_amount,
            user_current_debt_scaled: expected_debt_scaled_2_after_borrow_2,
            ..Default::default()
        },
    );

    let expected_repay_amount_scaled = compute_scaled_amount(
        repay_amount,
        expected_params_uusd.borrow_index,
        ScalingOperation::Ceil,
    )
    .unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "repay"),
            attr("sender", "borrower"),
            attr("on_behalf_of", "borrower"),
            attr("denom", "uusd"),
            attr("amount", repay_amount.to_string()),
            attr("amount_scaled", expected_repay_amount_scaled),
        ]
    );
    assert_eq!(res.events, vec![th_build_interests_updated_event("uusd", &expected_params_uusd)]);

    // user should have debt positions in both `uosmo` and `uusd`
    assert!(has_debt_position(deps.as_ref(), &borrower_addr, "uosmo"));
    assert!(has_debt_position(deps.as_ref(), &borrower_addr, "uusd"));

    let debt2 = DEBTS.load(&deps.storage, (&borrower_addr, "uusd")).unwrap();
    let market_2_after_repay_some_2 = MARKETS.load(&deps.storage, "uusd").unwrap();

    let expected_debt_scaled_2_after_repay_some_2 =
        expected_debt_scaled_2_after_borrow_2 - expected_repay_amount_scaled;
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
            attr("action", "repay"),
            attr("sender", "borrower"),
            attr("on_behalf_of", "borrower"),
            attr("denom", "uusd"),
            attr("amount", repay_amount.to_string()),
            attr("amount_scaled", expected_debt_scaled_2_after_repay_some_2),
        ]
    );
    assert_eq!(res.events, vec![th_build_interests_updated_event("uusd", &expected_params_uusd),]);

    // user should no longer has a debt position in uusd
    assert!(has_debt_position(deps.as_ref(), &borrower_addr, "uosmo"));
    assert!(!has_debt_position(deps.as_ref(), &borrower_addr, "uusd"));

    let market_2_after_repay_all_2 = MARKETS.load(&deps.storage, "uusd").unwrap();
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
            attr("action", "repay"),
            attr("sender", "borrower"),
            attr("on_behalf_of", "borrower"),
            attr("denom", "uosmo"),
            attr("amount", (repay_amount - expected_refund_amount).to_string()),
            attr("amount_scaled", expected_debt_scaled_1_after_borrow_again),
        ]
    );
    assert_eq!(
        res.events,
        vec![th_build_interests_updated_event("uosmo", &expected_params_uosmo),]
    );

    // user should no longer has a debt position in either asset
    assert!(!has_debt_position(deps.as_ref(), &borrower_addr, "uosmo"));
    assert!(!has_debt_position(deps.as_ref(), &borrower_addr, "uusd"));

    let market_1_after_repay_1 = MARKETS.load(&deps.storage, "uosmo").unwrap();
    assert_eq!(Uint128::zero(), market_1_after_repay_1.debt_total_scaled);
}

#[test]
fn repay_without_refund_on_behalf_of() {
    let mut deps = th_setup(&[coin(1000000000u128, "borrowedcoinnative")]);

    deps.querier.set_oracle_price("depositedcoinnative", Decimal::one());
    deps.querier.set_oracle_price("borrowedcoinnative", Decimal::one());

    let mock_market = Market {
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::one(),
        collateral_total_scaled: Uint128::new(1_000_000_000_000u128),
        ..Default::default()
    };

    let market_1_initial = th_init_market(deps.as_mut(), "depositedcoinnative", &mock_market); // collateral
    let market_2_initial = th_init_market(deps.as_mut(), "borrowedcoinnative", &mock_market);

    deps.querier.set_redbank_params(
        "depositedcoinnative",
        AssetParams {
            max_loan_to_value: Decimal::from_ratio(50u128, 100u128),
            ..th_default_asset_params()
        },
    );
    deps.querier.set_redbank_params(
        "borrowedcoinnative",
        AssetParams {
            max_loan_to_value: Decimal::from_ratio(50u128, 100u128),
            ..th_default_asset_params()
        },
    );

    let borrower_addr = Addr::unchecked("borrower");
    let user_addr = Addr::unchecked("user");

    // Set user as having the market_1_initial (collateral) deposited
    set_collateral(
        deps.as_mut(),
        &borrower_addr,
        &market_1_initial.denom,
        Uint128::new(10000) * SCALING_FACTOR,
        true,
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

    assert!(has_debt_position(deps.as_ref(), &borrower_addr, &market_2_initial.denom));

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

    // 'user' should not have positions in either the collateral or debt asset
    assert!(!has_collateral_position(deps.as_ref(), &user_addr, &market_1_initial.denom));
    assert!(!has_debt_position(deps.as_ref(), &user_addr, &market_2_initial.denom));

    // Debt for 'borrower' should be repayed in full, with the position deleted
    assert!(!has_debt_position(deps.as_ref(), &borrower_addr, &market_2_initial.denom));

    // Check msgs and attributes
    assert_eq!(res.messages, vec![]);
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "repay"),
            attr("sender", "user"),
            attr("on_behalf_of", "borrower"),
            attr("denom", "borrowedcoinnative"),
            attr("amount", repay_amount.to_string()),
            attr("amount_scaled", Uint128::new(repay_amount) * SCALING_FACTOR),
        ]
    );
}

#[test]
fn repay_with_refund_on_behalf_of() {
    let mut deps = th_setup(&[coin(1000000000u128, "borrowedcoinnative")]);

    deps.querier.set_oracle_price("depositedcoinnative", Decimal::one());
    deps.querier.set_oracle_price("borrowedcoinnative", Decimal::one());

    let mock_market = Market {
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::one(),
        collateral_total_scaled: Uint128::new(1_000_000_000_000u128),
        ..Default::default()
    };

    let market_1_initial = th_init_market(deps.as_mut(), "depositedcoinnative", &mock_market); // collateral
    let market_2_initial = th_init_market(deps.as_mut(), "borrowedcoinnative", &mock_market);

    deps.querier.set_redbank_params(
        "depositedcoinnative",
        AssetParams {
            max_loan_to_value: Decimal::from_ratio(50u128, 100u128),
            ..th_default_asset_params()
        },
    );
    deps.querier.set_redbank_params(
        "borrowedcoinnative",
        AssetParams {
            max_loan_to_value: Decimal::from_ratio(50u128, 100u128),
            ..th_default_asset_params()
        },
    );

    let borrower_addr = Addr::unchecked("borrower");
    let user_addr = Addr::unchecked("user");

    // Set user as having the market_1_initial (collateral) deposited
    set_collateral(
        deps.as_mut(),
        &borrower_addr,
        &market_1_initial.denom,
        Uint128::new(10000) * SCALING_FACTOR,
        true,
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

    assert!(has_debt_position(deps.as_ref(), &borrower_addr, &market_2_initial.denom));

    // *
    // 'user' repays part of the debt on behalf of 'borrower'
    // *
    let refund_amount = 800u128;
    let repay_amount = borrow_amount + refund_amount;
    let env = mock_env(MockEnvParams::default());
    let info = cosmwasm_std::testing::mock_info(
        user_addr.as_str(),
        &[coin(repay_amount, "borrowedcoinnative")],
    );
    let msg = ExecuteMsg::Repay {
        on_behalf_of: Some(borrower_addr.to_string()),
    };
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    // 'user' should not have positions in either the collateral or debt asset
    assert!(!has_collateral_position(deps.as_ref(), &user_addr, &market_1_initial.denom));
    assert!(!has_debt_position(deps.as_ref(), &user_addr, &market_2_initial.denom));

    // Debt for 'borrower' should be partially repayed
    assert!(!has_debt_position(deps.as_ref(), &borrower_addr, &market_2_initial.denom));

    // Check msgs and attributes
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: user_addr.to_string(),
            amount: coins(refund_amount, "borrowedcoinnative")
        }))]
    );
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "repay"),
            attr("sender", "user"),
            attr("on_behalf_of", "borrower"),
            attr("denom", "borrowedcoinnative"),
            attr("amount", borrow_amount.to_string()),
            attr("amount_scaled", Uint128::new(borrow_amount) * SCALING_FACTOR),
        ]
    );
}

#[test]
fn repay_on_behalf_of_credit_manager() {
    let mut deps = th_setup(&[]);

    let repayer_addr = Addr::unchecked("repayer");
    let another_user_addr = Addr::unchecked(MarsAddressType::CreditManager.to_string());

    let env = mock_env(MockEnvParams::default());
    let info = cosmwasm_std::testing::mock_info(repayer_addr.as_str(), &[coin(110000, "somecoin")]);
    let msg = ExecuteMsg::Repay {
        on_behalf_of: Some(another_user_addr.to_string()),
    };
    let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(error_res, ContractError::CannotRepayOnBehalfOfCreditManager {});
}

#[test]
fn borrow_uusd() {
    let initial_liquidity = 10000000;
    let mut deps = th_setup(&[coin(initial_liquidity, "uusd")]);
    let block_time = 1;

    let borrower_addr = Addr::unchecked("borrower");
    let ltv = Decimal::from_ratio(7u128, 10u128);

    let mock_market = Market {
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::from_ratio(20u128, 10u128),
        borrow_rate: Decimal::one(),
        liquidity_rate: Decimal::one(),
        collateral_total_scaled: Uint128::new(1_000_000_000_000u128),
        debt_total_scaled: Uint128::zero(),
        indexes_last_updated: block_time,
        ..Default::default()
    };
    let market = th_init_market(deps.as_mut(), "uusd", &mock_market);

    deps.querier.set_redbank_params(
        "uusd",
        AssetParams {
            max_loan_to_value: ltv,
            ..th_default_asset_params()
        },
    );

    // Set user as having the market_collateral deposited
    let deposit_amount_scaled = Uint128::new(110_000) * SCALING_FACTOR;
    set_collateral(deps.as_mut(), &borrower_addr, "uusd", deposit_amount_scaled, true);

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

    assert!(has_debt_position(deps.as_ref(), &borrower_addr, "uusd"));

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
fn borrow_full_liquidity_and_then_repay() {
    let initial_liquidity = 50000;
    let mut deps = th_setup(&[coin(initial_liquidity, "uusd")]);
    let info = mock_info("borrower", &[]);
    let borrower_addr = Addr::unchecked("borrower");
    let block_time = 1;
    let ltv = Decimal::one();

    let mock_market = Market {
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::one(),
        borrow_rate: Decimal::one(),
        liquidity_rate: Decimal::one(),
        collateral_total_scaled: Uint128::new(1_000_000_000_000u128),
        debt_total_scaled: Uint128::zero(),
        reserve_factor: Decimal::from_ratio(12u128, 100u128),
        indexes_last_updated: block_time,
        ..Default::default()
    };
    th_init_market(deps.as_mut(), "uusd", &mock_market);

    deps.querier.set_redbank_params(
        "uusd",
        AssetParams {
            max_loan_to_value: ltv,
            ..th_default_asset_params()
        },
    );

    // User should have amount of collateral more than initial liquidity in order to borrow full liquidity
    let deposit_amount = initial_liquidity + 1000u128;
    set_collateral(
        deps.as_mut(),
        &borrower_addr,
        "uusd",
        Uint128::new(deposit_amount) * SCALING_FACTOR,
        true,
    );

    // Borrow full liquidity
    {
        let env = mock_env_at_block_time(block_time);
        let msg = ExecuteMsg::Borrow {
            denom: "uusd".to_string(),
            amount: initial_liquidity.into(),
            recipient: None,
        };
        let _res = execute(deps.as_mut(), env, info, msg).unwrap();

        let market_after_borrow = MARKETS.load(&deps.storage, "uusd").unwrap();
        let debt_total = compute_underlying_amount(
            market_after_borrow.debt_total_scaled,
            market_after_borrow.borrow_index,
            ScalingOperation::Ceil,
        )
        .unwrap();
        assert_eq!(debt_total.u128(), initial_liquidity);
    }

    let new_block_time = 12000u64;
    // We need to update balance after borrowing
    deps.querier.set_contract_balances(&[coin(0, "uusd")]);

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
fn borrow_collateral_check() {
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
        collateral_total_scaled: Uint128::new(10_000_000_000_000u128),
        debt_total_scaled: Uint128::zero(),
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::from_ratio(1u128, 2u128),
        ..Default::default()
    };
    let mock_market_2 = Market {
        collateral_total_scaled: Uint128::new(10_000_000_000_000u128),
        debt_total_scaled: Uint128::zero(),
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::from_ratio(1u128, 2u128),
        ..Default::default()
    };
    let mock_market_3 = Market {
        collateral_total_scaled: Uint128::new(10_000_000_000_000u128),
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

    let asset_params_1 = AssetParams {
        max_loan_to_value: Decimal::from_ratio(8u128, 10u128),
        ..th_default_asset_params()
    };
    deps.querier.set_redbank_params("uatom", asset_params_1.clone());
    let asset_params_2 = AssetParams {
        max_loan_to_value: Decimal::from_ratio(6u128, 10u128),
        ..th_default_asset_params()
    };
    deps.querier.set_redbank_params("uosmo", asset_params_2.clone());
    let asset_params_3 = AssetParams {
        max_loan_to_value: Decimal::from_ratio(4u128, 10u128),
        ..th_default_asset_params()
    };
    deps.querier.set_redbank_params("uusd", asset_params_3.clone());

    let borrower_addr = Addr::unchecked("borrower");

    let balance_1 = Uint128::new(4_000_000) * SCALING_FACTOR;
    let balance_2 = Uint128::new(7_000_000) * SCALING_FACTOR;
    let balance_3 = Uint128::new(3_000_000) * SCALING_FACTOR;

    // Set user as having all the markets as collateral
    set_collateral(deps.as_mut(), &borrower_addr, &market_1_initial.denom, balance_1, true);
    set_collateral(deps.as_mut(), &borrower_addr, &market_2_initial.denom, balance_2, true);
    set_collateral(deps.as_mut(), &borrower_addr, &market_3_initial.denom, balance_3, true);

    let max_borrow_allowed_in_base_asset = (asset_params_1.max_loan_to_value
        * compute_underlying_amount(
            balance_1,
            market_1_initial.liquidity_index,
            ScalingOperation::Truncate,
        )
        .unwrap()
        * exchange_rate_1)
        + (asset_params_2.max_loan_to_value
            * compute_underlying_amount(
                balance_2,
                market_2_initial.liquidity_index,
                ScalingOperation::Truncate,
            )
            .unwrap()
            * exchange_rate_2)
        + (asset_params_3.max_loan_to_value
            * compute_underlying_amount(
                balance_3,
                market_3_initial.liquidity_index,
                ScalingOperation::Truncate,
            )
            .unwrap()
            * exchange_rate_3);
    let exceeding_borrow_amount =
        max_borrow_allowed_in_base_asset.checked_div_floor(exchange_rate_2).unwrap()
            + Uint128::from(100_u64);
    let permissible_borrow_amount =
        max_borrow_allowed_in_base_asset.checked_div_floor(exchange_rate_2).unwrap()
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
fn cannot_borrow_if_market_not_enabled() {
    let mut deps = th_setup(&[]);

    th_init_market(deps.as_mut(), "somecoin", &Market::default());

    deps.querier.set_redbank_params(
        "somecoin",
        AssetParams {
            credit_manager: CmSettings {
                whitelisted: false,

                hls: None,
            },
            red_bank: RedBankSettings {
                deposit_enabled: false,
                borrow_enabled: false,
            },
            ..th_default_asset_params()
        },
    );

    // Check error when borrowing not allowed on market
    let env = mock_env(MockEnvParams::default());
    let info = cosmwasm_std::testing::mock_info("borrower", &[]);
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
fn borrow_and_send_funds_to_another_user() {
    let initial_liquidity = 10000000;
    let mut deps = th_setup(&[coin(initial_liquidity, "uusd")]);

    let borrower_addr = Addr::unchecked("borrower");
    let another_user_addr = Addr::unchecked("another_user");

    let mock_market = Market {
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::one(),
        collateral_total_scaled: Uint128::new(1_000_000_000_000u128),
        debt_total_scaled: Uint128::zero(),
        ..Default::default()
    };
    let market = th_init_market(deps.as_mut(), "uusd", &mock_market);

    deps.querier.set_redbank_params(
        "uusd",
        AssetParams {
            max_loan_to_value: Decimal::from_ratio(5u128, 10u128),
            ..th_default_asset_params()
        },
    );

    // Set user as having the market_collateral deposited
    let deposit_amount_scaled = Uint128::new(100_000) * SCALING_FACTOR;
    set_collateral(deps.as_mut(), &borrower_addr, &market.denom, deposit_amount_scaled, true);

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
    assert!(has_debt_position(deps.as_ref(), &borrower_addr, &market.denom));

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
            attr("action", "borrow"),
            attr("sender", borrower_addr),
            attr("recipient", another_user_addr),
            attr("denom", "uusd"),
            attr("amount", borrow_amount.to_string()),
            attr("amount_scaled", borrow_amount * SCALING_FACTOR),
        ]
    );
}
