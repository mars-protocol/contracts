use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{
    attr, coin, coins, to_binary, Addr, BankMsg, CosmosMsg, Decimal, StdError, StdResult, SubMsg,
    Uint128, WasmMsg,
};
use cw_utils::PaymentError;

use mars_outpost::red_bank::{Debt, ExecuteMsg, InterestRateModel, Market, User};
use mars_outpost::{ma_token, math};
use mars_testing::{mock_env, mock_env_at_block_time, MockEnvParams};

use mars_red_bank::contract::execute;
use mars_red_bank::error::ContractError;
use mars_red_bank::events::build_collateral_position_changed_event;
use mars_red_bank::helpers::{get_bit, set_bit};
use mars_red_bank::interest_rates::{
    compute_scaled_amount, compute_underlying_amount, get_scaled_liquidity_amount,
    ScalingOperation, SCALING_FACTOR,
};
use mars_red_bank::state::{CONFIG, DEBTS, MARKETS, USERS};

use helpers::{
    th_build_interests_updated_event, th_get_expected_indices, th_get_expected_indices_and_rates,
    th_init_market, th_setup, TestUtilizationDeltaInfo,
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

    let user_address = Addr::unchecked("user");
    let liquidator_address = Addr::unchecked("liquidator");

    let collateral_max_ltv = Decimal::from_ratio(5u128, 10u128);
    let collateral_liquidation_threshold = Decimal::from_ratio(6u128, 10u128);
    let collateral_liquidation_bonus = Decimal::from_ratio(1u128, 10u128);
    let collateral_price = Decimal::from_ratio(2_u128, 1_u128);
    let debt_price = Decimal::from_ratio(11_u128, 10_u128);
    let umars_price = Decimal::from_ratio(15_u128, 10_u128);
    let user_collateral_balance = 2_000_000;
    let user_debt = Uint128::from(3_000_000_u64); // ltv = 0.75
    let close_factor = Decimal::from_ratio(1u128, 2u128);

    let first_debt_to_repay = Uint128::from(400_000_u64);
    let first_block_time = 15_000_000;

    let second_debt_to_repay = Uint128::from(10_000_000_u64);
    let second_block_time = 16_000_000;

    // Global debt for the debt market
    let mut expected_global_debt_scaled = Uint128::new(1_800_000_000) * SCALING_FACTOR;

    CONFIG
        .update(deps.as_mut().storage, |mut config| -> StdResult<_> {
            config.close_factor = close_factor;
            Ok(config)
        })
        .unwrap();

    // initialize collateral and debt markets

    deps.querier.set_oracle_price("collateral", collateral_price);
    deps.querier.set_oracle_price("debt", debt_price);
    deps.querier.set_oracle_price("umars", umars_price);

    // for the test to pass, we need an interest rate model that gives non-zero rates
    let mock_ir_model = InterestRateModel {
        optimal_utilization_rate: Decimal::one(),
        base: Decimal::percent(5),
        slope_1: Decimal::zero(),
        slope_2: Decimal::zero(),
    };

    let collateral_market_ma_token_addr = Addr::unchecked("ma_collateral");
    let collateral_market = Market {
        ma_token_address: collateral_market_ma_token_addr.clone(),
        max_loan_to_value: collateral_max_ltv,
        liquidation_threshold: collateral_liquidation_threshold,
        liquidation_bonus: collateral_liquidation_bonus,
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
        ma_token_address: Addr::unchecked("ma_debt"),
        max_loan_to_value: Decimal::from_ratio(6u128, 10u128),
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

    let collateral_market_initial = th_init_market(deps.as_mut(), "collateral", &collateral_market);
    let debt_market_initial = th_init_market(deps.as_mut(), "debt", &debt_market);

    let mut expected_user_debt_scaled =
        compute_scaled_amount(user_debt, debt_market_initial.borrow_index, ScalingOperation::Ceil)
            .unwrap();

    // Set user as having collateral and debt in respective markets
    {
        let mut user = User::default();
        set_bit(&mut user.collateral_assets, collateral_market_initial.index).unwrap();
        set_bit(&mut user.borrowed_assets, debt_market_initial.index).unwrap();
        USERS.save(deps.as_mut().storage, &user_address, &user).unwrap();
    }

    // trying to liquidate user with zero collateral balance should fail
    {
        deps.querier.set_cw20_balances(
            collateral_market_ma_token_addr.clone(),
            &[(user_address.clone(), Uint128::zero())],
        );

        let liquidate_msg = ExecuteMsg::Liquidate {
            user: user_address.to_string(),
            collateral_denom: "collateral".to_string(),
        };

        let env = mock_env(MockEnvParams::default());
        let info =
            mock_info(liquidator_address.as_str(), &coins(first_debt_to_repay.u128(), "debt"));
        let error_res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap_err();
        assert_eq!(error_res, ContractError::CannotLiquidateWhenNoCollateralBalance {});
    }

    // Set the querier to return positive collateral balance
    deps.querier.set_cw20_balances(
        collateral_market_ma_token_addr.clone(),
        &[(user_address.clone(), Uint128::new(user_collateral_balance) * SCALING_FACTOR)],
    );

    // trying to liquidate user with zero outstanding debt should fail (uncollateralized has not impact)
    {
        let debt = Debt {
            amount_scaled: Uint128::zero(),
            uncollateralized: false,
        };
        let uncollateralized_debt = Debt {
            amount_scaled: Uint128::new(10_000) * SCALING_FACTOR,
            uncollateralized: true,
        };
        DEBTS.save(deps.as_mut().storage, ("debt", &user_address), &debt).unwrap();
        DEBTS
            .save(
                deps.as_mut().storage,
                ("uncollateralized_debt", &user_address),
                &uncollateralized_debt,
            )
            .unwrap();

        let liquidate_msg = ExecuteMsg::Liquidate {
            user: user_address.to_string(),
            collateral_denom: "collateral".to_string(),
        };

        let env = mock_env(MockEnvParams::default());
        let info =
            mock_info(liquidator_address.as_str(), &coins(first_debt_to_repay.u128(), "debt"));
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
        DEBTS.save(deps.as_mut().storage, ("debt", &user_address), &debt).unwrap();
        DEBTS
            .save(
                deps.as_mut().storage,
                ("uncollateralized_debt", &user_address),
                &uncollateralized_debt,
            )
            .unwrap();
    }

    // trying to liquidate without sending funds should fail
    {
        let liquidate_msg = ExecuteMsg::Liquidate {
            user: user_address.to_string(),
            collateral_denom: "collateral".to_string(),
        };

        let env = mock_env(MockEnvParams::default());
        let info = mock_info(liquidator_address.as_str(), &[]);
        let error_res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap_err();
        assert_eq!(error_res, PaymentError::NoFunds {}.into());
    }

    // Perform first successful liquidation
    {
        let liquidate_msg = ExecuteMsg::Liquidate {
            user: user_address.to_string(),
            collateral_denom: "collateral".to_string(),
        };

        let collateral_market_before = MARKETS.load(&deps.storage, "collateral").unwrap();
        let debt_market_before = MARKETS.load(&deps.storage, "debt").unwrap();

        let block_time = first_block_time;
        let env = mock_env_at_block_time(block_time);
        let info =
            mock_info(liquidator_address.as_str(), &coins(first_debt_to_repay.u128(), "debt"));
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

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: collateral_market_ma_token_addr.to_string(),
                    msg: to_binary(
                        &mars_outpost::ma_token::msg::ExecuteMsg::TransferOnLiquidation {
                            sender: user_address.to_string(),
                            recipient: liquidator_address.to_string(),
                            amount: expected_liquidated_collateral_amount_scaled,
                        }
                    )
                    .unwrap(),
                    funds: vec![]
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: debt_market.ma_token_address.to_string(),
                    msg: to_binary(&ma_token::msg::ExecuteMsg::Mint {
                        recipient: "protocol_rewards_collector".to_string(),
                        amount: compute_scaled_amount(
                            expected_debt_rates.protocol_rewards_to_distribute,
                            expected_debt_rates.liquidity_index,
                            ScalingOperation::Truncate
                        )
                        .unwrap(),
                    })
                    .unwrap(),
                    funds: vec![]
                })),
            ]
        );

        mars_testing::assert_eq_vec(
            res.attributes,
            vec![
                attr("action", "outposts/red-bank/liquidate"),
                attr("collateral_denom", "collateral"),
                attr("debt_denom", "debt"),
                attr("user", user_address.as_str()),
                attr("liquidator", liquidator_address.as_str()),
                attr(
                    "collateral_amount_liquidated",
                    expected_liquidated_collateral_amount.to_string(),
                ),
                attr("debt_amount_repaid", first_debt_to_repay.to_string()),
                attr("refund_amount", "0"),
            ],
        );
        assert_eq!(
            res.events,
            vec![
                build_collateral_position_changed_event(
                    "collateral",
                    true,
                    liquidator_address.to_string()
                ),
                th_build_interests_updated_event("debt", &expected_debt_rates)
            ]
        );

        // check user still has deposited collateral asset and
        // still has outstanding debt in debt asset
        let user = USERS.load(&deps.storage, &user_address).unwrap();
        assert!(get_bit(user.collateral_assets, collateral_market_before.index).unwrap());
        assert!(get_bit(user.borrowed_assets, debt_market_before.index).unwrap());

        // check user's debt decreased by the appropriate amount
        let debt = DEBTS.load(&deps.storage, ("debt", &user_address)).unwrap();
        let expected_less_debt_scaled = expected_debt_rates.less_debt_scaled;
        expected_user_debt_scaled -= expected_less_debt_scaled;
        assert_eq!(expected_user_debt_scaled, debt.amount_scaled);

        // check global debt decreased by the appropriate amount
        expected_global_debt_scaled -= expected_less_debt_scaled;
        assert_eq!(expected_global_debt_scaled, debt_market_after.debt_total_scaled);
    }

    // Perform second successful liquidation sending an excess amount (should refund)
    {
        let liquidate_msg = ExecuteMsg::Liquidate {
            user: user_address.to_string(),
            collateral_denom: "collateral".to_string(),
        };

        let collateral_market_before = MARKETS.load(&deps.storage, "collateral").unwrap();
        let debt_market_before = MARKETS.load(&deps.storage, "debt").unwrap();

        let block_time = second_block_time;
        let env = mock_env_at_block_time(block_time);
        let info =
            mock_info(liquidator_address.as_str(), &coins(second_debt_to_repay.u128(), "debt"));
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

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: collateral_market_ma_token_addr.to_string(),
                    msg: to_binary(
                        &mars_outpost::ma_token::msg::ExecuteMsg::TransferOnLiquidation {
                            sender: user_address.to_string(),
                            recipient: liquidator_address.to_string(),
                            amount: expected_liquidated_collateral_amount_scaled,
                        }
                    )
                    .unwrap(),
                    funds: vec![]
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: debt_market.ma_token_address.to_string(),
                    msg: to_binary(&ma_token::msg::ExecuteMsg::Mint {
                        recipient: "protocol_rewards_collector".to_string(),
                        amount: compute_scaled_amount(
                            expected_debt_rates.protocol_rewards_to_distribute,
                            expected_debt_rates.liquidity_index,
                            ScalingOperation::Truncate
                        )
                        .unwrap(),
                    })
                    .unwrap(),
                    funds: vec![]
                })),
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: liquidator_address.to_string(),
                    amount: coins(expected_refund_amount.u128(), "debt")
                })),
            ]
        );

        mars_testing::assert_eq_vec(
            vec![
                attr("action", "outposts/red-bank/liquidate"),
                attr("collateral_denom", "collateral"),
                attr("debt_denom", "debt"),
                attr("user", user_address.as_str()),
                attr("liquidator", liquidator_address.as_str()),
                attr("collateral_amount_liquidated", expected_liquidated_collateral_amount),
                attr("debt_amount_repaid", expected_less_debt.to_string()),
                attr("refund_amount", expected_refund_amount.to_string()),
            ],
            res.attributes,
        );
        assert_eq!(
            res.events,
            vec![th_build_interests_updated_event("debt", &expected_debt_rates)],
        );

        // check user still has deposited collateral asset and
        // still has outstanding debt in debt asset
        let user = USERS.load(&deps.storage, &user_address).unwrap();
        assert!(get_bit(user.collateral_assets, collateral_market_initial.index).unwrap());
        assert!(get_bit(user.borrowed_assets, debt_market_initial.index).unwrap());

        // check user's debt decreased by the appropriate amount
        let debt = DEBTS.load(&deps.storage, ("debt", &user_address)).unwrap();
        let expected_less_debt_scaled = expected_debt_rates.less_debt_scaled;
        expected_user_debt_scaled -= expected_less_debt_scaled;
        assert_eq!(expected_user_debt_scaled, debt.amount_scaled);

        // check global debt decreased by the appropriate amount
        expected_global_debt_scaled -= expected_less_debt_scaled;
        assert_eq!(expected_global_debt_scaled, debt_market_after.debt_total_scaled);
    }

    // Perform full liquidation (user should not be able to use asset as collateral)
    {
        let user_collateral_balance_scaled = Uint128::new(100) * SCALING_FACTOR;
        let mut expected_user_debt_scaled = Uint128::new(400) * SCALING_FACTOR;
        let debt_to_repay = Uint128::from(300u128);

        // Set the querier to return positive collateral balance
        deps.querier.set_cw20_balances(
            collateral_market_ma_token_addr.clone(),
            &[(user_address.clone(), user_collateral_balance_scaled)],
        );

        // set user to have positive debt amount in debt asset
        let debt = Debt {
            amount_scaled: expected_user_debt_scaled,
            uncollateralized: false,
        };
        DEBTS.save(deps.as_mut().storage, ("debt", &user_address), &debt).unwrap();

        let liquidate_msg = ExecuteMsg::Liquidate {
            user: user_address.to_string(),
            collateral_denom: "collateral".to_string(),
        };

        let collateral_market_before = MARKETS.load(&deps.storage, "collateral").unwrap();
        let debt_market_before = MARKETS.load(&deps.storage, "debt").unwrap();

        let block_time = second_block_time;
        let env = mock_env_at_block_time(block_time);
        let info = mock_info(liquidator_address.as_str(), &coins(debt_to_repay.u128(), "debt"));
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
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: collateral_market_ma_token_addr.to_string(),
                    msg: to_binary(
                        &mars_outpost::ma_token::msg::ExecuteMsg::TransferOnLiquidation {
                            sender: user_address.to_string(),
                            recipient: liquidator_address.to_string(),
                            amount: expected_liquidated_collateral_amount_scaled,
                        }
                    )
                    .unwrap(),
                    funds: vec![]
                })),
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: liquidator_address.to_string(),
                    amount: coins(expected_refund_amount.u128(), "debt")
                }))
            ]
        );

        mars_testing::assert_eq_vec(
            vec![
                attr("action", "outposts/red-bank/liquidate"),
                attr("collateral_denom", "collateral"),
                attr("debt_denom", "debt"),
                attr("user", user_address.as_str()),
                attr("liquidator", liquidator_address.as_str()),
                attr("collateral_amount_liquidated", user_collateral_balance.to_string()),
                attr("debt_amount_repaid", expected_less_debt.to_string()),
                attr("refund_amount", expected_refund_amount.to_string()),
            ],
            res.attributes,
        );
        assert_eq!(
            res.events,
            vec![
                build_collateral_position_changed_event(
                    "collateral",
                    false,
                    user_address.to_string()
                ),
                th_build_interests_updated_event("debt", &expected_debt_rates),
            ]
        );

        // check user doesn't have deposited collateral asset and
        // still has outstanding debt in debt asset
        let user = USERS.load(&deps.storage, &user_address).unwrap();
        assert!(!get_bit(user.collateral_assets, collateral_market_initial.index).unwrap());
        assert!(get_bit(user.borrowed_assets, debt_market_initial.index).unwrap());

        // check user's debt decreased by the appropriate amount
        let debt = DEBTS.load(&deps.storage, ("debt", &user_address)).unwrap();
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
            user: user_address.to_string(),
            collateral_denom: "collateral".to_string(),
        };
        let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(error_res, PaymentError::MultipleDenoms {}.into());
    }
}

