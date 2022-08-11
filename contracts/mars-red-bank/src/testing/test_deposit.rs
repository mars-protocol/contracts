use std::any::type_name;

use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{
    attr, coin, to_binary, Addr, CosmosMsg, Decimal, StdError, SubMsg, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;

use mars_outpost::red_bank::{ExecuteMsg, Market};
use mars_testing::{mock_env, mock_env_at_block_time, MockEnvParams};

use crate::contract::execute;
use crate::error::ContractError;
use crate::events::build_collateral_position_changed_event;
use crate::helpers::get_bit;
use crate::interest_rates::{compute_scaled_amount, ScalingOperation, SCALING_FACTOR};
use crate::state::{MARKETS, USERS};

use super::helpers::{
    th_build_interests_updated_event, th_get_expected_indices_and_rates, th_init_market, th_setup,
};

#[test]
fn test_deposit_native_asset() {
    let initial_liquidity = Uint128::from(10000000_u128);
    let mut deps = th_setup(&[coin(initial_liquidity.into(), "somecoin")]);
    let reserve_factor = Decimal::from_ratio(1u128, 10u128);

    let mock_market = Market {
        ma_token_address: Addr::unchecked("matoken"),
        liquidity_index: Decimal::from_ratio(11u128, 10u128),
        max_loan_to_value: Decimal::one(),
        borrow_index: Decimal::from_ratio(1u128, 1u128),
        borrow_rate: Decimal::from_ratio(10u128, 100u128),
        liquidity_rate: Decimal::from_ratio(10u128, 100u128),
        reserve_factor,
        debt_total_scaled: Uint128::new(10_000_000) * SCALING_FACTOR,
        indexes_last_updated: 10000000,
        ..Default::default()
    };
    let market = th_init_market(deps.as_mut(), "somecoin", &mock_market);

    let deposit_amount = 110000;
    let env = mock_env_at_block_time(10000100);
    let info = cosmwasm_std::testing::mock_info("depositor", &[coin(deposit_amount, "somecoin")]);
    let msg = ExecuteMsg::Deposit {
        denom: String::from("somecoin"),
        on_behalf_of: None,
    };
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let expected_params = th_get_expected_indices_and_rates(
        &market,
        env.block.time.seconds(),
        initial_liquidity,
        Default::default(),
    );

    let expected_mint_amount = compute_scaled_amount(
        Uint128::from(deposit_amount),
        expected_params.liquidity_index,
        ScalingOperation::Truncate,
    )
    .unwrap();

    // mints coin_amount/liquidity_index
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "matoken".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: "depositor".to_string(),
                amount: expected_mint_amount.into(),
            })
            .unwrap(),
            funds: vec![]
        }))]
    );
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "deposit"),
            attr("denom", "somecoin"),
            attr("sender", "depositor"),
            attr("user", "depositor"),
            attr("amount", deposit_amount.to_string()),
        ]
    );
    assert_eq!(
        res.events,
        vec![
            build_collateral_position_changed_event("somecoin", true, "depositor".to_string()),
            th_build_interests_updated_event("somecoin", &expected_params)
        ]
    );

    let market = MARKETS.load(&deps.storage, "somecoin").unwrap();
    assert_eq!(market.borrow_rate, expected_params.borrow_rate);
    assert_eq!(market.liquidity_rate, expected_params.liquidity_rate);
    assert_eq!(market.liquidity_index, expected_params.liquidity_index);
    assert_eq!(market.borrow_index, expected_params.borrow_index);

    // send many native coins
    let info = cosmwasm_std::testing::mock_info(
        "depositor",
        &[coin(100, "somecoin1"), coin(200, "somecoin2")],
    );
    let msg = ExecuteMsg::Deposit {
        denom: String::from("somecoin2"),
        on_behalf_of: None,
    };
    let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
    assert_eq!(
        error_res,
        ContractError::InvalidCoinsSent {
            denom: "somecoin2".to_string()
        }
    );

    // empty deposit fails
    let info = mock_info("depositor", &[]);
    let msg = ExecuteMsg::Deposit {
        denom: String::from("somecoin"),
        on_behalf_of: None,
    };
    let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(
        error_res,
        ContractError::InvalidCoinsSent {
            denom: "somecoin".to_string()
        }
    );
}

