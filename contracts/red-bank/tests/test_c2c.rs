use cosmwasm_std::testing::{mock_info, MockApi, MockStorage};
use cosmwasm_std::{
    attr, coin, coins, Addr, BankMsg, CosmosMsg, Decimal, OwnedDeps, SubMsg, Uint128,
};

use mars_outpost::error::MarsError;
use mars_outpost::red_bank::{ExecuteMsg, Market};
use mars_testing::{mock_env_at_block_time, MarsMockQuerier};

use mars_red_bank::contract::execute;
use mars_red_bank::error::ContractError;
use mars_red_bank::interest_rates::{compute_scaled_amount, ScalingOperation, SCALING_FACTOR};
use mars_red_bank::state::{DEBTS, UNCOLLATERALIZED_LOAN_LIMITS};

use helpers::{
    set_collateral, set_debt, set_uncollatateralized_loan_limit, th_build_interests_updated_event,
    th_get_expected_indices_and_rates, th_init_market, th_setup, unset_debt,
    TestUtilizationDeltaInfo,
};

mod helpers;

struct TestSuite {
    deps: OwnedDeps<MockStorage, MockApi, MarsMockQuerier>,
    block_time: u64,
    market: Market,
    borrower_addr: Addr,
    limit: Uint128,
    available_liquidity: Uint128,
}

fn setup_test() -> TestSuite {
    let available_liquidity = Uint128::from(2000000000u128);
    let mut deps = th_setup(&[coin(available_liquidity.into(), "somecoin")]);

    let market = th_init_market(
        deps.as_mut(),
        "somecoin",
        &Market {
            ma_token_address: Addr::unchecked("matoken"),
            max_loan_to_value: Decimal::one(),
            borrow_index: Decimal::from_ratio(12u128, 10u128),
            liquidity_index: Decimal::from_ratio(8u128, 10u128),
            borrow_rate: Decimal::from_ratio(20u128, 100u128),
            liquidity_rate: Decimal::from_ratio(10u128, 100u128),
            reserve_factor: Decimal::from_ratio(1u128, 10u128),
            debt_total_scaled: Uint128::zero(),
            indexes_last_updated: 10000000,
            ..Default::default()
        },
    );

    // set oracle price for the asset
    deps.querier.set_oracle_price("somecoin", Decimal::one());

    // give borrower a limit
    let borrower_addr = Addr::unchecked("borrower");
    let limit = Uint128::new(2400);
    set_uncollatateralized_loan_limit(deps.as_mut(), &borrower_addr, &market.denom, limit);

    TestSuite {
        deps,
        block_time: market.indexes_last_updated + 10000,
        market,
        borrower_addr,
        limit,
        available_liquidity,
    }
}

#[test]
fn test_set_uncollatateralized_loan_limit() {
    let TestSuite {
        mut deps,
        block_time,
        market,
        borrower_addr,
        ..
    } = setup_test();

    let env = mock_env_at_block_time(block_time);

    let new_limit = Uint128::new(4800);
    let msg = ExecuteMsg::UpdateUncollateralizedLoanLimit {
        denom: market.denom.clone(),
        user: borrower_addr.to_string(),
        new_limit,
    };

    // update limit as unauthorized user, should fail
    let info = mock_info("random", &[]);
    let err = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap_err();
    assert_eq!(err, MarsError::Unauthorized {}.into());

    // Update borrower limit as owner
    let info = mock_info("owner", &[]);
    execute(deps.as_mut(), env, info, msg).unwrap();

    // check user's limit has been updated to the appropriate amount
    let stored_limit =
        UNCOLLATERALIZED_LOAN_LIMITS.load(&deps.storage, (&borrower_addr, &market.denom)).unwrap();
    assert_eq!(stored_limit, new_limit);
}

