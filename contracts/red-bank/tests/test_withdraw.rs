use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{
    attr, coin, coins, to_binary, Addr, BankMsg, CosmosMsg, Decimal, SubMsg, Uint128, WasmMsg,
};

use mars_outpost::red_bank::{Debt, ExecuteMsg, Market, User};
use mars_outpost::{ma_token, math};
use mars_testing::{mock_env, mock_env_at_block_time, MockEnvParams};

use mars_red_bank::contract::execute;
use mars_red_bank::error::ContractError;
use mars_red_bank::events::build_collateral_position_changed_event;
use mars_red_bank::helpers::{get_bit, set_bit};
use mars_red_bank::interest_rates::{
    compute_scaled_amount, compute_underlying_amount, get_scaled_liquidity_amount,
    get_updated_borrow_index, get_updated_liquidity_index, ScalingOperation, SCALING_FACTOR,
};
use mars_red_bank::state::{DEBTS, MARKETS, MARKET_DENOMS_BY_MA_TOKEN, USERS};

use helpers::{
    th_build_interests_updated_event, th_get_expected_indices_and_rates, th_init_market, th_setup,
    TestUtilizationDeltaInfo,
};

mod helpers;

#[test]
fn test_withdraw_native() {
    // Withdraw native token
    let initial_available_liquidity = Uint128::from(12000000u128);
    let mut deps = th_setup(&[coin(initial_available_liquidity.into(), "somecoin")]);

    let initial_liquidity_index = Decimal::from_ratio(15u128, 10u128);
    let mock_market = Market {
        ma_token_address: Addr::unchecked("matoken"),
        liquidity_index: initial_liquidity_index,
        borrow_index: Decimal::from_ratio(2u128, 1u128),
        borrow_rate: Decimal::from_ratio(20u128, 100u128),
        liquidity_rate: Decimal::from_ratio(10u128, 100u128),
        reserve_factor: Decimal::from_ratio(1u128, 10u128),

        debt_total_scaled: Uint128::new(10_000_000) * SCALING_FACTOR,
        indexes_last_updated: 10000000,
        ..Default::default()
    };
    let withdraw_amount = Uint128::from(20000u128);
    let seconds_elapsed = 2000u64;

    let initial_deposit_amount_scaled = Uint128::new(2_000_000) * SCALING_FACTOR;
    deps.querier.set_cw20_balances(
        Addr::unchecked("matoken"),
        &[(Addr::unchecked("withdrawer"), initial_deposit_amount_scaled)],
    );

    let market_initial = th_init_market(deps.as_mut(), "somecoin", &mock_market);
    MARKET_DENOMS_BY_MA_TOKEN
        .save(deps.as_mut().storage, &Addr::unchecked("matoken"), &"somecoin".to_string())
        .unwrap();

    let withdrawer_addr = Addr::unchecked("withdrawer");
    let user = User::default();
    USERS.save(deps.as_mut().storage, &withdrawer_addr, &user).unwrap();

    let msg = ExecuteMsg::Withdraw {
        denom: "somecoin".to_string(),
        amount: Some(withdraw_amount),
        recipient: None,
    };

    let env = mock_env_at_block_time(mock_market.indexes_last_updated + seconds_elapsed);
    let info = mock_info("withdrawer", &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    let market = MARKETS.load(&deps.storage, "somecoin").unwrap();

    let expected_params = th_get_expected_indices_and_rates(
        &market_initial,
        mock_market.indexes_last_updated + seconds_elapsed,
        initial_available_liquidity,
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

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "matoken".to_string(),
                msg: to_binary(&ma_token::msg::ExecuteMsg::Mint {
                    recipient: "protocol_rewards_collector".to_string(),
                    amount: compute_scaled_amount(
                        expected_params.protocol_rewards_to_distribute,
                        market.liquidity_index,
                        ScalingOperation::Truncate
                    )
                    .unwrap(),
                })
                .unwrap(),
                funds: vec![]
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "matoken".to_string(),
                msg: to_binary(&ma_token::msg::ExecuteMsg::Burn {
                    user: withdrawer_addr.to_string(),
                    amount: expected_burn_amount,
                })
                .unwrap(),
                funds: vec![]
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: withdrawer_addr.to_string(),
                amount: coins(withdraw_amount.u128(), "somecoin")
            })),
        ]
    );
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "outposts/red-bank/withdraw"),
            attr("denom", "somecoin"),
            attr("user", "withdrawer"),
            attr("recipient", "withdrawer"),
            attr("burn_amount", expected_burn_amount.to_string()),
            attr("withdraw_amount", withdraw_amount.to_string()),
        ]
    );
    assert_eq!(res.events, vec![th_build_interests_updated_event("somecoin", &expected_params)]);

    assert_eq!(market.borrow_rate, expected_params.borrow_rate);
    assert_eq!(market.liquidity_rate, expected_params.liquidity_rate);
    assert_eq!(market.liquidity_index, expected_params.liquidity_index);
    assert_eq!(market.borrow_index, expected_params.borrow_index);
}

