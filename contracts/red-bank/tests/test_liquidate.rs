use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{
    attr, coin, coins, to_binary, Addr, BankMsg, CosmosMsg, Decimal, StdResult, SubMsg, Uint128,
    WasmMsg,
};
use cw_utils::PaymentError;

use mars_outpost::address_provider::MarsContract;
use mars_outpost::red_bank::{Debt, ExecuteMsg, InterestRateModel, Market};
use mars_outpost::{incentives, math};
use mars_red_bank::contract::execute;
use mars_red_bank::error::ContractError;
use mars_red_bank::interest_rates::{
    compute_scaled_amount, compute_underlying_amount, get_scaled_liquidity_amount,
    ScalingOperation, SCALING_FACTOR,
};
use mars_red_bank::state::{COLLATERALS, CONFIG, DEBTS, MARKETS};
use mars_testing::{mock_env, mock_env_at_block_time, MockEnvParams};

use helpers::{
    has_collateral_position, set_collateral, th_build_interests_updated_event,
    th_get_expected_indices, th_get_expected_indices_and_rates, th_init_market, th_setup,
    unset_collateral, TestUtilizationDeltaInfo,
};

mod helpers;

#[test]
fn test_liquidate() {
    // Setup
    let available_liquidity_collateral = Uint128::from(1_000_000_000u128);
    let available_liquidity_debt = Uint128::from(2_000_000_000u128);
    let mut deps = th_setup(&[
        coin(available_liquidity_collateral.into(), "collateral"),
        coin(available_liquidity_debt.into(), "debt"),
    ]);

    let user_addr = Addr::unchecked("user");
    let liquidator_addr = Addr::unchecked("liquidator");

    let collateral_max_ltv = Decimal::from_ratio(5u128, 10u128);
    let collateral_liquidation_threshold = Decimal::from_ratio(6u128, 10u128);
    let collateral_liquidation_bonus = Decimal::from_ratio(1u128, 10u128);
    let collateral_price = Decimal::from_ratio(2_u128, 1_u128);
    let debt_price = Decimal::from_ratio(11_u128, 10_u128);
    let uncollateralized_debt_price = Decimal::from_ratio(15_u128, 10_u128);
    let user_collateral_balance = 2_000_000;
    let user_debt = Uint128::from(3_000_000_u64); // ltv = 0.75
    let close_factor = Decimal::from_ratio(1u128, 2u128);

    let first_debt_to_repay = Uint128::from(400_000_u64);
    let first_block_time = 15_000_000;

    let second_debt_to_repay = Uint128::from(10_000_000_u64);
    let second_block_time = 16_000_000;

    // Global debt for the debt market
    let expected_global_collateral_scaled = Uint128::new(1_500_000_000) * SCALING_FACTOR;
    let mut expected_global_debt_scaled = Uint128::new(1_800_000_000) * SCALING_FACTOR;
    let mut expected_global_reward_scaled = Uint128::zero(); // can be any number, but just using zero for now for convenience

    CONFIG
        .update(deps.as_mut().storage, |mut config| -> StdResult<_> {
            config.close_factor = close_factor;
            Ok(config)
        })
        .unwrap();

    // initialize collateral and debt markets

    deps.querier.set_oracle_price("collateral", collateral_price);
    deps.querier.set_oracle_price("debt", debt_price);
    deps.querier.set_oracle_price("uncollateralized_debt", uncollateralized_debt_price);

    // for the test to pass, we need an interest rate model that gives non-zero rates
    let mock_ir_model = InterestRateModel {
        optimal_utilization_rate: Decimal::one(),
        base: Decimal::percent(5),
        slope_1: Decimal::zero(),
        slope_2: Decimal::zero(),
    };

    let collateral_market = Market {
        max_loan_to_value: collateral_max_ltv,
        liquidation_threshold: collateral_liquidation_threshold,
        liquidation_bonus: collateral_liquidation_bonus,
        collateral_total_scaled: expected_global_collateral_scaled,
        debt_total_scaled: Uint128::new(800_000_000) * SCALING_FACTOR,
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::one(),
        borrow_rate: Decimal::from_ratio(2u128, 10u128),
        liquidity_rate: Decimal::from_ratio(2u128, 10u128),
        interest_rate_model: mock_ir_model.clone(),
        reserve_factor: Decimal::from_ratio(2u128, 100u128),
        indexes_last_updated: 0,
        ..Default::default()
    };

    let debt_market = Market {
        max_loan_to_value: Decimal::from_ratio(6u128, 10u128),
        collateral_total_scaled: expected_global_reward_scaled,
        debt_total_scaled: expected_global_debt_scaled,
        liquidity_index: Decimal::from_ratio(12u128, 10u128),
        borrow_index: Decimal::from_ratio(14u128, 10u128),
        borrow_rate: Decimal::from_ratio(2u128, 10u128),
        liquidity_rate: Decimal::from_ratio(2u128, 10u128),
        interest_rate_model: mock_ir_model,
        reserve_factor: Decimal::from_ratio(3u128, 100u128),
        indexes_last_updated: 0,
        ..Default::default()
    };

    let uncollateralized_debt_market = Market {
        denom: "uncollateralized_debt".to_string(),
        ..Default::default()
    };

    let collateral_market_initial = th_init_market(deps.as_mut(), "collateral", &collateral_market);
    let debt_market_initial = th_init_market(deps.as_mut(), "debt", &debt_market);
    th_init_market(deps.as_mut(), "uncollateralized_debt", &uncollateralized_debt_market);

    let mut expected_user_collateral_scaled =
        Uint128::new(user_collateral_balance) * SCALING_FACTOR;
    let mut expected_liquidator_collateral_scaled = Uint128::zero();

    let mut expected_user_debt_scaled =
        compute_scaled_amount(user_debt, debt_market_initial.borrow_index, ScalingOperation::Ceil)
            .unwrap();

    let mut expected_total_reward_scaled = Uint128::zero();

    // trying to liquidate user with zero collateral balance should fail
    {
        let liquidate_msg = ExecuteMsg::Liquidate {
            user: user_addr.to_string(),
            collateral_denom: "collateral".to_string(),
        };

        let env = mock_env(MockEnvParams::default());
        let info = mock_info(liquidator_addr.as_str(), &coins(first_debt_to_repay.u128(), "debt"));
        let error_res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap_err();
        assert_eq!(error_res, ContractError::CannotLiquidateWhenNoCollateralBalance {});
    }

    // Create collateral position for the user
    set_collateral(
        deps.as_mut(),
        &user_addr,
        &collateral_market_initial.denom,
        Uint128::new(user_collateral_balance) * SCALING_FACTOR,
        true,
    );

    // trying to liquidate user with zero outstanding debt should fail (uncollateralized has not impact)
    {
        // the user has a debt position in "uncollateralized_debt", but not in "debt"
        let uncollateralized_debt = Debt {
            amount_scaled: Uint128::new(10_000) * SCALING_FACTOR,
            uncollateralized: true,
        };
        DEBTS
            .save(
                deps.as_mut().storage,
                (&user_addr, "uncollateralized_debt"),
                &uncollateralized_debt,
            )
            .unwrap();

        let liquidate_msg = ExecuteMsg::Liquidate {
            user: user_addr.to_string(),
            collateral_denom: "collateral".to_string(),
        };

        let env = mock_env(MockEnvParams::default());
        let info = mock_info(liquidator_addr.as_str(), &coins(first_debt_to_repay.u128(), "debt"));
        let error_res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap_err();
        assert_eq!(error_res, ContractError::CannotLiquidateWhenNoDebtBalance {});
    }

    // set user to have positive debt amount in debt asset
    {
        let debt = Debt {
            amount_scaled: expected_user_debt_scaled,
            uncollateralized: false,
        };
        let uncollateralized_debt = Debt {
            amount_scaled: Uint128::new(10_000) * SCALING_FACTOR,
            uncollateralized: true,
        };
        DEBTS.save(deps.as_mut().storage, (&user_addr, "debt"), &debt).unwrap();
        DEBTS
            .save(
                deps.as_mut().storage,
                (&user_addr, "uncollateralized_debt"),
                &uncollateralized_debt,
            )
            .unwrap();
    }

    // trying to liquidate without sending funds should fail
    {
        let liquidate_msg = ExecuteMsg::Liquidate {
            user: user_addr.to_string(),
            collateral_denom: "collateral".to_string(),
        };

        let env = mock_env(MockEnvParams::default());
        let info = mock_info(liquidator_addr.as_str(), &[]);
        let error_res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap_err();
        assert_eq!(error_res, PaymentError::NoFunds {}.into());
    }

    // Perform first successful liquidation
    {
        let liquidate_msg = ExecuteMsg::Liquidate {
            user: user_addr.to_string(),
            collateral_denom: "collateral".to_string(),
        };

        let block_time = first_block_time;
        let env = mock_env_at_block_time(block_time);
        let info = mock_info(liquidator_addr.as_str(), &coins(first_debt_to_repay.u128(), "debt"));
        let res = execute(deps.as_mut(), env.clone(), info, liquidate_msg).unwrap();

        // get expected indices and rates for debt market
        let expected_debt_rates = th_get_expected_indices_and_rates(
            &debt_market_initial,
            block_time,
            available_liquidity_debt,
            TestUtilizationDeltaInfo {
                less_debt: first_debt_to_repay,
                user_current_debt_scaled: expected_user_debt_scaled,
                ..Default::default()
            },
        );

        let collateral_market_after = MARKETS.load(&deps.storage, "collateral").unwrap();
        let debt_market_after = MARKETS.load(&deps.storage, "debt").unwrap();

        let expected_liquidated_collateral_amount = math::divide_uint128_by_decimal(
            first_debt_to_repay * debt_price * (Decimal::one() + collateral_liquidation_bonus),
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

        // there should be up to three messages updating indices at the incentives contract, in the
        // order:
        // - collateral denom, user
        // - collatreal denom, liquidator
        // - debt denom, rewards collector (if rewards accrued > 0)
        //
        // NOTE that we don't expect a message to update rewards collector's index of the
        // **collateral** asset, because the liquidation action does NOT change the collateral
        // asset's utilization rate, it's interest rate does not need to be updated.
        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(WasmMsg::Execute {
                    contract_addr: MarsContract::Incentives.to_string(),
                    msg: to_binary(&incentives::msg::ExecuteMsg::BalanceChange {
                        user_addr: user_addr.clone(),
                        denom: collateral_market_initial.denom.clone(),
                        user_amount_scaled_before: expected_user_collateral_scaled,
                        total_amount_scaled_before: collateral_market_initial
                            .collateral_total_scaled,
                    })
                    .unwrap(),
                    funds: vec![]
                }),
                SubMsg::new(WasmMsg::Execute {
                    contract_addr: MarsContract::Incentives.to_string(),
                    msg: to_binary(&incentives::msg::ExecuteMsg::BalanceChange {
                        user_addr: liquidator_addr.clone(),
                        denom: collateral_market_initial.denom.clone(),
                        user_amount_scaled_before: expected_liquidator_collateral_scaled,
                        total_amount_scaled_before: collateral_market_initial
                            .collateral_total_scaled,
                    })
                    .unwrap(),
                    funds: vec![]
                }),
                SubMsg::new(WasmMsg::Execute {
                    contract_addr: MarsContract::Incentives.to_string(),
                    msg: to_binary(&incentives::msg::ExecuteMsg::BalanceChange {
                        user_addr: Addr::unchecked(
                            MarsContract::ProtocolRewardsCollector.to_string()
                        ),
                        denom: debt_market_initial.denom.clone(),
                        user_amount_scaled_before: expected_total_reward_scaled,
                        total_amount_scaled_before: debt_market_initial.collateral_total_scaled,
                    })
                    .unwrap(),
                    funds: vec![]
                }),
            ]
        );

        mars_testing::assert_eq_vec(
            res.attributes,
            vec![
                attr("action", "outposts/red-bank/liquidate"),
                attr("user", user_addr.as_str()),
                attr("liquidator", liquidator_addr.as_str()),
                attr("collateral_denom", "collateral"),
                attr("collateral_amount", expected_liquidated_collateral_amount),
                attr("collateral_amount_scaled", expected_liquidated_collateral_amount_scaled),
                attr("debt_denom", "debt"),
                attr("debt_amount", first_debt_to_repay),
                attr("debt_amount_scaled", expected_debt_rates.less_debt_scaled),
            ],
        );
        assert_eq!(
            res.events,
            vec![th_build_interests_updated_event("debt", &expected_debt_rates)]
        );

        // user's collateral scaled amount should have been correctly decreased
        let collateral =
            COLLATERALS.load(deps.as_ref().storage, (&user_addr, "collateral")).unwrap();
        expected_user_collateral_scaled -= expected_liquidated_collateral_amount_scaled;
        assert_eq!(collateral.amount_scaled, expected_user_collateral_scaled);

        // liquidator's collateral scaled amount should have been correctly increased
        let collateral =
            COLLATERALS.load(deps.as_ref().storage, (&liquidator_addr, "collateral")).unwrap();
        expected_liquidator_collateral_scaled += expected_liquidated_collateral_amount_scaled;
        assert_eq!(collateral.amount_scaled, expected_liquidator_collateral_scaled);

        // check user's debt decreased by the appropriate amount
        let debt = DEBTS.load(&deps.storage, (&user_addr, "debt")).unwrap();
        let expected_less_debt_scaled = expected_debt_rates.less_debt_scaled;
        expected_user_debt_scaled -= expected_less_debt_scaled;
        assert_eq!(expected_user_debt_scaled, debt.amount_scaled);

        // check global debt decreased by the appropriate amount
        expected_global_debt_scaled -= expected_less_debt_scaled;
        assert_eq!(expected_global_debt_scaled, debt_market_after.debt_total_scaled);

        // rewards collector's collateral scaled amount **of the debt asset** should have been correctly increased
        expected_total_reward_scaled += expected_reward_amount_scaled;
        let collateral = COLLATERALS
            .load(
                deps.as_ref().storage,
                (&Addr::unchecked(MarsContract::ProtocolRewardsCollector.to_string()), "debt"),
            )
            .unwrap();
        assert_eq!(collateral.amount_scaled, expected_total_reward_scaled);

        // global collateral scaled amount **of the debt asset** should have been correctly increased
        expected_global_reward_scaled += expected_reward_amount_scaled;
        assert_eq!(debt_market_after.collateral_total_scaled, expected_global_reward_scaled);
    }

    // Perform second successful liquidation sending an excess amount (should refund)
    {
        let liquidate_msg = ExecuteMsg::Liquidate {
            user: user_addr.to_string(),
            collateral_denom: "collateral".to_string(),
        };

        let collateral_market_before = MARKETS.load(&deps.storage, "collateral").unwrap();
        let debt_market_before = MARKETS.load(&deps.storage, "debt").unwrap();

        let block_time = second_block_time;
        let env = mock_env_at_block_time(block_time);
        let info = mock_info(liquidator_addr.as_str(), &coins(second_debt_to_repay.u128(), "debt"));
        let res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap();

        // get expected indices and rates for debt and collateral markets
        let expected_debt_indices = th_get_expected_indices(&debt_market_before, block_time);
        let user_debt_asset_total_debt = compute_underlying_amount(
            expected_user_debt_scaled,
            expected_debt_indices.borrow,
            ScalingOperation::Ceil,
        )
        .unwrap();
        // Since debt is being over_repayed, we expect to max out the liquidatable debt
        let expected_less_debt = user_debt_asset_total_debt * close_factor;

        let expected_refund_amount = second_debt_to_repay - expected_less_debt;

        let expected_debt_rates = th_get_expected_indices_and_rates(
            &debt_market_before,
            block_time,
            available_liquidity_debt, // this is the same as before as it comes from mocks
            TestUtilizationDeltaInfo {
                less_debt: expected_less_debt,
                user_current_debt_scaled: expected_user_debt_scaled,
                less_liquidity: expected_refund_amount,
                ..Default::default()
            },
        );

        let expected_liquidated_collateral_amount = math::divide_uint128_by_decimal(
            expected_less_debt * debt_price * (Decimal::one() + collateral_liquidation_bonus),
            collateral_price,
        )
        .unwrap();

        let expected_collateral_rates = th_get_expected_indices_and_rates(
            &collateral_market_before,
            block_time,
            available_liquidity_collateral, // this is the same as before as it comes from mocks
            TestUtilizationDeltaInfo {
                less_liquidity: expected_liquidated_collateral_amount,
                ..Default::default()
            },
        );

        let debt_market_after = MARKETS.load(&deps.storage, "debt").unwrap();

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

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(WasmMsg::Execute {
                    contract_addr: MarsContract::Incentives.to_string(),
                    msg: to_binary(&incentives::msg::ExecuteMsg::BalanceChange {
                        user_addr: user_addr.clone(),
                        denom: collateral_market_before.denom.clone(),
                        user_amount_scaled_before: expected_user_collateral_scaled,
                        total_amount_scaled_before: collateral_market_before
                            .collateral_total_scaled,
                    })
                    .unwrap(),
                    funds: vec![]
                }),
                SubMsg::new(WasmMsg::Execute {
                    contract_addr: MarsContract::Incentives.to_string(),
                    msg: to_binary(&incentives::msg::ExecuteMsg::BalanceChange {
                        user_addr: liquidator_addr.clone(),
                        denom: collateral_market_before.denom.clone(),
                        user_amount_scaled_before: expected_liquidator_collateral_scaled,
                        total_amount_scaled_before: collateral_market_before
                            .collateral_total_scaled,
                    })
                    .unwrap(),
                    funds: vec![]
                }),
                SubMsg::new(WasmMsg::Execute {
                    contract_addr: MarsContract::Incentives.to_string(),
                    msg: to_binary(&incentives::msg::ExecuteMsg::BalanceChange {
                        user_addr: Addr::unchecked(
                            MarsContract::ProtocolRewardsCollector.to_string()
                        ),
                        denom: debt_market_before.denom.clone(),
                        user_amount_scaled_before: expected_total_reward_scaled,
                        total_amount_scaled_before: debt_market_before.collateral_total_scaled,
                    })
                    .unwrap(),
                    funds: vec![]
                }),
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: liquidator_addr.to_string(),
                    amount: coins(expected_refund_amount.u128(), "debt")
                })),
            ]
        );

        mars_testing::assert_eq_vec(
            vec![
                attr("action", "outposts/red-bank/liquidate"),
                attr("user", user_addr.as_str()),
                attr("liquidator", liquidator_addr.as_str()),
                attr("collateral_denom", "collateral"),
                attr("collateral_amount", expected_liquidated_collateral_amount),
                attr("collateral_amount_scaled", expected_liquidated_collateral_amount_scaled),
                attr("debt_denom", "debt"),
                attr("debt_amount", expected_less_debt),
                attr("debt_amount_scaled", expected_debt_rates.less_debt_scaled),
            ],
            res.attributes,
        );
        assert_eq!(
            res.events,
            vec![th_build_interests_updated_event("debt", &expected_debt_rates)],
        );

        // user's collateral scaled amount should have been correctly decreased
        let collateral =
            COLLATERALS.load(deps.as_ref().storage, (&user_addr, "collateral")).unwrap();
        expected_user_collateral_scaled -= expected_liquidated_collateral_amount_scaled;
        assert_eq!(collateral.amount_scaled, expected_user_collateral_scaled);

        // liquidator's collateral scaled amount should have been correctly increased
        let collateral =
            COLLATERALS.load(deps.as_ref().storage, (&liquidator_addr, "collateral")).unwrap();
        expected_liquidator_collateral_scaled += expected_liquidated_collateral_amount_scaled;
        assert_eq!(collateral.amount_scaled, expected_liquidator_collateral_scaled);

        // check user's debt decreased by the appropriate amount
        let debt = DEBTS.load(&deps.storage, (&user_addr, "debt")).unwrap();
        let expected_less_debt_scaled = expected_debt_rates.less_debt_scaled;
        expected_user_debt_scaled -= expected_less_debt_scaled;
        assert_eq!(expected_user_debt_scaled, debt.amount_scaled);

        // check global debt decreased by the appropriate amount
        expected_global_debt_scaled -= expected_less_debt_scaled;
        assert_eq!(expected_global_debt_scaled, debt_market_after.debt_total_scaled);

        // rewards collector's collateral scaled amount **of the debt asset** should have been correctly increased
        expected_total_reward_scaled += expected_reward_amount_scaled;
        let collateral = COLLATERALS
            .load(
                deps.as_ref().storage,
                (&Addr::unchecked(MarsContract::ProtocolRewardsCollector.to_string()), "debt"),
            )
            .unwrap();
        assert_eq!(collateral.amount_scaled, expected_total_reward_scaled);

        // global collateral scaled amount **of the debt asset** should have been correctly increased
        expected_global_reward_scaled += expected_reward_amount_scaled;
        assert_eq!(debt_market_after.collateral_total_scaled, expected_global_reward_scaled);
    }

    // Perform full liquidation (user should not be able to use asset as collateral)
    {
        let user_collateral_balance_scaled = Uint128::new(100) * SCALING_FACTOR;
        let mut expected_user_debt_scaled = Uint128::new(400) * SCALING_FACTOR;
        let debt_to_repay = Uint128::from(300u128);

        set_collateral(
            deps.as_mut(),
            &user_addr,
            "collateral",
            user_collateral_balance_scaled,
            true,
        );

        // set user to have positive debt amount in debt asset
        let debt = Debt {
            amount_scaled: expected_user_debt_scaled,
            uncollateralized: false,
        };
        DEBTS.save(deps.as_mut().storage, (&user_addr, "debt"), &debt).unwrap();

        let liquidate_msg = ExecuteMsg::Liquidate {
            user: user_addr.to_string(),
            collateral_denom: "collateral".to_string(),
        };

        let collateral_market_before = MARKETS.load(&deps.storage, "collateral").unwrap();
        let debt_market_before = MARKETS.load(&deps.storage, "debt").unwrap();

        // let some time elapse since the last liquidation, so that there is a non-zero amount of
        // protocol rewards accrued
        let block_time = second_block_time + 12345;
        let env = mock_env_at_block_time(block_time);
        let info = mock_info(liquidator_addr.as_str(), &coins(debt_to_repay.u128(), "debt"));
        let res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap();

        // get expected indices and rates for debt and collateral markets
        let expected_collateral_indices =
            th_get_expected_indices(&collateral_market_before, block_time);
        let user_collateral_balance = compute_underlying_amount(
            user_collateral_balance_scaled,
            expected_collateral_indices.liquidity,
            ScalingOperation::Truncate,
        )
        .unwrap();

        // Since debt is being over_repayed, we expect to liquidate total collateral
        let expected_less_debt = math::divide_uint128_by_decimal(
            math::divide_uint128_by_decimal(collateral_price * user_collateral_balance, debt_price)
                .unwrap(),
            Decimal::one() + collateral_liquidation_bonus,
        )
        .unwrap();

        let expected_refund_amount = debt_to_repay - expected_less_debt;

        let expected_debt_rates = th_get_expected_indices_and_rates(
            &debt_market_before,
            block_time,
            available_liquidity_debt, // this is the same as before as it comes from mocks
            TestUtilizationDeltaInfo {
                less_debt: expected_less_debt,
                user_current_debt_scaled: expected_user_debt_scaled,
                less_liquidity: expected_refund_amount,
                ..Default::default()
            },
        );

        let expected_collateral_rates = th_get_expected_indices_and_rates(
            &collateral_market_before,
            block_time,
            available_liquidity_collateral, // this is the same as before as it comes from mocks
            TestUtilizationDeltaInfo {
                less_liquidity: user_collateral_balance,
                ..Default::default()
            },
        );

        let debt_market_after = MARKETS.load(&deps.storage, "debt").unwrap();

        // NOTE: expected_liquidated_collateral_amount_scaled should be equal user_collateral_balance_scaled
        // but there are rounding errors
        let expected_liquidated_collateral_amount_scaled = compute_scaled_amount(
            user_collateral_balance,
            expected_collateral_rates.liquidity_index,
            ScalingOperation::Truncate,
        )
        .unwrap();

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(WasmMsg::Execute {
                    contract_addr: MarsContract::Incentives.to_string(),
                    msg: to_binary(&incentives::msg::ExecuteMsg::BalanceChange {
                        user_addr: user_addr.clone(),
                        denom: collateral_market_before.denom.clone(),
                        user_amount_scaled_before: user_collateral_balance_scaled,
                        total_amount_scaled_before: collateral_market_before
                            .collateral_total_scaled,
                    })
                    .unwrap(),
                    funds: vec![]
                }),
                SubMsg::new(WasmMsg::Execute {
                    contract_addr: MarsContract::Incentives.to_string(),
                    msg: to_binary(&incentives::msg::ExecuteMsg::BalanceChange {
                        user_addr: liquidator_addr.clone(),
                        denom: collateral_market_before.denom.clone(),
                        user_amount_scaled_before: expected_liquidator_collateral_scaled,
                        total_amount_scaled_before: collateral_market_before
                            .collateral_total_scaled,
                    })
                    .unwrap(),
                    funds: vec![]
                }),
                SubMsg::new(WasmMsg::Execute {
                    contract_addr: MarsContract::Incentives.to_string(),
                    msg: to_binary(&incentives::msg::ExecuteMsg::BalanceChange {
                        user_addr: Addr::unchecked(
                            MarsContract::ProtocolRewardsCollector.to_string()
                        ),
                        denom: debt_market_before.denom.clone(),
                        user_amount_scaled_before: expected_total_reward_scaled,
                        total_amount_scaled_before: debt_market_before.collateral_total_scaled,
                    })
                    .unwrap(),
                    funds: vec![]
                }),
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: liquidator_addr.to_string(),
                    amount: coins(expected_refund_amount.u128(), "debt")
                })),
            ]
        );

        mars_testing::assert_eq_vec(
            vec![
                attr("action", "outposts/red-bank/liquidate"),
                attr("user", user_addr.as_str()),
                attr("liquidator", liquidator_addr.as_str()),
                attr("collateral_denom", "collateral"),
                attr("collateral_amount", user_collateral_balance),
                attr("collateral_amount_scaled", expected_liquidated_collateral_amount_scaled),
                attr("debt_denom", "debt"),
                attr("debt_amount", expected_less_debt),
                attr("debt_amount_scaled", expected_debt_rates.less_debt_scaled),
            ],
            res.attributes,
        );
        assert_eq!(
            res.events,
            vec![th_build_interests_updated_event("debt", &expected_debt_rates),]
        );

        // check user doesn't have deposited collateral asset and
        // still has outstanding debt in debt asset
        // TODO: Here the collateral position should be deleted if the contract behaves as designed.
        // However, due to a rounding error described in https://github.com/mars-protocol/outposts/issues/41,
        // the user will still have a dust amount of collateral shares left, leading to the position
        // not being deleted. This will be addresses in a follow-up PR.
        assert!(has_collateral_position(deps.as_ref(), &user_addr, "collateral"));

        // liquidator's collateral scaled amount should have been correctly increased
        let collateral =
            COLLATERALS.load(deps.as_ref().storage, (&liquidator_addr, "collateral")).unwrap();
        expected_liquidator_collateral_scaled += expected_liquidated_collateral_amount_scaled;
        assert_eq!(collateral.amount_scaled, expected_liquidator_collateral_scaled);

        // check user's debt decreased by the appropriate amount
        let debt = DEBTS.load(&deps.storage, (&user_addr, "debt")).unwrap();
        let expected_less_debt_scaled = expected_debt_rates.less_debt_scaled;
        expected_user_debt_scaled -= expected_less_debt_scaled;
        assert_eq!(expected_user_debt_scaled, debt.amount_scaled);

        // check global debt decreased by the appropriate amount
        expected_global_debt_scaled -= expected_less_debt_scaled;
        assert_eq!(expected_global_debt_scaled, debt_market_after.debt_total_scaled);
    }

    // send many native coins
    {
        let env = mock_env(MockEnvParams::default());
        let info = cosmwasm_std::testing::mock_info(
            "liquidator",
            &[coin(100, "somecoin1"), coin(200, "somecoin2")],
        );
        let msg = ExecuteMsg::Liquidate {
            user: user_addr.to_string(),
            collateral_denom: "collateral".to_string(),
        };
        let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(error_res, PaymentError::MultipleDenoms {}.into());
    }
}

