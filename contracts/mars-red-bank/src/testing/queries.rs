use cosmwasm_std::{attr, coin, coins, Addr, BankMsg, CosmosMsg, Decimal, SubMsg, Uint128};

use mars_outpost::error::MarsError;
use mars_outpost::math;
use mars_outpost::red_bank::msg::ExecuteMsg;
use mars_outpost::red_bank::{Debt, Market, User, UserHealthStatus};
use mars_testing::{mock_env, mock_env_at_block_time, mock_info, MockEnvParams};

use crate::accounts::get_user_position;
use crate::contract::execute;
use crate::error::ContractError;
use crate::events::build_debt_position_changed_event;
use crate::helpers::{get_bit, set_bit};
use crate::interest_rates::{
    compute_scaled_amount, compute_underlying_amount, get_scaled_debt_amount,
    get_updated_liquidity_index, ScalingOperation, SCALING_FACTOR,
};
use crate::state::{DEBTS, UNCOLLATERALIZED_LOAN_LIMITS, USERS};

use super::helpers as th;

#[test]
fn test_uncollateralized_loan_limits() {
    let available_liquidity = Uint128::from(2000000000u128);
    let mut deps = th::setup(&[coin(available_liquidity.into(), "somecoin")]);

    let mock_market = Market {
        ma_token_address: Addr::unchecked("matoken"),
        borrow_index: Decimal::from_ratio(12u128, 10u128),
        liquidity_index: Decimal::from_ratio(8u128, 10u128),
        borrow_rate: Decimal::from_ratio(20u128, 100u128),
        liquidity_rate: Decimal::from_ratio(10u128, 100u128),
        reserve_factor: Decimal::from_ratio(1u128, 10u128),
        debt_total_scaled: Uint128::zero(),
        indexes_last_updated: 10000000,
        ..Default::default()
    };

    // should get index 0
    let market_initial = th::init_market(deps.as_mut(), "somecoin", &mock_market);

    let mut block_time = mock_market.indexes_last_updated + 10000u64;
    let initial_uncollateralized_loan_limit = Uint128::from(2400_u128);

    // Check that borrowers with uncollateralized debt cannot get an uncollateralized loan limit
    let existing_borrower_addr = Addr::unchecked("existing_borrower");

    let mut existing_borrower = User::default();
    set_bit(&mut existing_borrower.borrowed_assets, 0).unwrap();
    USERS.save(&mut deps.storage, &existing_borrower_addr, &existing_borrower).unwrap();

    let update_limit_msg = ExecuteMsg::UpdateUncollateralizedLoanLimit {
        denom: "somecoin".to_string(),

        user_address: existing_borrower_addr.to_string(),
        new_limit: initial_uncollateralized_loan_limit,
    };
    let update_limit_env = mock_env_at_block_time(block_time);
    let info = mock_info("owner");
    let err = execute(deps.as_mut(), update_limit_env.clone(), info, update_limit_msg).unwrap_err();
    assert_eq!(err, ContractError::UserHasCollateralizedDebt {});

    // Update uncollateralized loan limit for users without collateralized loans
    let borrower_addr = Addr::unchecked("borrower");

    let update_limit_msg = ExecuteMsg::UpdateUncollateralizedLoanLimit {
        denom: "somecoin".to_string(),
        user_address: borrower_addr.to_string(),
        new_limit: initial_uncollateralized_loan_limit,
    };

    // update limit as unauthorized user, should fail
    let info = mock_info("random");
    let error_res =
        execute(deps.as_mut(), update_limit_env.clone(), info, update_limit_msg.clone())
            .unwrap_err();
    assert_eq!(error_res, MarsError::Unauthorized {}.into());

    // Update borrower limit as owner
    let info = mock_info("owner");
    execute(deps.as_mut(), update_limit_env, info, update_limit_msg).unwrap();

    // check user's limit has been updated to the appropriate amount
    let limit =
        UNCOLLATERALIZED_LOAN_LIMITS.load(&deps.storage, ("somecoin", &borrower_addr)).unwrap();
    assert_eq!(limit, initial_uncollateralized_loan_limit);

    // check user's uncollateralized debt flag is true (limit > 0)
    let debt = DEBTS.load(&deps.storage, ("somecoin", &borrower_addr)).unwrap();
    assert!(debt.uncollateralized);

    // Borrow asset
    block_time += 1000_u64;
    let initial_borrow_amount = initial_uncollateralized_loan_limit.multiply_ratio(1_u64, 2_u64);
    let borrow_msg = ExecuteMsg::Borrow {
        denom: "somecoin".to_string(),
        amount: initial_borrow_amount,
        recipient: None,
    };
    let borrow_env = mock_env_at_block_time(block_time);
    let info = mock_info("borrower");
    let res = execute(deps.as_mut(), borrow_env, info, borrow_msg).unwrap();

    let expected_params = th::get_expected_indices_and_rates(
        &market_initial,
        block_time,
        available_liquidity,
        th::TestUtilizationDeltaInfo {
            less_liquidity: initial_borrow_amount.into(),
            more_debt: initial_borrow_amount.into(),
            ..Default::default()
        },
    );

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: borrower_addr.to_string(),
            amount: coins(initial_borrow_amount.u128(), "somecoin")
        }))]
    );

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "borrow"),
            attr("denom", "somecoin"),
            attr("user", "borrower"),
            attr("recipient", "borrower"),
            attr("amount", initial_borrow_amount.to_string()),
        ]
    );
    assert_eq!(
        res.events,
        vec![
            build_debt_position_changed_event("somecoin", true, "borrower".to_string()),
            th::build_interests_updated_event("somecoin", &expected_params)
        ]
    );

    // Check debt
    let user = USERS.load(&deps.storage, &borrower_addr).unwrap();
    assert!(get_bit(user.borrowed_assets, 0).unwrap());

    let debt = DEBTS.load(&deps.storage, ("somecoin", &borrower_addr)).unwrap();

    let expected_debt_scaled_after_borrow = compute_scaled_amount(
        initial_borrow_amount,
        expected_params.borrow_index,
        ScalingOperation::Ceil,
    )
    .unwrap();

    assert_eq!(expected_debt_scaled_after_borrow, debt.amount_scaled);

    // Borrow an amount less than initial limit but exceeding current limit
    let remaining_limit = initial_uncollateralized_loan_limit - initial_borrow_amount;
    let exceeding_limit = remaining_limit + Uint128::from(100_u64);

    block_time += 1000_u64;
    let borrow_msg = ExecuteMsg::Borrow {
        denom: "somecoin".to_string(),
        amount: exceeding_limit,
        recipient: None,
    };
    let borrow_env = mock_env_at_block_time(block_time);
    let info = mock_info("borrower");
    let error_res = execute(deps.as_mut(), borrow_env, info, borrow_msg).unwrap_err();
    assert_eq!(error_res, ContractError::BorrowAmountExceedsUncollateralizedLoanLimit {});

    // Borrow a valid amount given uncollateralized loan limit
    block_time += 1000_u64;
    let borrow_msg = ExecuteMsg::Borrow {
        denom: "somecoin".to_string(),
        amount: remaining_limit - Uint128::from(20_u128),
        recipient: None,
    };
    let borrow_env = mock_env_at_block_time(block_time);
    let info = mock_info("borrower");
    execute(deps.as_mut(), borrow_env, info, borrow_msg).unwrap();

    // Set limit to zero
    let update_allowance_msg = ExecuteMsg::UpdateUncollateralizedLoanLimit {
        denom: "somecoin".to_string(),
        user_address: borrower_addr.to_string(),
        new_limit: Uint128::zero(),
    };
    let allowance_env = mock_env_at_block_time(block_time);
    let info = mock_info("owner");
    execute(deps.as_mut(), allowance_env, info, update_allowance_msg).unwrap();

    // check user's allowance is zero
    let allowance =
        UNCOLLATERALIZED_LOAN_LIMITS.load(&deps.storage, ("somecoin", &borrower_addr)).unwrap();
    assert_eq!(allowance, Uint128::zero());

    // check user's uncollateralized debt flag is false (limit == 0)
    let debt = DEBTS.load(&deps.storage, ("somecoin", &borrower_addr)).unwrap();
    assert!(!debt.uncollateralized);
}