#[test]
fn test_withdraw_and_send_funds_to_another_user() {
    // Withdraw cw20 token
    let mut deps = th_setup(&[]);
    let denom = "somecoin";
    let initial_available_liquidity = Uint128::from(12000000u128);

    let ma_token_addr = Addr::unchecked("matoken");

    let withdrawer_addr = Addr::unchecked("withdrawer");
    let another_user_addr = Addr::unchecked("another_user");

    deps.querier.set_contract_balances(&coins(initial_available_liquidity.u128(), denom));
    let ma_token_balance_scaled = Uint128::new(2_000_000) * SCALING_FACTOR;
    deps.querier.set_cw20_balances(
        ma_token_addr.clone(),
        &[(withdrawer_addr.clone(), ma_token_balance_scaled)],
    );

    let mock_market = Market {
        ma_token_address: Addr::unchecked("matoken"),
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::one(),
        reserve_factor: Decimal::zero(),
        ..Default::default()
    };

    let market_initial = th_init_market(deps.as_mut(), denom, &mock_market);
    MARKET_DENOMS_BY_MA_TOKEN
        .save(deps.as_mut().storage, &ma_token_addr, &denom.to_string())
        .unwrap();

    let user = User::default();
    USERS.save(deps.as_mut().storage, &withdrawer_addr, &user).unwrap();

    let msg = ExecuteMsg::Withdraw {
        denom: denom.to_string(),
        amount: None,
        recipient: Some(another_user_addr.to_string()),
    };

    let env = mock_env(MockEnvParams::default());
    let info = mock_info("withdrawer", &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    // User should have unset bit for collateral after full withdraw
    let user = USERS.load(&deps.storage, &withdrawer_addr).unwrap();
    assert!(!get_bit(user.collateral_assets, market_initial.index).unwrap());

    let withdraw_amount = compute_underlying_amount(
        ma_token_balance_scaled,
        market_initial.liquidity_index,
        ScalingOperation::Truncate,
    )
    .unwrap();

    // Check if maToken is received by `another_user`
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: ma_token_addr.to_string(),
                msg: to_binary(&ma_token::msg::ExecuteMsg::Burn {
                    user: withdrawer_addr.to_string(),
                    amount: ma_token_balance_scaled,
                })
                .unwrap(),
                funds: vec![]
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: another_user_addr.to_string(),
                amount: coins(withdraw_amount.u128(), denom)
            }))
        ]
    );
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "outposts/red-bank/withdraw"),
            attr("denom", denom.to_string()),
            attr("user", withdrawer_addr),
            attr("recipient", another_user_addr),
            attr("burn_amount", ma_token_balance_scaled.to_string()),
            attr("withdraw_amount", withdraw_amount.to_string()),
        ]
    );
}

#[test]
fn test_withdraw_cannot_exceed_balance() {
    let mut deps = th_setup(&[]);
    let env = mock_env(MockEnvParams::default());

    let mock_market = Market {
        ma_token_address: Addr::unchecked("matoken"),
        liquidity_index: Decimal::from_ratio(15u128, 10u128),
        ..Default::default()
    };

    deps.querier.set_cw20_balances(
        Addr::unchecked("matoken"),
        &[(Addr::unchecked("withdrawer"), Uint128::new(200u128))],
    );

    th_init_market(deps.as_mut(), "somecoin", &mock_market);

    let msg = ExecuteMsg::Withdraw {
        denom: "somecoin".to_string(),
        amount: Some(Uint128::from(2000u128)),
        recipient: None,
    };

    let info = mock_info("withdrawer", &[]);
    let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(
        error_res,
        ContractError::InvalidWithdrawAmount {
            denom: "somecoin".to_string()
        }
    );
}