#[test]
fn test_liquidate_with_same_asset_for_debt_and_collateral() {
    // Setup
    let available_liquidity = Uint128::from(1_000_000_000u128);
    let mut deps = th_setup(&[coin(available_liquidity.into(), "the_asset")]);

    let user_address = Addr::unchecked("user");
    let liquidator_address = Addr::unchecked("liquidator");
    let ma_token_address = Addr::unchecked("mathe_asset");

    let asset_max_ltv = Decimal::from_ratio(5u128, 10u128);
    let asset_liquidation_threshold = Decimal::from_ratio(6u128, 10u128);
    let asset_liquidation_bonus = Decimal::from_ratio(1u128, 10u128);
    let asset_price = Decimal::from_ratio(2_u128, 1_u128);

    let initial_user_debt_balance = Uint128::from(3_000_000_u64);
    // NOTE: this should change in practice but it will stay static on this test
    // as the balance is mocked and does not get updated
    let user_collateral_balance = Uint128::from(2_000_000_u64);

    let close_factor = Decimal::from_ratio(1u128, 2u128);

    // Global debt for the market (starts at index 1.000000000...)
    let initial_global_debt_scaled = Uint128::new(500_000_000) * SCALING_FACTOR;
    let liquidation_block_time = 15_000_000;

    CONFIG
        .update(deps.as_mut().storage, |mut config| -> StdResult<_> {
            config.close_factor = close_factor;
            Ok(config)
        })
        .unwrap();

    // initialize market
    deps.querier.set_oracle_price("the_asset", asset_price);

    let asset_market = Market {
        ma_token_address: ma_token_address.clone(),
        max_loan_to_value: asset_max_ltv,
        liquidation_threshold: asset_liquidation_threshold,
        liquidation_bonus: asset_liquidation_bonus,
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

    let asset_market_initial = th_init_market(deps.as_mut(), "the_asset", &asset_market);

    let initial_user_debt_scaled = compute_scaled_amount(
        initial_user_debt_balance,
        asset_market_initial.borrow_index,
        ScalingOperation::Ceil,
    )
    .unwrap();

    // Set user as having collateral and debt in market
    let mut user = User::default();
    set_bit(&mut user.collateral_assets, asset_market_initial.index).unwrap();
    set_bit(&mut user.borrowed_assets, asset_market_initial.index).unwrap();
    USERS.save(deps.as_mut().storage, &user_address, &user).unwrap();

    // Set the querier to return positive collateral balance
    deps.querier.set_cw20_balances(
        ma_token_address.clone(),
        &[(user_address.clone(), user_collateral_balance * SCALING_FACTOR)],
    );

    // set user to have positive debt amount in debt asset
    {
        let debt = Debt {
            amount_scaled: initial_user_debt_scaled,
            uncollateralized: false,
        };
        DEBTS.save(deps.as_mut().storage, ("the_asset", &user_address), &debt).unwrap();
    }

    // Perform partial liquidation
    {
        let debt_to_repay = Uint128::from(400_000_u64);
        let liquidate_msg = ExecuteMsg::Liquidate {
            user: user_address.to_string(),
            collateral_denom: "the_asset".to_string(),
        };

        let asset_market_before = MARKETS.load(&deps.storage, "the_asset").unwrap();

        let block_time = liquidation_block_time;
        let env = mock_env_at_block_time(block_time);
        let info = cosmwasm_std::testing::mock_info(
            liquidator_address.as_str(),
            &[coin(debt_to_repay.into(), "the_asset")],
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

        let asset_market_after = MARKETS.load(&deps.storage, "the_asset").unwrap();

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

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: ma_token_address.to_string(),
                    msg: to_binary(
                        &mars_outpost::ma_token::msg::ExecuteMsg::TransferOnLiquidation {
                            sender: user_address.to_string(),
                            recipient: liquidator_address.to_string(),
                            amount: expected_liquidated_amount_scaled,
                        }
                    )
                    .unwrap(),
                    funds: vec![]
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: ma_token_address.to_string(),
                    msg: to_binary(&ma_token::msg::ExecuteMsg::Mint {
                        recipient: "protocol_rewards_collector".to_string(),
                        amount: compute_scaled_amount(
                            expected_rates.protocol_rewards_to_distribute,
                            expected_rates.liquidity_index,
                            ScalingOperation::Truncate
                        )
                        .unwrap(),
                    })
                    .unwrap(),
                    funds: vec![]
                })),
            ]
        );

        mars_testing::assert_eq_vec(
            res.attributes,
            vec![
                attr("action", "outposts/red-bank/liquidate"),
                attr("collateral_denom", "the_asset"),
                attr("debt_denom", "the_asset"),
                attr("user", user_address.as_str()),
                attr("liquidator", liquidator_address.as_str()),
                attr("collateral_amount_liquidated", expected_liquidated_amount.to_string()),
                attr("debt_amount_repaid", debt_to_repay.to_string()),
                attr("refund_amount", "0"),
            ],
        );
        assert_eq!(
            res.events,
            vec![
                build_collateral_position_changed_event(
                    "the_asset",
                    true,
                    liquidator_address.to_string()
                ),
                th_build_interests_updated_event("the_asset", &expected_rates)
            ]
        );

        // check user still has deposited collateral asset and
        // still has outstanding debt in debt asset
        let user = USERS.load(&deps.storage, &user_address).unwrap();
        assert!(get_bit(user.collateral_assets, asset_market_before.index).unwrap());
        assert!(get_bit(user.borrowed_assets, asset_market_before.index).unwrap());

        // check liquidator gets its collateral bit set
        let liquidator = USERS.load(&deps.storage, &user_address).unwrap();
        assert!(get_bit(liquidator.collateral_assets, asset_market_before.index).unwrap());

        // check user's debt decreased by the appropriate amount
        let debt = DEBTS.load(&deps.storage, ("the_asset", &user_address)).unwrap();

        let expected_less_debt_scaled = expected_rates.less_debt_scaled;

        let expected_user_debt_scaled = initial_user_debt_scaled - expected_less_debt_scaled;

        assert_eq!(expected_user_debt_scaled, debt.amount_scaled);

        // check global debt decreased by the appropriate amount
        let expected_global_debt_scaled = initial_global_debt_scaled - expected_less_debt_scaled;

        assert_eq!(expected_global_debt_scaled, asset_market_after.debt_total_scaled);
    }

    // Reset state for next test
    {
        let debt = Debt {
            amount_scaled: initial_user_debt_scaled,
            uncollateralized: false,
        };
        DEBTS.save(deps.as_mut().storage, ("the_asset", &user_address), &debt).unwrap();

        MARKETS.save(deps.as_mut().storage, "the_asset", &asset_market_initial).unwrap();

        // NOTE: Do not reset liquidator in order to check that position is not reset in next
        // liquidation receiving ma tokens
    }

    // Perform partial liquidation
    {
        let debt_to_repay = Uint128::from(400_000_u64);
        let liquidate_msg = ExecuteMsg::Liquidate {
            user: user_address.to_string(),
            collateral_denom: "the_asset".to_string(),
        };

        let asset_market_before = MARKETS.load(&deps.storage, "the_asset").unwrap();

        let block_time = liquidation_block_time;
        let env = mock_env_at_block_time(block_time);
        let info = cosmwasm_std::testing::mock_info(
            liquidator_address.as_str(),
            &[coin(debt_to_repay.into(), "the_asset")],
        );
        let res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap();

        let asset_market_after = MARKETS.load(&deps.storage, "the_asset").unwrap();
        let expected_liquidated_amount = math::divide_uint128_by_decimal(
            debt_to_repay * asset_price * (Decimal::one() + asset_liquidation_bonus),
            asset_price,
        )
        .unwrap();

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

        let expected_liquidated_amount_scaled = compute_scaled_amount(
            expected_liquidated_amount,
            expected_rates.liquidity_index,
            ScalingOperation::Truncate,
        )
        .unwrap();

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: ma_token_address.to_string(),
                    msg: to_binary(
                        &mars_outpost::ma_token::msg::ExecuteMsg::TransferOnLiquidation {
                            sender: user_address.to_string(),
                            recipient: liquidator_address.to_string(),
                            amount: expected_liquidated_amount_scaled,
                        }
                    )
                    .unwrap(),
                    funds: vec![]
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: ma_token_address.to_string(),
                    msg: to_binary(&ma_token::msg::ExecuteMsg::Mint {
                        recipient: "protocol_rewards_collector".to_string(),
                        amount: compute_scaled_amount(
                            expected_rates.protocol_rewards_to_distribute,
                            expected_rates.liquidity_index,
                            ScalingOperation::Truncate
                        )
                        .unwrap(),
                    })
                    .unwrap(),
                    funds: vec![]
                })),
            ]
        );

        mars_testing::assert_eq_vec(
            res.attributes,
            vec![
                attr("action", "outposts/red-bank/liquidate"),
                attr("collateral_denom", "the_asset"),
                attr("debt_denom", "the_asset"),
                attr("user", user_address.as_str()),
                attr("liquidator", liquidator_address.as_str()),
                attr("collateral_amount_liquidated", expected_liquidated_amount.to_string()),
                attr("debt_amount_repaid", debt_to_repay.to_string()),
                attr("refund_amount", "0"),
            ],
        );
        assert_eq!(
            res.events,
            vec![th_build_interests_updated_event("the_asset", &expected_rates)],
        );

        // check user still has deposited collateral asset and
        // still has outstanding debt in debt asset
        let user = USERS.load(&deps.storage, &user_address).unwrap();
        assert!(get_bit(user.collateral_assets, asset_market_before.index).unwrap());
        assert!(get_bit(user.borrowed_assets, asset_market_before.index).unwrap());

        // check user's debt decreased by the appropriate amount
        let debt = DEBTS.load(&deps.storage, ("the_asset", &user_address)).unwrap();

        let expected_less_debt_scaled = expected_rates.less_debt_scaled;

        let expected_user_debt_scaled = initial_user_debt_scaled - expected_less_debt_scaled;

        assert_eq!(expected_user_debt_scaled, debt.amount_scaled);

        // check global debt decreased by the appropriate amount
        let expected_global_debt_scaled = initial_global_debt_scaled - expected_less_debt_scaled;

        assert_eq!(expected_global_debt_scaled, asset_market_after.debt_total_scaled);
    }

    // Reset state for next test
    {
        let debt = Debt {
            amount_scaled: initial_user_debt_scaled,
            uncollateralized: false,
        };
        DEBTS.save(deps.as_mut().storage, ("the_asset", &user_address), &debt).unwrap();

        MARKETS.save(deps.as_mut().storage, "the_asset", &asset_market_initial).unwrap();

        // NOTE: Do not reset liquidator having the asset as collateral in order to check
        // position changed event is not emitted
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
            user: user_address.to_string(),
            collateral_denom: "the_asset".to_string(),
        };

        let asset_market_before = MARKETS.load(&deps.storage, "the_asset").unwrap();

        let env = mock_env_at_block_time(block_time);
        let info = cosmwasm_std::testing::mock_info(
            liquidator_address.as_str(),
            &[coin(debt_to_repay.into(), "the_asset")],
        );
        let res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap();

        let asset_market_after = MARKETS.load(&deps.storage, "the_asset").unwrap();
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

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: ma_token_address.to_string(),
                    msg: to_binary(
                        &mars_outpost::ma_token::msg::ExecuteMsg::TransferOnLiquidation {
                            sender: user_address.to_string(),
                            recipient: liquidator_address.to_string(),
                            amount: expected_liquidated_amount_scaled,
                        }
                    )
                    .unwrap(),
                    funds: vec![]
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: ma_token_address.to_string(),
                    msg: to_binary(&ma_token::msg::ExecuteMsg::Mint {
                        recipient: "protocol_rewards_collector".to_string(),
                        amount: compute_scaled_amount(
                            expected_rates.protocol_rewards_to_distribute,
                            expected_rates.liquidity_index,
                            ScalingOperation::Truncate
                        )
                        .unwrap(),
                    })
                    .unwrap(),
                    funds: vec![]
                })),
                // NOTE: Tax set to 0 so no tax should be charged
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: liquidator_address.to_string(),
                    amount: coins(expected_refund_amount.u128(), "the_asset")
                })),
            ]
        );

        mars_testing::assert_eq_vec(
            res.attributes,
            vec![
                attr("action", "outposts/red-bank/liquidate"),
                attr("collateral_denom", "the_asset"),
                attr("debt_denom", "the_asset"),
                attr("user", user_address.as_str()),
                attr("liquidator", liquidator_address.as_str()),
                attr("collateral_amount_liquidated", expected_liquidated_amount.to_string()),
                attr("debt_amount_repaid", expected_less_debt.to_string()),
                attr("refund_amount", expected_refund_amount),
            ],
        );
        assert_eq!(
            res.events,
            vec![
                th_build_interests_updated_event("the_asset", &expected_rates),
                // NOTE: Should not emit position change event as it was changed on the
                // first call and was not reset
            ]
        );

        // check user still has deposited collateral asset and
        // still has outstanding debt in debt asset
        let user = USERS.load(&deps.storage, &user_address).unwrap();
        assert!(get_bit(user.collateral_assets, asset_market_before.index).unwrap());
        assert!(get_bit(user.borrowed_assets, asset_market_before.index).unwrap());

        // check user's debt decreased by the appropriate amount
        let debt = DEBTS.load(&deps.storage, ("the_asset", &user_address)).unwrap();

        let expected_less_debt_scaled = expected_rates.less_debt_scaled;

        let expected_user_debt_scaled = initial_user_debt_scaled - expected_less_debt_scaled;

        assert_eq!(expected_user_debt_scaled, debt.amount_scaled);

        // check global debt decreased by the appropriate amount
        let expected_global_debt_scaled = initial_global_debt_scaled - expected_less_debt_scaled;

        assert_eq!(expected_global_debt_scaled, asset_market_after.debt_total_scaled);
    }

    // Reset state for next test
    {
        let debt = Debt {
            amount_scaled: initial_user_debt_scaled,
            uncollateralized: false,
        };
        DEBTS.save(deps.as_mut().storage, ("the_asset", &user_address), &debt).unwrap();

        MARKETS.save(deps.as_mut().storage, "the_asset", &asset_market_initial).unwrap();

        // NOTE: reset liquidator to not having the asset as collateral in order to check
        // position is not changed when receiving underlying asset
        let liquidator = User::default();
        USERS.save(deps.as_mut().storage, &liquidator_address, &liquidator).unwrap();
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
            user: user_address.to_string(),
            collateral_denom: "the_asset".to_string(),
        };

        let asset_market_before = MARKETS.load(&deps.storage, "the_asset").unwrap();

        let env = mock_env_at_block_time(block_time);
        let info = cosmwasm_std::testing::mock_info(
            liquidator_address.as_str(),
            &coins(debt_to_repay.u128(), "the_asset"),
        );
        let res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap();

        let asset_market_after = MARKETS.load(&deps.storage, "the_asset").unwrap();
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

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: ma_token_address.to_string(),
                    msg: to_binary(
                        &mars_outpost::ma_token::msg::ExecuteMsg::TransferOnLiquidation {
                            sender: user_address.to_string(),
                            recipient: liquidator_address.to_string(),
                            amount: expected_liquidated_amount_scaled,
                        }
                    )
                    .unwrap(),
                    funds: vec![]
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: ma_token_address.to_string(),
                    msg: to_binary(&ma_token::msg::ExecuteMsg::Mint {
                        recipient: "protocol_rewards_collector".to_string(),
                        amount: compute_scaled_amount(
                            expected_rates.protocol_rewards_to_distribute,
                            expected_rates.liquidity_index,
                            ScalingOperation::Truncate
                        )
                        .unwrap(),
                    })
                    .unwrap(),
                    funds: vec![]
                })),
                // NOTE: Tax set to 0 so no tax should be charged
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: liquidator_address.to_string(),
                    amount: coins(expected_refund_amount.u128(), "the_asset")
                })),
            ]
        );

        mars_testing::assert_eq_vec(
            res.attributes,
            vec![
                attr("action", "outposts/red-bank/liquidate"),
                attr("collateral_denom", "the_asset"),
                attr("debt_denom", "the_asset"),
                attr("user", user_address.as_str()),
                attr("liquidator", liquidator_address.as_str()),
                attr("collateral_amount_liquidated", expected_liquidated_amount.to_string()),
                attr("debt_amount_repaid", expected_less_debt.to_string()),
                attr("refund_amount", expected_refund_amount),
            ],
        );
        assert_eq!(
            res.events,
            vec![
                build_collateral_position_changed_event(
                    "the_asset",
                    true,
                    liquidator_address.to_string()
                ),
                th_build_interests_updated_event("the_asset", &expected_rates),
            ],
        );

        // check user still has deposited collateral asset and
        // still has outstanding debt in debt asset
        let user = USERS.load(&deps.storage, &user_address).unwrap();
        assert!(get_bit(user.collateral_assets, asset_market_before.index).unwrap());
        assert!(get_bit(user.borrowed_assets, asset_market_before.index).unwrap());

        // liquidator receives maTokens, should have collateral bit set
        let liquidator = USERS.load(&deps.storage, &liquidator_address).unwrap();
        assert!(get_bit(liquidator.collateral_assets, asset_market_before.index).unwrap());

        // check user's debt decreased by the appropriate amount
        let debt = DEBTS.load(&deps.storage, ("the_asset", &user_address)).unwrap();

        let expected_less_debt_scaled = expected_rates.less_debt_scaled;

        let expected_user_debt_scaled = initial_user_debt_scaled - expected_less_debt_scaled;

        assert_eq!(expected_user_debt_scaled, debt.amount_scaled);

        // check global debt decreased by the appropriate amount
        let expected_global_debt_scaled = initial_global_debt_scaled - expected_less_debt_scaled;

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

    let collateral_ltv = Decimal::from_ratio(5u128, 10u128);
    let collateral_liquidation_threshold = Decimal::from_ratio(7u128, 10u128);
    let collateral_liquidation_bonus = Decimal::from_ratio(1u128, 10u128);

    let collateral_market = Market {
        ma_token_address: Addr::unchecked("collateral"),
        max_loan_to_value: collateral_ltv,
        liquidation_threshold: collateral_liquidation_threshold,
        liquidation_bonus: collateral_liquidation_bonus,
        debt_total_scaled: Uint128::zero(),
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::one(),
        ..Default::default()
    };
    let debt_market = Market {
        ma_token_address: Addr::unchecked("debt"),
        max_loan_to_value: Decimal::from_ratio(6u128, 10u128),
        debt_total_scaled: Uint128::new(20_000_000) * SCALING_FACTOR,
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::one(),
        ..Default::default()
    };

    // initialize markets
    let collateral_market_initial = th_init_market(deps.as_mut(), "collateral", &collateral_market);

    let debt_market_initial = th_init_market(deps.as_mut(), "debt", &debt_market);

    // test health factor check
    let healthy_user_address = Addr::unchecked("healthy_user");

    // Set user as having collateral and debt in respective markets
    let mut healthy_user = User::default();

    set_bit(&mut healthy_user.collateral_assets, collateral_market_initial.index).unwrap();
    set_bit(&mut healthy_user.borrowed_assets, debt_market_initial.index).unwrap();

    USERS.save(deps.as_mut().storage, &healthy_user_address, &healthy_user).unwrap();

    // set initial collateral and debt balances for user
    let collateral_address = Addr::unchecked("collateral");
    let healthy_user_collateral_balance_scaled = Uint128::new(10_000_000) * SCALING_FACTOR;

    // Set the querier to return a certain collateral balance
    deps.querier.set_cw20_balances(
        collateral_address,
        &[(healthy_user_address.clone(), healthy_user_collateral_balance_scaled)],
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
    DEBTS.save(deps.as_mut().storage, ("debt", &healthy_user_address), &healthy_user_debt).unwrap();
    DEBTS
        .save(
            deps.as_mut().storage,
            ("uncollateralized_debt", &healthy_user_address),
            &uncollateralized_debt,
        )
        .unwrap();

    // perform liquidation (should fail because health factor is > 1)
    let liquidator_address = Addr::unchecked("liquidator");
    let debt_to_cover = Uint128::from(1_000_000u64);

    let liquidate_msg = ExecuteMsg::Liquidate {
        user: healthy_user_address.to_string(),
        collateral_denom: "collateral".to_string(),
    };

    let env = mock_env(MockEnvParams::default());
    let info = mock_info(liquidator_address.as_str(), &coins(debt_to_cover.u128(), "debt"));
    let error_res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap_err();
    assert_eq!(error_res, ContractError::CannotLiquidateHealthyPosition {});
}