#[test]
fn test_liquidate_with_same_asset_for_debt_and_collateral() {
    let denom = "the_asset";

    // Setup
    let available_liquidity = Uint128::from(1_000_000_000u128);
    let mut deps = th_setup(&[coin(available_liquidity.into(), denom)]);

    let user_addr = Addr::unchecked("user");
    let liquidator_addr = Addr::unchecked("liquidator");

    let asset_max_ltv = Decimal::from_ratio(5u128, 10u128);
    let asset_liquidation_threshold = Decimal::from_ratio(6u128, 10u128);
    let asset_liquidation_bonus = Decimal::from_ratio(1u128, 10u128);
    let asset_price = Decimal::from_ratio(2_u128, 1_u128);

    let close_factor = Decimal::from_ratio(1u128, 2u128);

    let initial_user_debt_balance = Uint128::from(3_000_000_u64);
    let initial_user_collateral_scaled = Uint128::from(2_000_000_u64) * SCALING_FACTOR;

    let initial_global_collateral_scaled = Uint128::new(400_000_000) * SCALING_FACTOR;
    let initial_global_debt_scaled = Uint128::new(500_000_000) * SCALING_FACTOR;

    let liquidation_block_time = 15_000_000;

    CONFIG
        .update(deps.as_mut().storage, |mut config| -> StdResult<_> {
            config.close_factor = close_factor;
            Ok(config)
        })
        .unwrap();

    // initialize market
    deps.querier.set_oracle_price(denom, asset_price);

    let asset_market = Market {
        max_loan_to_value: asset_max_ltv,
        liquidation_threshold: asset_liquidation_threshold,
        liquidation_bonus: asset_liquidation_bonus,
        collateral_total_scaled: initial_global_collateral_scaled,
        debt_total_scaled: initial_global_debt_scaled,
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::one(),
        borrow_rate: Decimal::from_ratio(2u128, 10u128),
        liquidity_rate: Decimal::from_ratio(2u128, 10u128),
        reserve_factor: Decimal::from_ratio(2u128, 100u128),
        indexes_last_updated: 0,
        interest_rate_model: InterestRateModel {
            optimal_utilization_rate: Decimal::from_ratio(80u128, 100u128),
            base: Decimal::from_ratio(0u128, 100u128),
            slope_1: Decimal::from_ratio(10u128, 100u128),
            slope_2: Decimal::one(),
        },
        ..Default::default()
    };

    let asset_market_initial = th_init_market(deps.as_mut(), denom, &asset_market);

    let initial_user_debt_scaled = compute_scaled_amount(
        initial_user_debt_balance,
        asset_market_initial.borrow_index,
        ScalingOperation::Ceil,
    )
    .unwrap();

    // Create collateral position for the user
    set_collateral(
        deps.as_mut(),
        &user_addr,
        &asset_market_initial.denom,
        initial_user_collateral_scaled,
        true,
    );

    // set user to have positive debt amount in debt asset
    {
        let debt = Debt {
            amount_scaled: initial_user_debt_scaled,
            uncollateralized: false,
        };
        DEBTS.save(deps.as_mut().storage, (&user_addr, denom), &debt).unwrap();
    }

    // Perform partial liquidation
    {
        let debt_to_repay = Uint128::from(400_000_u64);
        let liquidate_msg = ExecuteMsg::Liquidate {
            user: user_addr.to_string(),
            collateral_denom: denom.to_string(),
        };

        let asset_market_before = MARKETS.load(&deps.storage, denom).unwrap();

        let block_time = liquidation_block_time;
        let env = mock_env_at_block_time(block_time);
        let info = cosmwasm_std::testing::mock_info(
            liquidator_addr.as_str(),
            &[coin(debt_to_repay.into(), denom)],
        );
        let res = execute(deps.as_mut(), env.clone(), info, liquidate_msg).unwrap();

        // get expected indices and rates for debt market
        let expected_rates = th_get_expected_indices_and_rates(
            &asset_market_before,
            block_time,
            available_liquidity,
            TestUtilizationDeltaInfo {
                less_debt: debt_to_repay,
                user_current_debt_scaled: initial_user_debt_scaled,
                ..Default::default()
            },
        );

        let asset_market_after = MARKETS.load(&deps.storage, denom).unwrap();

        let expected_liquidated_amount = math::divide_uint128_by_decimal(
            debt_to_repay * asset_price * (Decimal::one() + asset_liquidation_bonus),
            asset_price,
        )
        .unwrap();

        let expected_liquidated_amount_scaled = get_scaled_liquidity_amount(
            expected_liquidated_amount,
            &asset_market_after,
            env.block.time.seconds(),
        )
        .unwrap();

        let expected_reward_amount_scaled = compute_scaled_amount(
            expected_rates.protocol_rewards_to_distribute,
            expected_rates.liquidity_index,
            ScalingOperation::Truncate,
        )
        .unwrap();

        // there should be three messages updating indices at the incentives contract, in the order:
        // - rewards collector
        // - user
        // - liquidator
        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(WasmMsg::Execute {
                    contract_addr: MarsContract::Incentives.to_string(),
                    msg: to_binary(&incentives::msg::ExecuteMsg::BalanceChange {
                        user_addr: user_addr.clone(),
                        denom: asset_market_before.denom.clone(),
                        user_amount_scaled_before: initial_user_collateral_scaled,
                        total_amount_scaled_before: asset_market_before.collateral_total_scaled,
                    })
                    .unwrap(),
                    funds: vec![]
                }),
                SubMsg::new(WasmMsg::Execute {
                    contract_addr: MarsContract::Incentives.to_string(),
                    msg: to_binary(&incentives::msg::ExecuteMsg::BalanceChange {
                        user_addr: liquidator_addr.clone(),
                        denom: asset_market_before.denom.clone(),
                        user_amount_scaled_before: Uint128::zero(),
                        total_amount_scaled_before: asset_market_before.collateral_total_scaled,
                    })
                    .unwrap(),
                    funds: vec![]
                }),
                SubMsg::new(WasmMsg::Execute {
                    contract_addr: MarsContract::Incentives.to_string(),
                    msg: to_binary(&incentives::msg::ExecuteMsg::BalanceChange {
                        user_addr: Addr::unchecked(
                            MarsContract::ProtocolRewardsCollector.to_string()
                        ),
                        denom: asset_market_before.denom.clone(),
                        user_amount_scaled_before: Uint128::zero(),
                        total_amount_scaled_before: asset_market_before.collateral_total_scaled,
                    })
                    .unwrap(),
                    funds: vec![]
                }),
            ]
        );

        mars_testing::assert_eq_vec(
            res.attributes,
            vec![
                attr("action", "outposts/red-bank/liquidate"),
                attr("user", user_addr.as_str()),
                attr("liquidator", liquidator_addr.as_str()),
                attr("collateral_denom", denom),
                attr("collateral_amount", expected_liquidated_amount),
                attr("collateral_amount_scaled", expected_liquidated_amount_scaled),
                attr("debt_denom", denom),
                attr("debt_amount", debt_to_repay),
                attr("debt_amount_scaled", expected_rates.less_debt_scaled),
            ],
        );
        assert_eq!(res.events, vec![th_build_interests_updated_event(denom, &expected_rates)]);

        // user's collateral scaled amount should have been correctly decreased
        let collateral =
            COLLATERALS.load(deps.as_ref().storage, (&user_addr, "the_asset")).unwrap();
        let expected_user_collateral_scaled =
            initial_user_collateral_scaled - expected_liquidated_amount_scaled;
        assert_eq!(collateral.amount_scaled, expected_user_collateral_scaled);

        // liquidator's collateral scaled amount should have been correctly increased
        let collateral =
            COLLATERALS.load(deps.as_ref().storage, (&liquidator_addr, "the_asset")).unwrap();
        assert_eq!(collateral.amount_scaled, expected_liquidated_amount_scaled);

        // rewards collector's collateral scaled amount **of the debt asset** should have been correctly increased
        let collateral = COLLATERALS
            .load(
                deps.as_ref().storage,
                (&Addr::unchecked(MarsContract::ProtocolRewardsCollector.to_string()), "the_asset"),
            )
            .unwrap();
        assert_eq!(collateral.amount_scaled, expected_reward_amount_scaled);

        // check user's debt decreased by the appropriate amount
        let debt = DEBTS.load(&deps.storage, (&user_addr, denom)).unwrap();
        let expected_user_debt_scaled = initial_user_debt_scaled - expected_rates.less_debt_scaled;
        assert_eq!(expected_user_debt_scaled, debt.amount_scaled);

        // global collateral scaled amount **of the debt asset** should have been correctly increased
        let expected_global_collateral_scaled =
            initial_global_collateral_scaled + expected_reward_amount_scaled;
        assert_eq!(asset_market_after.collateral_total_scaled, expected_global_collateral_scaled);

        // check global debt decreased by the appropriate amount
        let expected_global_debt_scaled =
            initial_global_debt_scaled - expected_rates.less_debt_scaled;
        assert_eq!(expected_global_debt_scaled, asset_market_after.debt_total_scaled);
    }

    // Reset state for next test
    {
        // user debt
        let debt = Debt {
            amount_scaled: initial_user_debt_scaled,
            uncollateralized: false,
        };
        DEBTS.save(deps.as_mut().storage, (&user_addr, denom), &debt).unwrap();

        // user collateral
        set_collateral(
            deps.as_mut(),
            &user_addr,
            &asset_market_initial.denom,
            initial_user_collateral_scaled,
            true,
        );

        // liquidator and collector collateral
        unset_collateral(deps.as_mut(), &liquidator_addr, &asset_market_initial.denom);
        unset_collateral(
            deps.as_mut(),
            &Addr::unchecked(MarsContract::ProtocolRewardsCollector.to_string()),
            &asset_market_initial.denom,
        );

        MARKETS.save(deps.as_mut().storage, denom, &asset_market_initial).unwrap();
    }

    // Perform overpaid liquidation
    {
        let block_time = liquidation_block_time;
        // Since debt is being over repayed, we expect to max out the liquidatable debt
        // get expected indices and rates for debt and collateral markets
        let expected_indices = th_get_expected_indices(&asset_market_initial, block_time);
        let user_debt_balance_before = compute_underlying_amount(
            initial_user_debt_scaled,
            expected_indices.borrow,
            ScalingOperation::Ceil,
        )
        .unwrap();
        let debt_to_repay = user_debt_balance_before;
        let expected_less_debt = user_debt_balance_before * close_factor;
        let expected_refund_amount = debt_to_repay - expected_less_debt;

        let liquidate_msg = ExecuteMsg::Liquidate {
            user: user_addr.to_string(),
            collateral_denom: denom.to_string(),
        };

        let asset_market_before = MARKETS.load(&deps.storage, denom).unwrap();

        let env = mock_env_at_block_time(block_time);
        let info = cosmwasm_std::testing::mock_info(
            liquidator_addr.as_str(),
            &coins(debt_to_repay.u128(), denom),
        );
        let res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap();

        let asset_market_after = MARKETS.load(&deps.storage, denom).unwrap();
        let expected_liquidated_amount = math::divide_uint128_by_decimal(
            expected_less_debt * asset_price * (Decimal::one() + asset_liquidation_bonus),
            asset_price,
        )
        .unwrap();

        // get expected indices and rates for debt market
        let expected_rates = th_get_expected_indices_and_rates(
            &asset_market_before,
            block_time,
            available_liquidity,
            TestUtilizationDeltaInfo {
                less_debt: expected_less_debt,
                less_liquidity: expected_refund_amount,
                user_current_debt_scaled: initial_user_debt_scaled,
                ..Default::default()
            },
        );

        let expected_liquidated_amount_scaled = compute_scaled_amount(
            expected_liquidated_amount,
            expected_rates.liquidity_index,
            ScalingOperation::Truncate,
        )
        .unwrap();

        let expected_reward_amount_scaled = compute_scaled_amount(
            expected_rates.protocol_rewards_to_distribute,
            expected_rates.liquidity_index,
            ScalingOperation::Truncate,
        )
        .unwrap();

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(WasmMsg::Execute {
                    contract_addr: MarsContract::Incentives.to_string(),
                    msg: to_binary(&incentives::msg::ExecuteMsg::BalanceChange {
                        user_addr: user_addr.clone(),
                        denom: asset_market_before.denom.clone(),
                        user_amount_scaled_before: initial_user_collateral_scaled,
                        total_amount_scaled_before: asset_market_before.collateral_total_scaled,
                    })
                    .unwrap(),
                    funds: vec![]
                }),
                SubMsg::new(WasmMsg::Execute {
                    contract_addr: MarsContract::Incentives.to_string(),
                    msg: to_binary(&incentives::msg::ExecuteMsg::BalanceChange {
                        user_addr: liquidator_addr.clone(),
                        denom: asset_market_before.denom.clone(),
                        user_amount_scaled_before: Uint128::zero(),
                        total_amount_scaled_before: asset_market_before.collateral_total_scaled,
                    })
                    .unwrap(),
                    funds: vec![]
                }),
                SubMsg::new(WasmMsg::Execute {
                    contract_addr: MarsContract::Incentives.to_string(),
                    msg: to_binary(&incentives::msg::ExecuteMsg::BalanceChange {
                        user_addr: Addr::unchecked(
                            MarsContract::ProtocolRewardsCollector.to_string()
                        ),
                        denom: asset_market_before.denom.clone(),
                        user_amount_scaled_before: Uint128::zero(),
                        total_amount_scaled_before: asset_market_before.collateral_total_scaled,
                    })
                    .unwrap(),
                    funds: vec![]
                }),
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: liquidator_addr.to_string(),
                    amount: coins(expected_refund_amount.u128(), denom)
                })),
            ]
        );

        mars_testing::assert_eq_vec(
            res.attributes,
            vec![
                attr("action", "outposts/red-bank/liquidate"),
                attr("user", user_addr.as_str()),
                attr("liquidator", liquidator_addr.as_str()),
                attr("collateral_denom", denom),
                attr("collateral_amount", expected_liquidated_amount),
                attr("collateral_amount_scaled", expected_liquidated_amount_scaled),
                attr("debt_denom", denom),
                attr("debt_amount", expected_less_debt),
                attr("debt_amount_scaled", expected_rates.less_debt_scaled),
            ],
        );
        assert_eq!(res.events, vec![th_build_interests_updated_event(denom, &expected_rates),],);

        // user's collateral scaled amount should have been correctly decreased
        let collateral =
            COLLATERALS.load(deps.as_ref().storage, (&user_addr, "the_asset")).unwrap();
        let expected_user_collateral_scaled =
            initial_user_collateral_scaled - expected_liquidated_amount_scaled;
        assert_eq!(collateral.amount_scaled, expected_user_collateral_scaled);

        // liquidator's collateral scaled amount should have been correctly increased
        let collateral =
            COLLATERALS.load(deps.as_ref().storage, (&liquidator_addr, "the_asset")).unwrap();
        assert_eq!(collateral.amount_scaled, expected_liquidated_amount_scaled);

        // rewards collector's collateral scaled amount **of the debt asset** should have been correctly increased
        let collateral = COLLATERALS
            .load(
                deps.as_ref().storage,
                (&Addr::unchecked(MarsContract::ProtocolRewardsCollector.to_string()), "the_asset"),
            )
            .unwrap();
        assert_eq!(collateral.amount_scaled, expected_reward_amount_scaled);

        // check user's debt decreased by the appropriate amount
        let debt = DEBTS.load(&deps.storage, (&user_addr, denom)).unwrap();
        let expected_user_debt_scaled = initial_user_debt_scaled - expected_rates.less_debt_scaled;
        assert_eq!(expected_user_debt_scaled, debt.amount_scaled);

        // global collateral scaled amount **of the debt asset** should have been correctly increased
        let expected_global_collateral_scaled =
            initial_global_collateral_scaled + expected_reward_amount_scaled;
        assert_eq!(asset_market_after.collateral_total_scaled, expected_global_collateral_scaled);

        // check global debt decreased by the appropriate amount
        let expected_global_debt_scaled =
            initial_global_debt_scaled - expected_rates.less_debt_scaled;
        assert_eq!(expected_global_debt_scaled, asset_market_after.debt_total_scaled);
    }
}

#[test]
fn test_liquidation_health_factor_check() {
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
    };

    let env = mock_env(MockEnvParams::default());
    let info = mock_info(liquidator_addr.as_str(), &coins(debt_to_cover.u128(), "debt"));
    let error_res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap_err();
    assert_eq!(error_res, ContractError::CannotLiquidateHealthyPosition {});
}

#[test]
fn test_liquidate_if_collateral_disabled() {
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