#[test]
fn test_update_asset_collateral() {
    let mut deps = th::setup(&[]);

    let user_addr = Addr::unchecked(String::from("user"));

    let denom_1 = "depositedcoin1";
    let ma_token_addr_1 = Addr::unchecked("matoken1");
    let mock_market_1 = Market {
        ma_token_address: ma_token_addr_1.clone(),
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::one(),
        max_loan_to_value: Decimal::from_ratio(40u128, 100u128),
        liquidation_threshold: Decimal::from_ratio(60u128, 100u128),
        ..Default::default()
    };
    let denom_2 = "depositedcoin2";
    let ma_token_addr_2 = Addr::unchecked("matoken2");
    let mock_market_2 = Market {
        ma_token_address: ma_token_addr_2.clone(),
        liquidity_index: Decimal::from_ratio(1u128, 2u128),
        borrow_index: Decimal::one(),
        max_loan_to_value: Decimal::from_ratio(50u128, 100u128),
        liquidation_threshold: Decimal::from_ratio(80u128, 100u128),
        ..Default::default()
    };
    let denom_3 = "depositedcoin3";
    let ma_token_addr_3 = Addr::unchecked("matoken3");
    let mock_market_3 = Market {
        ma_token_address: ma_token_addr_3.clone(),
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::from_ratio(2u128, 1u128),
        max_loan_to_value: Decimal::from_ratio(20u128, 100u128),
        liquidation_threshold: Decimal::from_ratio(40u128, 100u128),
        ..Default::default()
    };

    let market_1_initial = th::init_market(deps.as_mut(), denom_1, &mock_market_1);
    let market_2_initial = th::init_market(deps.as_mut(), denom_2, &mock_market_2);
    let market_3_initial = th::init_market(deps.as_mut(), denom_3, &mock_market_3);

    // Set the querier to return exchange rates
    let token_1_exchange_rate = Decimal::from_ratio(2u128, 1u128);
    let token_2_exchange_rate = Decimal::from_ratio(3u128, 1u128);
    let token_3_exchange_rate = Decimal::from_ratio(4u128, 1u128);
    deps.querier.set_oracle_price(denom_1, token_1_exchange_rate);
    deps.querier.set_oracle_price(denom_2, token_2_exchange_rate);
    deps.querier.set_oracle_price(denom_3, token_3_exchange_rate);

    let env = mock_env(MockEnvParams::default());
    let info = mock_info(user_addr.as_str());

    {
        // Set second asset as collateral
        let mut user = User::default();
        set_bit(&mut user.collateral_assets, market_2_initial.index).unwrap();
        USERS.save(deps.as_mut().storage, &user_addr, &user).unwrap();

        // Set the querier to return zero for the first asset
        deps.querier
            .set_cw20_balances(ma_token_addr_1.clone(), &[(user_addr.clone(), Uint128::zero())]);

        // Enable first market index which is currently disabled as collateral and ma-token balance is 0
        let update_msg = ExecuteMsg::UpdateAssetCollateralStatus {
            denom: denom_1.to_string(),
            enable: true,
        };
        let error_res =
            execute(deps.as_mut(), env.clone(), info.clone(), update_msg.clone()).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::UserNoCollateralBalance {
                user_address: user_addr.to_string(),
                denom: denom_1.to_string()
            }
        );

        let user = USERS.load(&deps.storage, &user_addr).unwrap();
        let market_1_collateral = get_bit(user.collateral_assets, market_1_initial.index).unwrap();
        // Balance for first asset is zero so don't update bit
        assert!(!market_1_collateral);

        // Set the querier to return balance more than zero for the first asset
        deps.querier.set_cw20_balances(
            ma_token_addr_1.clone(),
            &[(user_addr.clone(), Uint128::new(100_000))],
        );

        // Enable first market index which is currently disabled as collateral and ma-token balance is more than 0
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), update_msg).unwrap();
        let user = USERS.load(&deps.storage, &user_addr).unwrap();
        let market_1_collateral = get_bit(user.collateral_assets, market_1_initial.index).unwrap();
        // Balance for first asset is more than zero so update bit
        assert!(market_1_collateral);

        // Disable second market index
        let update_msg = ExecuteMsg::UpdateAssetCollateralStatus {
            denom: denom_2.to_string(),
            enable: false,
        };
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), update_msg).unwrap();
        let user = USERS.load(&deps.storage, &user_addr).unwrap();
        let market_2_collateral = get_bit(user.collateral_assets, market_2_initial.index).unwrap();
        assert!(!market_2_collateral);
    }

    // User's health factor can't be less than 1 after disabling collateral
    {
        // Initialize user with market_1 and market_2 as collaterals
        // User borrows market_3
        let mut user = User::default();
        set_bit(&mut user.collateral_assets, market_1_initial.index).unwrap();
        set_bit(&mut user.collateral_assets, market_2_initial.index).unwrap();
        set_bit(&mut user.borrowed_assets, market_3_initial.index).unwrap();
        USERS.save(deps.as_mut().storage, &user_addr, &user).unwrap();

        // Set the querier to return collateral balances (ma_token_1 and ma_token_2)
        let ma_token_1_balance_scaled = Uint128::new(150_000) * SCALING_FACTOR;
        deps.querier.set_cw20_balances(
            ma_token_addr_1.clone(),
            &[(user_addr.clone(), ma_token_1_balance_scaled.into())],
        );
        let ma_token_2_balance_scaled = Uint128::new(220_000) * SCALING_FACTOR;
        deps.querier.set_cw20_balances(
            ma_token_addr_2.clone(),
            &[(user_addr.clone(), ma_token_2_balance_scaled.into())],
        );

        // Calculate maximum debt for the user to have valid health factor
        let token_1_weighted_lt_in_base_asset = compute_underlying_amount(
            ma_token_1_balance_scaled,
            get_updated_liquidity_index(&market_1_initial, env.block.time.seconds()).unwrap(),
            ScalingOperation::Truncate,
        )
        .unwrap()
            * market_1_initial.liquidation_threshold
            * token_1_exchange_rate;
        let token_2_weighted_lt_in_base_asset = compute_underlying_amount(
            ma_token_2_balance_scaled,
            get_updated_liquidity_index(&market_2_initial, env.block.time.seconds()).unwrap(),
            ScalingOperation::Truncate,
        )
        .unwrap()
            * market_2_initial.liquidation_threshold
            * token_2_exchange_rate;
        let weighted_liquidation_threshold_in_base_asset =
            token_1_weighted_lt_in_base_asset + token_2_weighted_lt_in_base_asset;
        let max_debt_for_valid_hf = math::divide_uint128_by_decimal(
            weighted_liquidation_threshold_in_base_asset,
            token_3_exchange_rate,
        )
        .unwrap();
        let token_3_debt_scaled = get_scaled_debt_amount(
            max_debt_for_valid_hf,
            &market_3_initial,
            env.block.time.seconds(),
        )
        .unwrap();

        // Set user to have max debt for valid health factor
        let debt = Debt {
            amount_scaled: token_3_debt_scaled,
            uncollateralized: false,
        };
        DEBTS.save(deps.as_mut().storage, (denom_3, &user_addr), &debt).unwrap();

        let user_position = get_user_position(
            deps.as_ref(),
            env.block.time.seconds(),
            &user_addr,
            &Addr::unchecked("oracle"),
            &user,
            3,
        )
        .unwrap();
        // Should have valid health factor
        assert_eq!(user_position.health_status, UserHealthStatus::Borrowing(Decimal::one()));

        // Disable second market index
        let update_msg = ExecuteMsg::UpdateAssetCollateralStatus {
            denom: denom_2.to_string(),
            enable: false,
        };
        let res_error = execute(deps.as_mut(), env.clone(), info, update_msg).unwrap_err();
        assert_eq!(res_error, ContractError::InvalidHealthFactorAfterDisablingCollateral {})
    }
}