#[test]
fn test_cannot_deposit_if_no_market() {
    let mut deps = th_setup(&[]);
    let env = mock_env(MockEnvParams::default());

    let info = cosmwasm_std::testing::mock_info("depositer", &[coin(110000, "somecoin")]);
    let msg = ExecuteMsg::Deposit {
        denom: String::from("somecoin"),
        on_behalf_of: None,
    };
    let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(error_res, StdError::not_found(type_name::<Market>()).into());
}

#[test]
fn test_cannot_deposit_if_market_not_active() {
    let mut deps = th_setup(&[]);

    let mock_market = Market {
        ma_token_address: Addr::unchecked("ma_somecoin"),
        active: false,
        deposit_enabled: true,
        ..Default::default()
    };
    th_init_market(deps.as_mut(), "somecoin", &mock_market);

    // Check error when deposit not allowed on market
    let env = mock_env(MockEnvParams::default());
    let info = cosmwasm_std::testing::mock_info("depositor", &[coin(110000, "somecoin")]);
    let msg = ExecuteMsg::Deposit {
        denom: String::from("somecoin"),
        on_behalf_of: None,
    };
    let error_res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap_err();
    assert_eq!(
        error_res,
        ContractError::MarketNotActive {
            denom: "somecoin".to_string()
        }
    );
}

#[test]
fn test_cannot_deposit_if_market_not_enabled() {
    let mut deps = th_setup(&[]);

    let mock_market = Market {
        ma_token_address: Addr::unchecked("ma_somecoin"),
        active: true,
        deposit_enabled: false,
        ..Default::default()
    };
    th_init_market(deps.as_mut(), "somecoin", &mock_market);

    // Check error when deposit not allowed on market
    let env = mock_env(MockEnvParams::default());
    let info = cosmwasm_std::testing::mock_info("depositor", &[coin(110000, "somecoin")]);
    let msg = ExecuteMsg::Deposit {
        denom: String::from("somecoin"),
        on_behalf_of: None,
    };
    let error_res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap_err();
    assert_eq!(
        error_res,
        ContractError::DepositNotEnabled {
            denom: "somecoin".to_string()
        }
    );
}

#[test]
fn test_deposit_on_behalf_of() {
    let initial_liquidity = 10000000;
    let mut deps = th_setup(&[coin(initial_liquidity, "somecoin")]);

    let mock_market = Market {
        ma_token_address: Addr::unchecked("matoken"),
        liquidity_index: Decimal::one(),
        borrow_index: Decimal::one(),
        ..Default::default()
    };
    let market = th_init_market(deps.as_mut(), "somecoin", &mock_market);

    let depositor_addr = Addr::unchecked("depositor");
    let another_user_addr = Addr::unchecked("another_user");
    let deposit_amount = 110000;
    let env = mock_env(MockEnvParams::default());
    let info = cosmwasm_std::testing::mock_info(
        depositor_addr.as_str(),
        &[coin(deposit_amount, "somecoin")],
    );
    let msg = ExecuteMsg::Deposit {
        denom: String::from("somecoin"),
        on_behalf_of: Some(another_user_addr.to_string()),
    };
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let expected_mint_amount = compute_scaled_amount(
        Uint128::from(deposit_amount),
        market.liquidity_index,
        ScalingOperation::Truncate,
    )
    .unwrap();

    // 'depositor' should not be saved
    let _user = USERS.load(&deps.storage, &depositor_addr).unwrap_err();

    // 'another_user' should have collateral bit set
    let user = USERS.load(&deps.storage, &another_user_addr).unwrap();
    assert!(get_bit(user.collateral_assets, market.index).unwrap());

    // recipient should be `another_user`
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "matoken".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: another_user_addr.to_string(),
                amount: expected_mint_amount.into(),
            })
            .unwrap(),
            funds: vec![]
        }))]
    );
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "deposit"),
            attr("denom", "somecoin"),
            attr("sender", depositor_addr),
            attr("user", another_user_addr),
            attr("amount", deposit_amount.to_string()),
        ]
    );
}