#[test]
fn test_liquidate_if_collateral_disabled() {
    // initialize collateral and debt markets
    let mut deps = th_setup(&[]);

    let collateral_market_1 = Market {
        ma_token_address: Addr::unchecked("collateral1"),
        ..Default::default()
    };
    let collateral_market_2 = Market {
        ma_token_address: Addr::unchecked("collateral2"),
        ..Default::default()
    };
    let debt_market = Market {
        ma_token_address: Addr::unchecked("debt"),
        ..Default::default()
    };

    // initialize markets
    let collateral_market_initial_1 =
        th_init_market(deps.as_mut(), "collateral1", &collateral_market_1);
    let _collateral_market_initial_2 =
        th_init_market(deps.as_mut(), "collateral2", &collateral_market_2);

    let debt_market_initial = th_init_market(deps.as_mut(), "debt", &debt_market);

    // Set user as having collateral and debt in respective markets
    let user_address = Addr::unchecked("user");
    let mut user = User::default();
    set_bit(&mut user.collateral_assets, collateral_market_initial_1.index).unwrap();
    set_bit(&mut user.borrowed_assets, debt_market_initial.index).unwrap();

    USERS.save(deps.as_mut().storage, &user_address, &user).unwrap();

    // perform liquidation (should fail because collateral2 isn't set as collateral for user)
    let liquidator_address = Addr::unchecked("liquidator");
    let debt_to_cover = Uint128::from(1_000_000u64);

    let liquidate_msg = ExecuteMsg::Liquidate {
        user: user_address.to_string(),
        collateral_denom: "collateral2".to_string(),
    };

    let env = mock_env(MockEnvParams::default());
    let info = mock_info(liquidator_address.as_str(), &coins(debt_to_cover.u128(), "debt"));
    let error_res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap_err();
    assert_eq!(
        error_res,
        ContractError::CannotLiquidateWhenCollateralUnset {
            denom: "collateral2".to_string()
        }
    );
}