#[test]
fn test_withdraw_if_health_factor_not_met() {
    let initial_available_liquidity = Uint128::from(10000000u128);
    let mut deps = th_setup(&[coin(initial_available_liquidity.into(), "token3")]);

    let withdrawer_addr = Addr::unchecked("withdrawer");

    // Initialize markets
    let ma_token_1_addr = Addr::unchecked("matoken1");
    let market_1 = Market {
        ma_token_address: ma_token_1_addr.clone(),
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::one(),
        max_loan_to_value: Decimal::from_ratio(40u128, 100u128),
        liquidation_threshold: Decimal::from_ratio(60u128, 100u128),
        ..Default::default()
    };
    let ma_token_2_addr = Addr::unchecked("matoken2");
    let market_2 = Market {
        ma_token_address: ma_token_2_addr,
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::one(),
        max_loan_to_value: Decimal::from_ratio(50u128, 100u128),
        liquidation_threshold: Decimal::from_ratio(80u128, 100u128),
        ..Default::default()
    };
    let ma_token_3_addr = Addr::unchecked("matoken3");
    let market_3 = Market {
        ma_token_address: ma_token_3_addr.clone(),
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::one(),
        max_loan_to_value: Decimal::from_ratio(20u128, 100u128),
        liquidation_threshold: Decimal::from_ratio(40u128, 100u128),
        ..Default::default()
    };
    let market_1_initial = th_init_market(deps.as_mut(), "token1", &market_1);
    let market_2_initial = th_init_market(deps.as_mut(), "token2", &market_2);
    let market_3_initial = th_init_market(deps.as_mut(), "token3", &market_3);

    // Initialize user with market_1 and market_3 as collaterals
    // User borrows market_2
    let mut user = User::default();
    set_bit(&mut user.collateral_assets, market_1_initial.index).unwrap();
    set_bit(&mut user.collateral_assets, market_3_initial.index).unwrap();
    set_bit(&mut user.borrowed_assets, market_2_initial.index).unwrap();
    USERS.save(deps.as_mut().storage, &withdrawer_addr, &user).unwrap();

    // Set the querier to return collateral balances (ma_token_1 and ma_token_3)
    let ma_token_1_balance_scaled = Uint128::new(100_000) * SCALING_FACTOR;
    deps.querier.set_cw20_balances(
        ma_token_1_addr,
        &[(withdrawer_addr.clone(), ma_token_1_balance_scaled)],
    );
    let ma_token_3_balance_scaled = Uint128::new(600_000) * SCALING_FACTOR;
    deps.querier.set_cw20_balances(
        ma_token_3_addr,
        &[(withdrawer_addr.clone(), ma_token_3_balance_scaled)],
    );

    // Set user to have positive debt amount in debt asset
    // Uncollateralized debt shouldn't count for health factor
    let token_2_debt_scaled = Uint128::new(200_000) * SCALING_FACTOR;
    let debt = Debt {
        amount_scaled: token_2_debt_scaled,
        uncollateralized: false,
    };
    let uncollateralized_debt = Debt {
        amount_scaled: Uint128::new(200_000) * SCALING_FACTOR,
        uncollateralized: true,
    };
    DEBTS.save(deps.as_mut().storage, ("token2", &withdrawer_addr), &debt).unwrap();
    DEBTS
        .save(deps.as_mut().storage, ("token3", &withdrawer_addr), &uncollateralized_debt)
        .unwrap();

    // Set the querier to return native exchange rates
    let token_1_exchange_rate = Decimal::from_ratio(3u128, 1u128);
    let token_2_exchange_rate = Decimal::from_ratio(2u128, 1u128);
    let token_3_exchange_rate = Decimal::from_ratio(1u128, 1u128);

    deps.querier.set_oracle_price("token1", token_1_exchange_rate);
    deps.querier.set_oracle_price("token2", token_2_exchange_rate);
    deps.querier.set_oracle_price("token3", token_3_exchange_rate);

    let env = mock_env(MockEnvParams::default());
    let info = mock_info("withdrawer", &[]);

    // Calculate how much to withdraw to have health factor equal to one
    let how_much_to_withdraw = {
        let token_1_weighted_lt_in_base_asset = compute_underlying_amount(
            ma_token_1_balance_scaled,
            get_updated_liquidity_index(&market_1_initial, env.block.time.seconds()).unwrap(),
            ScalingOperation::Truncate,
        )
        .unwrap()
            * market_1_initial.liquidation_threshold
            * token_1_exchange_rate;
        let token_3_weighted_lt_in_base_asset = compute_underlying_amount(
            ma_token_3_balance_scaled,
            get_updated_liquidity_index(&market_3_initial, env.block.time.seconds()).unwrap(),
            ScalingOperation::Truncate,
        )
        .unwrap()
            * market_3_initial.liquidation_threshold
            * token_3_exchange_rate;
        let weighted_liquidation_threshold_in_base_asset =
            token_1_weighted_lt_in_base_asset + token_3_weighted_lt_in_base_asset;

        let total_collateralized_debt_in_base_asset = compute_underlying_amount(
            token_2_debt_scaled,
            get_updated_borrow_index(&market_2_initial, env.block.time.seconds()).unwrap(),
            ScalingOperation::Ceil,
        )
        .unwrap()
            * token_2_exchange_rate;

        // How much to withdraw in base asset to have health factor equal to one
        let how_much_to_withdraw_in_base_asset = math::divide_uint128_by_decimal(
            weighted_liquidation_threshold_in_base_asset - total_collateralized_debt_in_base_asset,
            market_3_initial.liquidation_threshold,
        )
        .unwrap();
        math::divide_uint128_by_decimal(how_much_to_withdraw_in_base_asset, token_3_exchange_rate)
            .unwrap()
    };

    // Withdraw token3 with failure
    // The withdraw amount needs to be a little bit greater to have health factor less than one
    {
        let withdraw_amount = how_much_to_withdraw + Uint128::from(10u128);
        let msg = ExecuteMsg::Withdraw {
            denom: "token3".to_string(),
            amount: Some(withdraw_amount),
            recipient: None,
        };
        let error_res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
        assert_eq!(error_res, ContractError::InvalidHealthFactorAfterWithdraw {});
    }

    // Withdraw token3 with success
    // The withdraw amount needs to be a little bit smaller to have health factor greater than one
    {
        let withdraw_amount = how_much_to_withdraw - Uint128::from(10u128);
        let msg = ExecuteMsg::Withdraw {
            denom: "token3".to_string(),
            amount: Some(withdraw_amount),
            recipient: None,
        };
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let withdraw_amount_scaled = get_scaled_liquidity_amount(
            withdraw_amount,
            &market_3_initial,
            env.block.time.seconds(),
        )
        .unwrap();

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "matoken3".to_string(),
                    msg: to_binary(&ma_token::msg::ExecuteMsg::Burn {
                        user: withdrawer_addr.to_string(),
                        amount: withdraw_amount_scaled,
                    })
                    .unwrap(),
                    funds: vec![]
                })),
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: withdrawer_addr.to_string(),
                    amount: coins(withdraw_amount.u128(), "token3")
                })),
            ]
        );
    }
}

