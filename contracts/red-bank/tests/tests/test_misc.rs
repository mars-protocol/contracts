use cosmwasm_std::{
    attr, coin, coins, testing::mock_info, Addr, BankMsg, CosmosMsg, Decimal, SubMsg, Uint128,
};
use mars_interest_rate::{
    compute_scaled_amount, compute_underlying_amount, get_scaled_debt_amount,
    get_updated_liquidity_index, ScalingOperation, SCALING_FACTOR,
};
use mars_owner::OwnerError::NotOwner;
use mars_red_bank::{
    contract::execute,
    error::ContractError,
    health,
    state::{DEBTS, MARKETS, UNCOLLATERALIZED_LOAN_LIMITS},
};
use mars_red_bank_types::{
    params::asset::AssetParams,
    red_bank::{Debt, ExecuteMsg, Market},
};
use mars_testing::{mock_env, mock_env_at_block_time, MockEnvParams};

use super::helpers::{
    has_collateral_enabled, has_collateral_position, has_debt_position, set_collateral, set_debt,
    th_build_interests_updated_event, th_default_asset_params, th_get_expected_indices_and_rates,
    th_init_market, th_setup, TestUtilizationDeltaInfo,
};

#[test]
fn uncollateralized_loan_limits() {
    let available_liquidity = Uint128::from(2000000000u128);
    let mut deps = th_setup(&[coin(available_liquidity.into(), "somecoin")]);

    let mock_market = Market {
        borrow_index: Decimal::from_ratio(12u128, 10u128),
        liquidity_index: Decimal::from_ratio(8u128, 10u128),
        borrow_rate: Decimal::from_ratio(20u128, 100u128),
        liquidity_rate: Decimal::from_ratio(10u128, 100u128),
        reserve_factor: Decimal::from_ratio(1u128, 10u128),
        collateral_total_scaled: Uint128::new(1_000_000_000_000u128),
        debt_total_scaled: Uint128::zero(),
        indexes_last_updated: 10000000,
        ..Default::default()
    };

    // should get index 0
    let market_initial = th_init_market(deps.as_mut(), "somecoin", &mock_market);
    deps.querier.set_redbank_params("somecoin", th_default_asset_params());

    let mut block_time = mock_market.indexes_last_updated + 10000u64;
    let initial_uncollateralized_loan_limit = Uint128::from(2400_u128);

    // Check that borrowers with uncollateralized debt cannot get an uncollateralized loan limit
    let existing_borrower_addr = Addr::unchecked("existing_borrower");
    set_debt(deps.as_mut(), &existing_borrower_addr, "somecoin", 123u128, false);

    let update_limit_msg = ExecuteMsg::UpdateUncollateralizedLoanLimit {
        denom: "somecoin".to_string(),
        user: existing_borrower_addr.to_string(),
        new_limit: initial_uncollateralized_loan_limit,
    };
    let update_limit_env = mock_env_at_block_time(block_time);
    let info = mock_info("owner", &[]);
    let err = execute(deps.as_mut(), update_limit_env.clone(), info, update_limit_msg).unwrap_err();
    assert_eq!(err, ContractError::UserHasCollateralizedDebt {});

    // Update uncollateralized loan limit for users without collateralized loans
    let borrower_addr = Addr::unchecked("borrower");

    let update_limit_msg = ExecuteMsg::UpdateUncollateralizedLoanLimit {
        denom: "somecoin".to_string(),
        user: borrower_addr.to_string(),
        new_limit: initial_uncollateralized_loan_limit,
    };

    // update limit as unauthorized user, should fail
    let info = mock_info("random", &[]);
    let error_res =
        execute(deps.as_mut(), update_limit_env.clone(), info, update_limit_msg.clone())
            .unwrap_err();
    assert_eq!(error_res, ContractError::Owner(NotOwner {}));

    // Update borrower limit as owner
    let info = mock_info("owner", &[]);
    execute(deps.as_mut(), update_limit_env, info, update_limit_msg).unwrap();

    // check user's limit has been updated to the appropriate amount
    let limit =
        UNCOLLATERALIZED_LOAN_LIMITS.load(&deps.storage, (&borrower_addr, "somecoin")).unwrap();
    assert_eq!(limit, initial_uncollateralized_loan_limit);

    // check user's uncollateralized debt flag is true (limit > 0)
    let debt = DEBTS.load(&deps.storage, (&borrower_addr, "somecoin")).unwrap();
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
    let info = mock_info("borrower", &[]);
    let res = execute(deps.as_mut(), borrow_env, info, borrow_msg).unwrap();

    let expected_params = th_get_expected_indices_and_rates(
        &market_initial,
        block_time,
        TestUtilizationDeltaInfo {
            less_liquidity: initial_borrow_amount,
            more_debt: initial_borrow_amount,
            ..Default::default()
        },
    );

    let market = MARKETS.load(deps.as_ref().storage, "somecoin").unwrap();
    let expected_borrow_amount_scaled =
        get_scaled_debt_amount(initial_borrow_amount, &market, block_time).unwrap();

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
            attr("sender", "borrower"),
            attr("recipient", "borrower"),
            attr("denom", "somecoin"),
            attr("amount", initial_borrow_amount),
            attr("amount_scaled", expected_borrow_amount_scaled),
        ]
    );
    assert_eq!(res.events, vec![th_build_interests_updated_event("somecoin", &expected_params)]);

    // Check debt
    assert!(has_debt_position(deps.as_ref(), &borrower_addr, "somecoin"));

    let debt = DEBTS.load(&deps.storage, (&borrower_addr, "somecoin")).unwrap();

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
    let info = mock_info("borrower", &[]);
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
    let info = mock_info("borrower", &[]);
    execute(deps.as_mut(), borrow_env, info, borrow_msg).unwrap();

    // Set limit to zero
    let update_allowance_msg = ExecuteMsg::UpdateUncollateralizedLoanLimit {
        user: borrower_addr.to_string(),
        denom: "somecoin".to_string(),
        new_limit: Uint128::zero(),
    };
    let allowance_env = mock_env_at_block_time(block_time);
    let info = mock_info("owner", &[]);
    let res_err = execute(deps.as_mut(), allowance_env, info, update_allowance_msg).unwrap_err();
    assert_eq!(res_err, ContractError::UserHasUncollateralizedDebt {});

    // check user's allowance is different than zero
    let allowance =
        UNCOLLATERALIZED_LOAN_LIMITS.load(&deps.storage, (&borrower_addr, "somecoin")).unwrap();
    assert!(!allowance.is_zero());

    // check user's uncollateralized debt flag is true (limit > 0)
    let debt = DEBTS.load(&deps.storage, (&borrower_addr, "somecoin")).unwrap();
    assert!(debt.uncollateralized);
}