#[test]
fn test_finalize_liquidity_token_transfer() {
    // Setup
    let mut deps = th_setup(&[]);
    let env = mock_env(MockEnvParams::default());
    let info_matoken = mock_info("masomecoin", &[]);

    let mock_market = Market {
        ma_token_address: Addr::unchecked("masomecoin"),
        liquidity_index: Decimal::one(),
        liquidation_threshold: Decimal::from_ratio(5u128, 10u128),
        ..Default::default()
    };
    let market = th_init_market(deps.as_mut(), "somecoin", &mock_market);
    let debt_mock_market = Market {
        borrow_index: Decimal::one(),
        ..Default::default()
    };
    let debt_market = th_init_market(deps.as_mut(), "debtcoin", &debt_mock_market);

    deps.querier.set_oracle_price("somecoin", Decimal::from_ratio(1u128, 2u128));
    deps.querier.set_oracle_price("debtcoin", Decimal::from_ratio(2u128, 1u128));

    let sender_address = Addr::unchecked("fromaddr");
    let recipient_address = Addr::unchecked("toaddr");

    deps.querier.set_cw20_balances(
        Addr::unchecked("masomecoin"),
        &[(sender_address.clone(), Uint128::new(500_000) * SCALING_FACTOR)],
    );

    {
        let mut sender_user = User::default();
        set_bit(&mut sender_user.collateral_assets, market.index).unwrap();
        USERS.save(deps.as_mut().storage, &sender_address, &sender_user).unwrap();
    }

    // Finalize transfer with sender not borrowing passes
    {
        let msg = ExecuteMsg::FinalizeLiquidityTokenTransfer {
            sender_address: sender_address.clone(),
            recipient_address: recipient_address.clone(),
            sender_previous_balance: Uint128::new(1_000_000),
            recipient_previous_balance: Uint128::new(0),
            amount: Uint128::new(500_000),
        };

        let res = execute(deps.as_mut(), env.clone(), info_matoken.clone(), msg).unwrap();

        let sender_user = USERS.load(&deps.storage, &sender_address).unwrap();
        let recipient_user = USERS.load(&deps.storage, &recipient_address).unwrap();
        assert!(get_bit(sender_user.collateral_assets, market.index).unwrap());
        // Should create user and set deposited to true as previous balance is 0
        assert!(get_bit(recipient_user.collateral_assets, market.index).unwrap());

        assert_eq!(
            res.events,
            vec![build_collateral_position_changed_event(
                "somecoin",
                true,
                recipient_address.to_string()
            )]
        );
    }

    // Finalize transfer with health factor < 1 for sender doesn't go through
    {
        // set debt for user in order for health factor to be < 1
        let debt = Debt {
            amount_scaled: Uint128::new(500_000) * SCALING_FACTOR,
            uncollateralized: false,
        };
        let uncollateralized_debt = Debt {
            amount_scaled: Uint128::new(10_000) * SCALING_FACTOR,
            uncollateralized: true,
        };
        DEBTS.save(deps.as_mut().storage, ("debtcoin", &sender_address), &debt).unwrap();
        DEBTS
            .save(
                deps.as_mut().storage,
                ("uncollateralized_debt", &sender_address),
                &uncollateralized_debt,
            )
            .unwrap();
        let mut sender_user = USERS.load(&deps.storage, &sender_address).unwrap();
        set_bit(&mut sender_user.borrowed_assets, debt_market.index).unwrap();
        USERS.save(deps.as_mut().storage, &sender_address, &sender_user).unwrap();
    }

    {
        let msg = ExecuteMsg::FinalizeLiquidityTokenTransfer {
            sender_address: sender_address.clone(),
            recipient_address: recipient_address.clone(),
            sender_previous_balance: Uint128::new(1_000_000),
            recipient_previous_balance: Uint128::new(0),
            amount: Uint128::new(500_000),
        };

        let error_res = execute(deps.as_mut(), env.clone(), info_matoken.clone(), msg).unwrap_err();
        assert_eq!(error_res, ContractError::CannotTransferTokenWhenInvalidHealthFactor {});
    }

    // Finalize transfer with health factor > 1 for goes through
    {
        // set debt for user in order for health factor to be > 1
        let debt = Debt {
            amount_scaled: Uint128::new(1_000) * SCALING_FACTOR,
            uncollateralized: false,
        };
        let uncollateralized_debt = Debt {
            amount_scaled: Uint128::new(10_000u128) * SCALING_FACTOR,
            uncollateralized: true,
        };
        DEBTS.save(deps.as_mut().storage, ("debtcoin", &sender_address), &debt).unwrap();
        DEBTS
            .save(
                deps.as_mut().storage,
                ("uncollateralized_debt", &sender_address),
                &uncollateralized_debt,
            )
            .unwrap();
        let mut sender_user = USERS.load(&deps.storage, &sender_address).unwrap();
        set_bit(&mut sender_user.borrowed_assets, debt_market.index).unwrap();
        USERS.save(deps.as_mut().storage, &sender_address, &sender_user).unwrap();
    }

    {
        let msg = ExecuteMsg::FinalizeLiquidityTokenTransfer {
            sender_address: sender_address.clone(),
            recipient_address: recipient_address.clone(),
            sender_previous_balance: Uint128::new(500_000),
            recipient_previous_balance: Uint128::new(500_000),
            amount: Uint128::new(500_000),
        };

        let res = execute(deps.as_mut(), env.clone(), info_matoken, msg).unwrap();

        let sender_user = USERS.load(&deps.storage, &sender_address).unwrap();
        let recipient_user = USERS.load(&deps.storage, &recipient_address).unwrap();
        // Should set deposited to false as: previous_balance - amount = 0
        assert!(!get_bit(sender_user.collateral_assets, market.index).unwrap());
        assert!(get_bit(recipient_user.collateral_assets, market.index).unwrap());

        assert_eq!(
            res.events,
            vec![build_collateral_position_changed_event(
                "somecoin",
                false,
                sender_address.to_string()
            )]
        );
    }

    // Calling this with other token fails
    {
        let msg = ExecuteMsg::FinalizeLiquidityTokenTransfer {
            sender_address,
            recipient_address,
            sender_previous_balance: Uint128::new(500_000),
            recipient_previous_balance: Uint128::new(500_000),
            amount: Uint128::new(500_000),
        };
        let info = mock_info("othertoken", &[]);

        let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(error_res, StdError::not_found("alloc::string::String").into());
    }
}