#[test]
fn test_withdraw_total_balance() {
    // Withdraw native token
    let initial_available_liquidity = Uint128::from(12000000u128);
    let mut deps = th_setup(&[coin(initial_available_liquidity.into(), "somecoin")]);

    let initial_liquidity_index = Decimal::from_ratio(15u128, 10u128);
    let mock_market = Market {
        ma_token_address: Addr::unchecked("matoken"),
        liquidity_index: initial_liquidity_index,
        borrow_index: Decimal::from_ratio(2u128, 1u128),
        borrow_rate: Decimal::from_ratio(20u128, 100u128),
        liquidity_rate: Decimal::from_ratio(10u128, 100u128),
        reserve_factor: Decimal::from_ratio(1u128, 10u128),
        debt_total_scaled: Uint128::new(10_000_000) * SCALING_FACTOR,
        indexes_last_updated: 10000000,
        ..Default::default()
    };
    let withdrawer_balance_scaled = Uint128::new(123_456) * SCALING_FACTOR;
    let seconds_elapsed = 2000u64;

    deps.querier.set_cw20_balances(
        Addr::unchecked("matoken"),
        &[(Addr::unchecked("withdrawer"), withdrawer_balance_scaled)],
    );

    let market_initial = th_init_market(deps.as_mut(), "somecoin", &mock_market);
    MARKET_DENOMS_BY_MA_TOKEN
        .save(deps.as_mut().storage, &Addr::unchecked("matoken"), &"somecoin".to_string())
        .unwrap();

    // Mark the market as collateral for the user
    let withdrawer_addr = Addr::unchecked("withdrawer");
    let mut user = User::default();
    set_bit(&mut user.collateral_assets, market_initial.index).unwrap();
    USERS.save(deps.as_mut().storage, &withdrawer_addr, &user).unwrap();
    // Check if user has set bit for collateral
    assert!(get_bit(user.collateral_assets, market_initial.index).unwrap());

    let msg = ExecuteMsg::Withdraw {
        denom: "somecoin".to_string(),
        amount: None,
        recipient: None,
    };

    let env = mock_env_at_block_time(mock_market.indexes_last_updated + seconds_elapsed);
    let info = mock_info("withdrawer", &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    let market = MARKETS.load(&deps.storage, "somecoin").unwrap();

    let withdrawer_balance = compute_underlying_amount(
        withdrawer_balance_scaled,
        get_updated_liquidity_index(
            &market_initial,
            market_initial.indexes_last_updated + seconds_elapsed,
        )
        .unwrap(),
        ScalingOperation::Truncate,
    )
    .unwrap();

    let expected_params = th_get_expected_indices_and_rates(
        &market_initial,
        mock_market.indexes_last_updated + seconds_elapsed,
        initial_available_liquidity,
        TestUtilizationDeltaInfo {
            less_liquidity: withdrawer_balance,
            ..Default::default()
        },
    );

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "matoken".to_string(),
                msg: to_binary(&ma_token::msg::ExecuteMsg::Mint {
                    recipient: "protocol_rewards_collector".to_string(),
                    amount: compute_scaled_amount(
                        expected_params.protocol_rewards_to_distribute,
                        expected_params.liquidity_index,
                        ScalingOperation::Truncate
                    )
                    .unwrap(),
                })
                .unwrap(),
                funds: vec![]
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "matoken".to_string(),
                msg: to_binary(&ma_token::msg::ExecuteMsg::Burn {
                    user: withdrawer_addr.to_string(),
                    amount: withdrawer_balance_scaled,
                })
                .unwrap(),
                funds: vec![]
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: withdrawer_addr.to_string(),
                amount: coins(withdrawer_balance.u128(), "somecoin")
            })),
        ]
    );
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "outposts/red-bank/withdraw"),
            attr("denom", "somecoin"),
            attr("user", "withdrawer"),
            attr("recipient", "withdrawer"),
            attr("burn_amount", withdrawer_balance_scaled.to_string()),
            attr("withdraw_amount", withdrawer_balance.to_string()),
        ]
    );
    assert_eq!(
        res.events,
        vec![
            build_collateral_position_changed_event("somecoin", false, "withdrawer".to_string()),
            th_build_interests_updated_event("somecoin", &expected_params)
        ]
    );

    assert_eq!(market.borrow_rate, expected_params.borrow_rate);
    assert_eq!(market.liquidity_rate, expected_params.liquidity_rate);
    assert_eq!(market.liquidity_index, expected_params.liquidity_index);
    assert_eq!(market.borrow_index, expected_params.borrow_index);

    // User should have unset bit for collateral after full withdraw
    let user = USERS.load(&deps.storage, &withdrawer_addr).unwrap();
    assert!(!get_bit(user.collateral_assets, market_initial.index).unwrap());
}

