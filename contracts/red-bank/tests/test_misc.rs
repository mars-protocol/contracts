use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{Addr, Decimal, Uint128};

use mars_outpost::math;
use mars_outpost::red_bank::{ExecuteMsg, Market};
use mars_testing::{mock_env, MockEnvParams};

use mars_red_bank::contract::execute;
use mars_red_bank::error::ContractError;
use mars_red_bank::health;
use mars_red_bank::interest_rates::{
    compute_underlying_amount, get_scaled_debt_amount, get_updated_liquidity_index,
    ScalingOperation, SCALING_FACTOR,
};

use helpers::{
    has_collateral_enabled, has_collateral_position, set_collateral, set_debt, th_init_market,
    th_setup,
};

mod helpers;

#[test]
fn test_update_asset_collateral() {
    let mut deps = th_setup(&[]);

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
        ma_token_address: ma_token_addr_3,
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::from_ratio(2u128, 1u128),
        max_loan_to_value: Decimal::from_ratio(20u128, 100u128),
        liquidation_threshold: Decimal::from_ratio(40u128, 100u128),
        ..Default::default()
    };

    let market_1_initial = th_init_market(deps.as_mut(), denom_1, &mock_market_1);
    let market_2_initial = th_init_market(deps.as_mut(), denom_2, &mock_market_2);
    let market_3_initial = th_init_market(deps.as_mut(), denom_3, &mock_market_3);

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
        set_collateral(deps.as_mut(), &user_addr, &market_2_initial.denom, true);

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
                user: user_addr.to_string(),
                denom: denom_1.to_string()
            }
        );

        // Balance for first asset is zero so don't update bit
        assert!(!has_collateral_position(deps.as_ref(), &user_addr, &market_1_initial.denom));

        // Set the querier to return balance more than zero for the first asset
        deps.querier.set_cw20_balances(
            ma_token_addr_1.clone(),
            &[(user_addr.clone(), Uint128::new(100_000))],
        );

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
        set_collateral(deps.as_mut(), &user_addr, &market_1_initial.denom, true);
        set_collateral(deps.as_mut(), &user_addr, &market_2_initial.denom, true);

        // Set the querier to return collateral balances (ma_token_1 and ma_token_2)
        let ma_token_1_balance_scaled = Uint128::new(150_000) * SCALING_FACTOR;
        deps.querier
            .set_cw20_balances(ma_token_addr_1, &[(user_addr.clone(), ma_token_1_balance_scaled)]);
        let ma_token_2_balance_scaled = Uint128::new(220_000) * SCALING_FACTOR;
        deps.querier
            .set_cw20_balances(ma_token_addr_2, &[(user_addr.clone(), ma_token_2_balance_scaled)]);

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
        set_debt(deps.as_mut(), &user_addr, denom_3, token_3_debt_scaled);

        let positions = health::get_user_positions_map(
            deps.as_ref(),
            &env,
            &user_addr,
            &Addr::unchecked("oracle"),
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