#[test]
fn update_asset_collateral() {
    let mut deps = th_setup(&[]);

    let user_addr = Addr::unchecked(String::from("user"));

    let denom_1 = "depositedcoin1";
    let mock_market_1 = Market {
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::one(),
        ..Default::default()
    };
    let denom_2 = "depositedcoin2";
    let mock_market_2 = Market {
        liquidity_index: Decimal::from_ratio(1u128, 2u128),
        borrow_index: Decimal::one(),
        ..Default::default()
    };
    let denom_3 = "depositedcoin3";
    let mock_market_3 = Market {
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::from_ratio(2u128, 1u128),
        ..Default::default()
    };

    let market_1_initial = th_init_market(deps.as_mut(), denom_1, &mock_market_1);
    let market_2_initial = th_init_market(deps.as_mut(), denom_2, &mock_market_2);
    let market_3_initial = th_init_market(deps.as_mut(), denom_3, &mock_market_3);

    let asset_params_1 = AssetParams {
        max_loan_to_value: Decimal::from_ratio(40u128, 100u128),
        liquidation_threshold: Decimal::from_ratio(60u128, 100u128),
        ..th_default_asset_params()
    };
    deps.querier.set_redbank_params(denom_1, asset_params_1.clone());
    let asset_params_2 = AssetParams {
        max_loan_to_value: Decimal::from_ratio(50u128, 100u128),
        liquidation_threshold: Decimal::from_ratio(80u128, 100u128),
        ..th_default_asset_params()
    };
    deps.querier.set_redbank_params(denom_2, asset_params_2.clone());
    let asset_params_3 = AssetParams {
        max_loan_to_value: Decimal::from_ratio(20u128, 100u128),
        liquidation_threshold: Decimal::from_ratio(40u128, 100u128),
        ..th_default_asset_params()
    };
    deps.querier.set_redbank_params(denom_3, asset_params_3);

    // Set the querier to return exchange rates
    let token_1_exchange_rate = Decimal::from_ratio(2u128, 1u128);
    let token_2_exchange_rate = Decimal::from_ratio(3u128, 1u128);
    let token_3_exchange_rate = Decimal::from_ratio(4u128, 1u128);
    deps.querier.set_oracle_price(denom_1, token_1_exchange_rate);
    deps.querier.set_oracle_price(denom_2, token_2_exchange_rate);
    deps.querier.set_oracle_price(denom_3, token_3_exchange_rate);

    let env = mock_env(MockEnvParams::default());
    let info = mock_info(user_addr.as_str(), &[]);

    {
        // Set second asset as collateral
        set_collateral(deps.as_mut(), &user_addr, &market_2_initial.denom, Uint128::new(123), true);

        // Enable denom 1 as collateral in which the user currently doesn't have a position in
        let update_msg = ExecuteMsg::UpdateAssetCollateralStatus {
            denom: denom_1.to_string(),
            enable: true,
        };
        let error_res =
            execute(deps.as_mut(), env.clone(), info.clone(), update_msg.clone()).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::UserNoCollateralBalance {
                user: user_addr.to_string(),
                denom: denom_1.to_string()
            }
        );

        // Balance for first asset is zero so don't update bit
        assert!(!has_collateral_position(deps.as_ref(), &user_addr, &market_1_initial.denom));

        // Set the querier to return balance more than zero for the first asset
        set_collateral(deps.as_mut(), &user_addr, denom_1, Uint128::new(100_000), false);

        // Enable first market index which is currently disabled as collateral and ma-token balance is more than 0
        execute(deps.as_mut(), env.clone(), info.clone(), update_msg).unwrap();
        assert!(has_collateral_enabled(deps.as_ref(), &user_addr, &market_1_initial.denom));

        // Disable second market index
        let update_msg = ExecuteMsg::UpdateAssetCollateralStatus {
            denom: denom_2.to_string(),
            enable: false,
        };
        execute(deps.as_mut(), env.clone(), info.clone(), update_msg).unwrap();
        assert!(!has_collateral_enabled(deps.as_ref(), &user_addr, &market_2_initial.denom));
    }

    // User's health factor can't be less than 1 after disabling collateral
    {
        // Initialize user with market_1 and market_2 as collaterals
        // User borrows market_3, which will be set up later in the test
        let token_1_balance_scaled = Uint128::new(150_000) * SCALING_FACTOR;
        set_collateral(
            deps.as_mut(),
            &user_addr,
            &market_1_initial.denom,
            token_1_balance_scaled,
            true,
        );
        let token_2_balance_scaled = Uint128::new(220_000) * SCALING_FACTOR;
        set_collateral(
            deps.as_mut(),
            &user_addr,
            &market_2_initial.denom,
            token_2_balance_scaled,
            true,
        );

        // Calculate maximum debt for the user to have valid health factor
        let token_1_weighted_lt_in_base_asset = compute_underlying_amount(
            token_1_balance_scaled,
            get_updated_liquidity_index(&market_1_initial, env.block.time.seconds()).unwrap(),
            ScalingOperation::Truncate,
        )
        .unwrap()
            * asset_params_1.liquidation_threshold
            * token_1_exchange_rate;
        let token_2_weighted_lt_in_base_asset = compute_underlying_amount(
            token_2_balance_scaled,
            get_updated_liquidity_index(&market_2_initial, env.block.time.seconds()).unwrap(),
            ScalingOperation::Truncate,
        )
        .unwrap()
            * asset_params_2.liquidation_threshold
            * token_2_exchange_rate;
        let weighted_liquidation_threshold_in_base_asset =
            token_1_weighted_lt_in_base_asset + token_2_weighted_lt_in_base_asset;
        let max_debt_for_valid_hf = weighted_liquidation_threshold_in_base_asset
            .checked_div_floor(token_3_exchange_rate)
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
        DEBTS.save(deps.as_mut().storage, (&user_addr, denom_3), &debt).unwrap();

        let positions = health::get_user_positions_map(
            &deps.as_ref(),
            &env,
            &user_addr,
            "",
            &Addr::unchecked("oracle"),
            &Addr::unchecked("params"),
            false,
        )
        .unwrap();
        let health = health::compute_position_health(&positions).unwrap();

        // Should have valid health factor
        assert_eq!(health.liquidation_health_factor.unwrap(), Decimal::one());

        // Disable second market index
        let update_msg = ExecuteMsg::UpdateAssetCollateralStatus {
            denom: denom_2.to_string(),
            enable: false,
        };
        let res_error = execute(deps.as_mut(), env, info, update_msg).unwrap_err();
        assert_eq!(res_error, ContractError::InvalidHealthFactorAfterDisablingCollateral {})
    }
}