#[test]
fn test_withdraw_without_existing_position() {
    // Withdraw native token
    let initial_available_liquidity = Uint128::from(12000000u128);
    let mut deps = th_setup(&[coin(initial_available_liquidity.into(), "somecoin")]);

    deps.querier.set_cw20_balances(
        Addr::unchecked("matoken"),
        &[
            (Addr::unchecked("withdrawer"), Uint128::new(2_000_000) * SCALING_FACTOR),
            (
                Addr::unchecked("protocol_rewards_collector"),
                Uint128::new(2_000_000) * SCALING_FACTOR,
            ),
        ],
    );

    let market = Market {
        ma_token_address: Addr::unchecked("matoken"),
        ..Default::default()
    };
    th_init_market(deps.as_mut(), "somecoin", &market);

    let msg = ExecuteMsg::Withdraw {
        denom: "somecoin".to_string(),
        amount: None,
        recipient: None,
    };

    // normal address cannot withdraw without an existing position
    {
        let info = mock_info("withdrawer", &[]);
        let env = mock_env(MockEnvParams::default());
        let error = execute(deps.as_mut(), env, info, msg.clone()).unwrap_err();
        assert_eq!(error, ContractError::ExistingUserPositionRequired {});
    }

    // protocol_rewards_collector can withdraw without an existing position
    {
        let info = mock_info("protocol_rewards_collector", &[]);
        let env = mock_env(MockEnvParams::default());
        execute(deps.as_mut(), env, info, msg).unwrap();
    }
}