#[test]
fn test_borrow_under_limit() {
    let TestSuite {
        mut deps,
        market,
        mut block_time,
        borrower_addr,
        limit,
        available_liquidity,
    } = setup_test();

    block_time += 1000_u64;

    // Borrow asset
    let borrow_amount = limit.multiply_ratio(1_u64, 2_u64);
    let res = execute(
        deps.as_mut(),
        mock_env_at_block_time(block_time),
        mock_info(borrower_addr.as_str(), &[]),
        ExecuteMsg::Borrow {
            denom: market.denom.clone(),
            amount: borrow_amount,
            recipient: None,
        },
    )
    .unwrap();

    let expected_params = th_get_expected_indices_and_rates(
        &market,
        block_time,
        available_liquidity,
        TestUtilizationDeltaInfo {
            less_liquidity: borrow_amount,
            more_debt: borrow_amount,
            ..Default::default()
        },
    );

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: borrower_addr.to_string(),
            amount: coins(borrow_amount.u128(), &market.denom)
        }))]
    );
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "outposts/red-bank/borrow"),
            attr("denom", &market.denom),
            attr("user", &borrower_addr),
            attr("recipient", &borrower_addr),
            attr("amount", borrow_amount.to_string()),
        ]
    );
    assert_eq!(res.events, vec![th_build_interests_updated_event(&market.denom, &expected_params)]);

    // Check debt
    let debt_amount_scaled = DEBTS.load(&deps.storage, (&borrower_addr, &market.denom)).unwrap();
    let expected_debt_scaled_after_borrow =
        compute_scaled_amount(borrow_amount, expected_params.borrow_index, ScalingOperation::Ceil)
            .unwrap();
    assert_eq!(expected_debt_scaled_after_borrow, debt_amount_scaled);
}

#[test]
fn test_borrow_above_limit_without_collateral() {
    let TestSuite {
        mut deps,
        mut block_time,
        market,
        borrower_addr,
        limit,
        ..
    } = setup_test();

    block_time += 1000_u64;
    let borrow_amount = limit + Uint128::from(100_u64);

    // Borrow an amount exceeding current limit
    // Without any collateral, this should fail
    let err = execute(
        deps.as_mut(),
        mock_env_at_block_time(block_time),
        mock_info(borrower_addr.as_str(), &[]),
        ExecuteMsg::Borrow {
            denom: market.denom,
            amount: borrow_amount,
            recipient: None,
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::BorrowAmountExceedsGivenCollateral {});
}

#[test]
fn test_borrow_above_limit_with_collateral() {
    let TestSuite {
        mut deps,
        market,
        mut block_time,
        borrower_addr,
        limit,
        available_liquidity,
    } = setup_test();

    block_time += 1000_u64;
    let borrow_amount = limit + Uint128::from(100_u64);

    // The borrower deposits sufficient collateral
    set_collateral(deps.as_mut(), &borrower_addr, &market.denom, true);
    deps.querier.set_cw20_balances(
        market.ma_token_address.clone(),
        &[(borrower_addr.clone(), Uint128::new(400) * SCALING_FACTOR)],
    );

    // borrower attempts to borrow above limit, should work
    execute(
        deps.as_mut(),
        mock_env_at_block_time(block_time),
        mock_info(borrower_addr.as_str(), &[]),
        ExecuteMsg::Borrow {
            denom: market.denom.clone(),
            amount: borrow_amount,
            recipient: None,
        },
    )
    .unwrap();

    let expected_params = th_get_expected_indices_and_rates(
        &market,
        block_time,
        available_liquidity,
        TestUtilizationDeltaInfo {
            less_liquidity: borrow_amount,
            more_debt: borrow_amount,
            ..Default::default()
        },
    );

    let debt_amount_scaled = DEBTS.load(&deps.storage, (&borrower_addr, &market.denom)).unwrap();
    let borrow_amount_scaled =
        compute_scaled_amount(borrow_amount, expected_params.borrow_index, ScalingOperation::Ceil)
            .unwrap();
    assert_eq!(debt_amount_scaled, borrow_amount_scaled);
}

#[test]
fn test_reduce_uncollateralized_loan_limit() {
    let TestSuite {
        mut deps,
        market,
        block_time,
        borrower_addr,
        ..
    } = setup_test();

    // set user debt
    set_debt(deps.as_mut(), &borrower_addr, &market.denom, 12345u128);

    // Set limit to zero, should fail as user's health factor would be below 1
    let env = mock_env_at_block_time(block_time);
    let info = mock_info("owner", &[]);
    let msg = ExecuteMsg::UpdateUncollateralizedLoanLimit {
        user: borrower_addr.to_string(),
        denom: market.denom.clone(),
        new_limit: Uint128::zero(),
    };
    let err = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap_err();
    assert_eq!(err, ContractError::InvalidHealthFactorAfterSettingUncollateralizedLoanLimit {});

    // remove the user's debt and try again, should work
    unset_debt(deps.as_mut(), &borrower_addr, &market.denom);
    execute(deps.as_mut(), env, info, msg).unwrap();

    // check user's allowance should have been deleted
    let opt = UNCOLLATERALIZED_LOAN_LIMITS
        .may_load(&deps.storage, (&borrower_addr, &market.denom))
        .unwrap();
    assert!(opt.is_none());
}
