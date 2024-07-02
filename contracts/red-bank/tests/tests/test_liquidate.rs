use std::{collections::HashMap, str::FromStr};

use cosmwasm_std::{
    attr, coin,
    testing::{mock_dependencies, mock_env, mock_info},
    to_json_binary, Addr, Decimal, SubMsg, Uint128, WasmMsg,
};
use cw_utils::PaymentError;
use mars_red_bank::{contract::execute, error::ContractError};
use mars_testing::{
    integration::mock_env::{MockEnv, MockEnvBuilder},
    mock_env_at_block_time,
};
use mars_types::{
    address_provider::MarsAddressType,
    incentives,
    params::{AssetParams, CmSettings, LiquidationBonus, RedBankSettings},
    red_bank::{
        ExecuteMsg, InitOrUpdateAssetParams, InterestRateModel, Market, QueryMsg,
        UserCollateralResponse, UserDebtResponse,
    },
};

use super::helpers::{
    assert_err, liq_threshold_hf, merge_collaterals_and_debts, th_build_interests_updated_event,
    th_get_expected_indices_and_rates, th_get_scaled_liquidity_amount, th_query, th_setup,
    TestUtilizationDeltaInfo,
};

// NOTE: See spreadsheet with liquidation numbers for reference:
// contracts/red-bank/tests/files/Red Bank - Dynamic LB & CF test cases v1.1.xlsx

#[test]
fn cannot_self_liquidate() {
    let mut mock_env = MockEnvBuilder::new(None, Addr::unchecked("owner")).build();

    let red_bank = mock_env.red_bank.clone();

    let (_, _, _, liquidator) = setup_env(&mut mock_env);

    let error_res = red_bank.liquidate(
        &mut mock_env,
        &liquidator,
        &liquidator,
        "ujake",
        &[coin(1000, "uusdc")],
    );
    assert_err(error_res, ContractError::CannotLiquidateSelf {});
}

#[test]
fn cannot_liquidate_credit_manager() {
    let mut mock_env = MockEnvBuilder::new(None, Addr::unchecked("owner")).build();

    let red_bank = mock_env.red_bank.clone();
    let credit_manager = mock_env.credit_manager.clone();

    let (_, _, _, liquidator) = setup_env(&mut mock_env);

    let error_res = red_bank.liquidate(
        &mut mock_env,
        &liquidator,
        &credit_manager,
        "ujake",
        &[coin(1000, "uusdc")],
    );
    assert_err(error_res, ContractError::CannotLiquidateCreditManager {});
}

#[test]
fn liquidate_if_no_coins_sent() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("liquidator", &[]);
    let msg = ExecuteMsg::Liquidate {
        user: "user".to_string(),
        collateral_denom: "collateral".to_string(),
        recipient: None,
    };
    let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(error_res, PaymentError::NoFunds {}.into());
}

#[test]
fn liquidate_if_many_coins_sent() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("liquidator", &[coin(100, "somecoin1"), coin(200, "somecoin2")]);
    let msg = ExecuteMsg::Liquidate {
        user: "user".to_string(),
        collateral_denom: "collateral".to_string(),
        recipient: None,
    };
    let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(error_res, PaymentError::MultipleDenoms {}.into());
}

#[test]
fn liquidate_if_no_requested_collateral() {
    let mut mock_env = MockEnvBuilder::new(None, Addr::unchecked("owner")).build();

    let red_bank = mock_env.red_bank.clone();
    let oracle = mock_env.oracle.clone();

    let (_, _, liquidatee, liquidator) = setup_env(&mut mock_env);

    // change price to be able to liquidate
    oracle.set_price_source_fixed(&mut mock_env, "uosmo", Decimal::from_ratio(3u128, 1u128));
    oracle.set_price_source_fixed(&mut mock_env, "uusdc", Decimal::from_ratio(85u128, 10u128));

    // liquidate user
    let error_res = red_bank.liquidate(
        &mut mock_env,
        &liquidator,
        &liquidatee,
        "other",
        &[coin(1000, "uusdc")],
    );
    assert_err(error_res, ContractError::CannotLiquidateWhenNoCollateralBalance {});
}

#[test]
fn liquidate_if_no_requested_debt() {
    let mut mock_env = MockEnvBuilder::new(None, Addr::unchecked("owner")).build();

    let red_bank = mock_env.red_bank.clone();
    let oracle = mock_env.oracle.clone();

    let (_, _, liquidatee, liquidator) = setup_env(&mut mock_env);

    // change price to be able to liquidate
    oracle.set_price_source_fixed(&mut mock_env, "uosmo", Decimal::from_ratio(3u128, 1u128));
    oracle.set_price_source_fixed(&mut mock_env, "uusdc", Decimal::from_ratio(85u128, 10u128));

    // liquidate user
    let error_res = red_bank.liquidate(
        &mut mock_env,
        &liquidator,
        &liquidatee,
        "uosmo",
        &[coin(1000, "other")],
    );
    assert_err(error_res, ContractError::CannotLiquidateWhenNoDebtBalance {});
}

#[test]
fn liquidate_if_requested_collateral_disabled() {
    let mut mock_env = MockEnvBuilder::new(None, Addr::unchecked("owner")).build();

    let red_bank = mock_env.red_bank.clone();
    let oracle = mock_env.oracle.clone();

    let (_, _, liquidatee, liquidator) = setup_env(&mut mock_env);

    // disable osmo collateral for liquidatee
    red_bank.update_user_collateral_status(&mut mock_env, &liquidatee, "ujake", false);

    // change price to be able to liquidate
    oracle.set_price_source_fixed(&mut mock_env, "uusdc", Decimal::from_ratio(85u128, 10u128));

    // liquidate user
    let error_res = red_bank.liquidate(
        &mut mock_env,
        &liquidator,
        &liquidatee,
        "ujake",
        &[coin(1000, "uusdc")],
    );
    assert_err(
        error_res,
        ContractError::CannotLiquidateWhenCollateralUnset {
            denom: "ujake".to_string(),
        },
    );
}

#[test]
fn cannot_liquidate_healthy_position() {
    let mut mock_env = MockEnvBuilder::new(None, Addr::unchecked("owner"))
        .target_health_factor(Decimal::from_ratio(12u128, 10u128))
        .build();

    let red_bank = mock_env.red_bank.clone();

    let (_, _, liquidatee, liquidator) = setup_env(&mut mock_env);

    // liquidate user
    let error_res = red_bank.liquidate(
        &mut mock_env,
        &liquidator,
        &liquidatee,
        "uosmo",
        &[coin(1000, "uusdc")],
    );
    assert_err(error_res, ContractError::CannotLiquidateHealthyPosition {});
}

#[test]
fn target_health_factor_reached_after_max_debt_repayed() {
    let mut mock_env = MockEnvBuilder::new(None, Addr::unchecked("owner"))
        .target_health_factor(Decimal::from_ratio(12u128, 10u128))
        .build();

    let red_bank = mock_env.red_bank.clone();
    let oracle = mock_env.oracle.clone();
    let rewards_collector = mock_env.rewards_collector.clone();

    let (funded_amt, provider, liquidatee, liquidator) = setup_env(&mut mock_env);

    // change price to be able to liquidate
    oracle.set_price_source_fixed(&mut mock_env, "uosmo", Decimal::from_ratio(3u128, 1u128));
    oracle.set_price_source_fixed(&mut mock_env, "uusdc", Decimal::from_ratio(85u128, 10u128));

    // liquidatee should be liquidatable
    let liquidatee_position = red_bank.query_user_position(&mut mock_env, &liquidatee);
    let prev_liq_threshold_hf = liq_threshold_hf(&liquidatee_position);

    // liquidate user
    let usdc_repay_amt = 2373;
    red_bank
        .liquidate(
            &mut mock_env,
            &liquidator,
            &liquidatee,
            "uosmo",
            &[coin(usdc_repay_amt, "uusdc")],
        )
        .unwrap();

    // check provider positions
    let provider_collaterals = red_bank.query_user_collaterals(&mut mock_env, &provider);
    assert_eq!(provider_collaterals.len(), 2);
    assert_eq!(provider_collaterals.get("uusdc").unwrap().amount.u128(), 1000000);
    assert_eq!(provider_collaterals.get("untrn").unwrap().amount.u128(), 1000000);
    let provider_debts = red_bank.query_user_debts(&mut mock_env, &provider);
    assert_eq!(provider_debts.len(), 0);

    // check liquidatee positions
    let liquidatee_collaterals = red_bank.query_user_collaterals(&mut mock_env, &liquidatee);
    assert_eq!(liquidatee_collaterals.len(), 3);
    assert_eq!(liquidatee_collaterals.get("uosmo").unwrap().amount.u128(), 2809);
    assert_eq!(liquidatee_collaterals.get("ujake").unwrap().amount.u128(), 2000);
    assert_eq!(liquidatee_collaterals.get("uatom").unwrap().amount.u128(), 900);
    let liquidatee_debts = red_bank.query_user_debts(&mut mock_env, &liquidatee);
    assert_eq!(liquidatee_debts.len(), 2);
    assert_eq!(liquidatee_debts.get("uusdc").unwrap().amount.u128(), 627);
    assert_eq!(liquidatee_debts.get("untrn").unwrap().amount.u128(), 1200);

    // check liquidator positions
    let liquidator_collaterals = red_bank.query_user_collaterals(&mut mock_env, &liquidator);
    assert_eq!(liquidator_collaterals.len(), 1);
    assert_eq!(liquidator_collaterals.get("uosmo").unwrap().amount.u128(), 7182);
    let liquidator_debts = red_bank.query_user_debts(&mut mock_env, &liquidator);
    assert_eq!(liquidator_debts.len(), 0);

    // check rewards-collector positions (protocol fee)
    let rc_collaterals =
        red_bank.query_user_collaterals(&mut mock_env, &rewards_collector.contract_addr);
    assert_eq!(rc_collaterals.len(), 1);
    assert_eq!(rc_collaterals.get("uosmo").unwrap().amount.u128(), 9);
    let rc_debts = red_bank.query_user_debts(&mut mock_env, &rewards_collector.contract_addr);
    assert_eq!(rc_debts.len(), 0);

    let (merged_collaterals, merged_debts, merged_balances) = merge_collaterals_and_debts(
        &[&provider_collaterals, &liquidatee_collaterals, &liquidator_collaterals, &rc_collaterals],
        &[&provider_debts, &liquidatee_debts, &liquidator_debts, &rc_debts],
    );

    // check if users collaterals and debts are equal to markets scaled amounts
    assert_users_and_markets_scaled_amounts(&mut mock_env, merged_collaterals, merged_debts);

    // check red bank underlying balances
    assert_underlying_balances(&mock_env, merged_balances);

    // check liquidator account balance
    let omso_liquidator_balance = mock_env.query_balance(&liquidator, "uosmo").unwrap();
    assert_eq!(omso_liquidator_balance.amount.u128(), funded_amt);
    let usdc_liquidator_balance = mock_env.query_balance(&liquidator, "uusdc").unwrap();
    assert_eq!(usdc_liquidator_balance.amount.u128(), funded_amt - usdc_repay_amt);

    // liquidatee hf should improve
    let liquidatee_position = red_bank.query_user_position(&mut mock_env, &liquidatee);
    let liq_threshold_hf = liq_threshold_hf(&liquidatee_position);
    assert!(liq_threshold_hf > prev_liq_threshold_hf);
    // it should be 1.2, but because of roundings it is hard to achieve an exact number
    assert_eq!(liq_threshold_hf, Decimal::from_str("1.200016765864699471").unwrap());
}

#[test]
fn debt_amt_adjusted_to_total_debt_then_refund() {
    let mut mock_env = MockEnvBuilder::new(None, Addr::unchecked("owner"))
        .target_health_factor(Decimal::from_ratio(12u128, 10u128))
        .build();

    let red_bank = mock_env.red_bank.clone();
    let oracle = mock_env.oracle.clone();
    let rewards_collector = mock_env.rewards_collector.clone();

    let (funded_amt, provider, liquidatee, liquidator) = setup_env(&mut mock_env);

    // change price to be able to liquidate
    oracle.set_price_source_fixed(&mut mock_env, "uosmo", Decimal::from_ratio(25u128, 10u128));
    oracle.set_price_source_fixed(&mut mock_env, "uusdc", Decimal::from_ratio(755u128, 100u128));

    // liquidatee should be liquidatable
    let liquidatee_position = red_bank.query_user_position(&mut mock_env, &liquidatee);
    let prev_liq_threshold_hf = liq_threshold_hf(&liquidatee_position);

    // liquidate user
    let usdc_repay_amt = 3250;
    red_bank
        .liquidate(
            &mut mock_env,
            &liquidator,
            &liquidatee,
            "uosmo",
            &[coin(usdc_repay_amt, "uusdc")],
        )
        .unwrap();

    // check provider positions
    let provider_collaterals = red_bank.query_user_collaterals(&mut mock_env, &provider);
    assert_eq!(provider_collaterals.len(), 2);
    assert_eq!(provider_collaterals.get("uusdc").unwrap().amount.u128(), 1000000);
    assert_eq!(provider_collaterals.get("untrn").unwrap().amount.u128(), 1000000);
    let provider_debts = red_bank.query_user_debts(&mut mock_env, &provider);
    assert_eq!(provider_debts.len(), 0);

    // check liquidatee positions (no usdc debt, fully repayed)
    let liquidatee_collaterals = red_bank.query_user_collaterals(&mut mock_env, &liquidatee);
    assert_eq!(liquidatee_collaterals.len(), 3);
    assert_eq!(liquidatee_collaterals.get("uosmo").unwrap().amount.u128(), 34);
    assert_eq!(liquidatee_collaterals.get("ujake").unwrap().amount.u128(), 2000);
    assert_eq!(liquidatee_collaterals.get("uatom").unwrap().amount.u128(), 900);
    let liquidatee_debts = red_bank.query_user_debts(&mut mock_env, &liquidatee);
    assert_eq!(liquidatee_debts.len(), 1);
    assert_eq!(liquidatee_debts.get("untrn").unwrap().amount.u128(), 1200);

    // check liquidator positions
    let liquidator_collaterals = red_bank.query_user_collaterals(&mut mock_env, &liquidator);
    assert_eq!(liquidator_collaterals.len(), 1);
    assert_eq!(liquidator_collaterals.get("uosmo").unwrap().amount.u128(), 9948);
    let liquidator_debts = red_bank.query_user_debts(&mut mock_env, &liquidator);
    assert_eq!(liquidator_debts.len(), 0);

    // check rewards-collector positions (protocol fee)
    let rc_collaterals =
        red_bank.query_user_collaterals(&mut mock_env, &rewards_collector.contract_addr);
    assert_eq!(rc_collaterals.len(), 1);
    assert_eq!(rc_collaterals.get("uosmo").unwrap().amount.u128(), 18);
    let rc_debts = red_bank.query_user_debts(&mut mock_env, &rewards_collector.contract_addr);
    assert_eq!(rc_debts.len(), 0);

    let (merged_collaterals, merged_debts, merged_balances) = merge_collaterals_and_debts(
        &[&provider_collaterals, &liquidatee_collaterals, &liquidator_collaterals, &rc_collaterals],
        &[&provider_debts, &liquidatee_debts, &liquidator_debts, &rc_debts],
    );

    // check if users collaterals and debts are equal to markets scaled amounts
    assert_users_and_markets_scaled_amounts(&mut mock_env, merged_collaterals, merged_debts);

    // check red bank underlying balances
    assert_underlying_balances(&mock_env, merged_balances);

    // check liquidator account balance
    let omso_liquidator_balance = mock_env.query_balance(&liquidator, "uosmo").unwrap();
    assert_eq!(omso_liquidator_balance.amount.u128(), funded_amt);
    let usdc_liquidator_balance = mock_env.query_balance(&liquidator, "uusdc").unwrap();
    assert_eq!(usdc_liquidator_balance.amount.u128(), funded_amt - usdc_repay_amt + 250); // 250 refunded

    // liquidatee hf should improve
    let liquidatee_position = red_bank.query_user_position(&mut mock_env, &liquidatee);
    let liq_threshold_hf = liq_threshold_hf(&liquidatee_position);
    assert!(liq_threshold_hf > prev_liq_threshold_hf);
}

#[test]
fn debt_amt_adjusted_to_max_allowed_by_requested_coin() {
    let mut mock_env = MockEnvBuilder::new(None, Addr::unchecked("owner"))
        .target_health_factor(Decimal::from_ratio(12u128, 10u128))
        .build();

    let red_bank = mock_env.red_bank.clone();
    let oracle = mock_env.oracle.clone();
    let rewards_collector = mock_env.rewards_collector.clone();

    let (funded_amt, provider, liquidatee, liquidator) = setup_env(&mut mock_env);

    // change price to be able to liquidate
    oracle.set_price_source_fixed(&mut mock_env, "uosmo", Decimal::from_ratio(2u128, 1u128));
    oracle.set_price_source_fixed(&mut mock_env, "uusdc", Decimal::from_ratio(64u128, 10u128));

    // liquidatee should be liquidatable
    let liquidatee_position = red_bank.query_user_position(&mut mock_env, &liquidatee);
    let prev_liq_threshold_hf = liq_threshold_hf(&liquidatee_position);

    // liquidate user
    let usdc_repay_amt = 2840;
    red_bank
        .liquidate(
            &mut mock_env,
            &liquidator,
            &liquidatee,
            "uosmo",
            &[coin(usdc_repay_amt, "uusdc")],
        )
        .unwrap();

    // check provider positions
    let provider_collaterals = red_bank.query_user_collaterals(&mut mock_env, &provider);
    assert_eq!(provider_collaterals.len(), 2);
    assert_eq!(provider_collaterals.get("uusdc").unwrap().amount.u128(), 1000000);
    assert_eq!(provider_collaterals.get("untrn").unwrap().amount.u128(), 1000000);
    let provider_debts = red_bank.query_user_debts(&mut mock_env, &provider);
    assert_eq!(provider_debts.len(), 0);

    // check liquidatee positions
    let liquidatee_collaterals = red_bank.query_user_collaterals(&mut mock_env, &liquidatee);
    assert_eq!(liquidatee_collaterals.len(), 3);
    assert_eq!(liquidatee_collaterals.get("uosmo").unwrap().amount.u128(), 4);
    assert_eq!(liquidatee_collaterals.get("ujake").unwrap().amount.u128(), 2000);
    assert_eq!(liquidatee_collaterals.get("uatom").unwrap().amount.u128(), 900);
    let liquidatee_debts = red_bank.query_user_debts(&mut mock_env, &liquidatee);
    assert_eq!(liquidatee_debts.len(), 2);
    assert_eq!(liquidatee_debts.get("uusdc").unwrap().amount.u128(), 160);
    assert_eq!(liquidatee_debts.get("untrn").unwrap().amount.u128(), 1200);

    // check liquidator positions
    let liquidator_collaterals = red_bank.query_user_collaterals(&mut mock_env, &liquidator);
    assert_eq!(liquidator_collaterals.len(), 1);
    assert_eq!(liquidator_collaterals.get("uosmo").unwrap().amount.u128(), 9978);
    let liquidator_debts = red_bank.query_user_debts(&mut mock_env, &liquidator);
    assert_eq!(liquidator_debts.len(), 0);

    // check rewards-collector positions (protocol fee)
    let rc_collaterals =
        red_bank.query_user_collaterals(&mut mock_env, &rewards_collector.contract_addr);
    assert_eq!(rc_collaterals.len(), 1);
    assert_eq!(rc_collaterals.get("uosmo").unwrap().amount.u128(), 18);
    let rc_debts = red_bank.query_user_debts(&mut mock_env, &rewards_collector.contract_addr);
    assert_eq!(rc_debts.len(), 0);

    let (merged_collaterals, merged_debts, merged_balances) = merge_collaterals_and_debts(
        &[&provider_collaterals, &liquidatee_collaterals, &liquidator_collaterals, &rc_collaterals],
        &[&provider_debts, &liquidatee_debts, &liquidator_debts, &rc_debts],
    );

    // check if users collaterals and debts are equal to markets scaled amounts
    assert_users_and_markets_scaled_amounts(&mut mock_env, merged_collaterals, merged_debts);

    // check red bank underlying balances
    assert_underlying_balances(&mock_env, merged_balances);

    // check liquidator account balance
    let omso_liquidator_balance = mock_env.query_balance(&liquidator, "uosmo").unwrap();
    assert_eq!(omso_liquidator_balance.amount.u128(), funded_amt);
    let usdc_liquidator_balance = mock_env.query_balance(&liquidator, "uusdc").unwrap();
    assert_eq!(usdc_liquidator_balance.amount.u128(), funded_amt - usdc_repay_amt);

    // liquidatee hf should improve
    let liquidatee_position = red_bank.query_user_position(&mut mock_env, &liquidatee);
    let liq_threshold_hf = liq_threshold_hf(&liquidatee_position);
    assert!(liq_threshold_hf > prev_liq_threshold_hf);
}

#[test]
fn debt_amt_no_adjustment_with_different_recipient() {
    let mut mock_env = MockEnvBuilder::new(None, Addr::unchecked("owner"))
        .target_health_factor(Decimal::from_ratio(12u128, 10u128))
        .build();

    let red_bank = mock_env.red_bank.clone();
    let oracle = mock_env.oracle.clone();
    let rewards_collector = mock_env.rewards_collector.clone();

    let (funded_amt, provider, liquidatee, liquidator) = setup_env(&mut mock_env);

    // change price to be able to liquidate
    oracle.set_price_source_fixed(&mut mock_env, "uusdc", Decimal::from_ratio(68u128, 10u128));

    // liquidatee should be liquidatable
    let liquidatee_position = red_bank.query_user_position(&mut mock_env, &liquidatee);
    let prev_liq_threshold_hf = liq_threshold_hf(&liquidatee_position);

    // liquidate user
    let usdc_repay_amt = 120;
    let recipient = Addr::unchecked("recipient");
    red_bank
        .liquidate_with_different_recipient(
            &mut mock_env,
            &liquidator,
            &liquidatee,
            "uosmo",
            &[coin(usdc_repay_amt, "uusdc")],
            Some(recipient.to_string()),
        )
        .unwrap();

    // check provider positions
    let provider_collaterals = red_bank.query_user_collaterals(&mut mock_env, &provider);
    assert_eq!(provider_collaterals.len(), 2);
    assert_eq!(provider_collaterals.get("uusdc").unwrap().amount.u128(), 1000000);
    assert_eq!(provider_collaterals.get("untrn").unwrap().amount.u128(), 1000000);
    let provider_debts = red_bank.query_user_debts(&mut mock_env, &provider);
    assert_eq!(provider_debts.len(), 0);

    // check liquidatee positions
    let liquidatee_collaterals = red_bank.query_user_collaterals(&mut mock_env, &liquidatee);
    assert_eq!(liquidatee_collaterals.len(), 3);
    assert_eq!(liquidatee_collaterals.get("uosmo").unwrap().amount.u128(), 9593);
    assert_eq!(liquidatee_collaterals.get("ujake").unwrap().amount.u128(), 2000);
    assert_eq!(liquidatee_collaterals.get("uatom").unwrap().amount.u128(), 900);
    let liquidatee_debts = red_bank.query_user_debts(&mut mock_env, &liquidatee);
    assert_eq!(liquidatee_debts.len(), 2);
    assert_eq!(liquidatee_debts.get("uusdc").unwrap().amount.u128(), 2880);
    assert_eq!(liquidatee_debts.get("untrn").unwrap().amount.u128(), 1200);

    // check liquidator positions
    let liquidator_collaterals = red_bank.query_user_collaterals(&mut mock_env, &liquidator);
    assert_eq!(liquidator_collaterals.len(), 0);
    let liquidator_debts = red_bank.query_user_debts(&mut mock_env, &liquidator);
    assert_eq!(liquidator_debts.len(), 0);

    // check recipient positions
    let recipient_collaterals = red_bank.query_user_collaterals(&mut mock_env, &recipient);
    assert_eq!(recipient_collaterals.len(), 1);
    assert_eq!(recipient_collaterals.get("uosmo").unwrap().amount.u128(), 407);
    let recipient_debts = red_bank.query_user_debts(&mut mock_env, &recipient);
    assert_eq!(recipient_debts.len(), 0);

    // check rewards-collector positions (protocol fee)
    let rc_collaterals =
        red_bank.query_user_collaterals(&mut mock_env, &rewards_collector.contract_addr);
    assert_eq!(rc_collaterals.len(), 0);
    let rc_debts = red_bank.query_user_debts(&mut mock_env, &rewards_collector.contract_addr);
    assert_eq!(rc_debts.len(), 0);

    let (merged_collaterals, merged_debts, merged_balances) = merge_collaterals_and_debts(
        &[
            &provider_collaterals,
            &liquidatee_collaterals,
            &liquidator_collaterals,
            &recipient_collaterals,
            &rc_collaterals,
        ],
        &[&provider_debts, &liquidatee_debts, &liquidator_debts, &recipient_debts, &rc_debts],
    );

    // check if users collaterals and debts are equal to markets scaled amounts
    assert_users_and_markets_scaled_amounts(&mut mock_env, merged_collaterals, merged_debts);

    // check red bank underlying balances
    assert_underlying_balances(&mock_env, merged_balances);

    // check liquidator account balance
    let omso_liquidator_balance = mock_env.query_balance(&liquidator, "uosmo").unwrap();
    assert_eq!(omso_liquidator_balance.amount.u128(), funded_amt);
    let usdc_liquidator_balance = mock_env.query_balance(&liquidator, "uusdc").unwrap();
    assert_eq!(usdc_liquidator_balance.amount.u128(), funded_amt - usdc_repay_amt);

    // liquidatee hf should improve
    let liquidatee_position = red_bank.query_user_position(&mut mock_env, &liquidatee);
    let liq_threshold_hf = liq_threshold_hf(&liquidatee_position);
    assert!(liq_threshold_hf > prev_liq_threshold_hf);
}

#[test]
fn same_asset_for_debt_and_collateral_with_refund() {
    let mut mock_env = MockEnvBuilder::new(None, Addr::unchecked("owner"))
        .target_health_factor(Decimal::from_ratio(12u128, 10u128))
        .build();

    let red_bank = mock_env.red_bank.clone();
    let params = mock_env.params.clone();
    let oracle = mock_env.oracle.clone();
    let rewards_collector = mock_env.rewards_collector.clone();

    let funded_amt = 1_000_000_000_000u128;
    let provider = Addr::unchecked("provider"); // provides collateral to be borrowed by others
    let liquidatee = Addr::unchecked("liquidatee");
    let liquidator = Addr::unchecked("liquidator");

    // setup red-bank
    let (market_params, asset_params) =
        default_asset_params_with("uosmo", Decimal::percent(70), Decimal::percent(78));
    red_bank.init_asset(&mut mock_env, &asset_params.denom, market_params);
    params.init_params(&mut mock_env, asset_params);
    let (market_params, asset_params) =
        default_asset_params_with("uatom", Decimal::percent(82), Decimal::percent(90));
    red_bank.init_asset(&mut mock_env, &asset_params.denom, market_params);
    params.init_params(&mut mock_env, asset_params);

    // setup oracle
    oracle.set_price_source_fixed(&mut mock_env, "uosmo", Decimal::from_ratio(15u128, 10u128));
    oracle.set_price_source_fixed(&mut mock_env, "uatom", Decimal::from_ratio(10u128, 1u128));

    // fund accounts
    mock_env.fund_accounts(&[&provider, &liquidatee, &liquidator], funded_amt, &["uosmo", "uatom"]);

    // provider deposits collaterals
    red_bank.deposit(&mut mock_env, &provider, coin(1000000, "uosmo")).unwrap();

    // liquidatee deposits and borrows
    red_bank.deposit(&mut mock_env, &liquidatee, coin(1000, "uosmo")).unwrap();
    red_bank.deposit(&mut mock_env, &liquidatee, coin(1000, "uatom")).unwrap();
    red_bank.borrow(&mut mock_env, &liquidatee, "uosmo", 3000).unwrap();

    // change price to be able to liquidate
    oracle.set_price_source_fixed(&mut mock_env, "uatom", Decimal::from_ratio(2u128, 1u128));

    // liquidatee should be liquidatable
    let liquidatee_position = red_bank.query_user_position(&mut mock_env, &liquidatee);
    let prev_liq_threshold_hf = liq_threshold_hf(&liquidatee_position);

    // liquidate user
    let osmo_repay_amt = 1000;
    red_bank
        .liquidate(
            &mut mock_env,
            &liquidator,
            &liquidatee,
            "uosmo",
            &[coin(osmo_repay_amt, "uosmo")],
        )
        .unwrap();

    // check provider positions
    let provider_collaterals = red_bank.query_user_collaterals(&mut mock_env, &provider);
    assert_eq!(provider_collaterals.len(), 1);
    assert_eq!(provider_collaterals.get("uosmo").unwrap().amount.u128(), 1000000);
    let provider_debts = red_bank.query_user_debts(&mut mock_env, &provider);
    assert_eq!(provider_debts.len(), 0);

    // check liquidatee positions
    let liquidatee_collaterals = red_bank.query_user_collaterals(&mut mock_env, &liquidatee);
    assert_eq!(liquidatee_collaterals.len(), 2);
    assert_eq!(liquidatee_collaterals.get("uosmo").unwrap().amount.u128(), 1);
    assert_eq!(liquidatee_collaterals.get("uatom").unwrap().amount.u128(), 1000);
    let liquidatee_debts = red_bank.query_user_debts(&mut mock_env, &liquidatee);
    assert_eq!(liquidatee_debts.len(), 1);
    assert_eq!(liquidatee_debts.get("uosmo").unwrap().amount.u128(), 2020);

    // check liquidator positions
    let liquidator_collaterals = red_bank.query_user_collaterals(&mut mock_env, &liquidator);
    assert_eq!(liquidator_collaterals.len(), 1);
    assert_eq!(liquidator_collaterals.get("uosmo").unwrap().amount.u128(), 999);
    let liquidator_debts = red_bank.query_user_debts(&mut mock_env, &liquidator);
    assert_eq!(liquidator_debts.len(), 0);

    // check rewards-collector positions (protocol fee)
    let rc_collaterals =
        red_bank.query_user_collaterals(&mut mock_env, &rewards_collector.contract_addr);
    assert_eq!(rc_collaterals.len(), 0);
    let rc_debts = red_bank.query_user_debts(&mut mock_env, &rewards_collector.contract_addr);
    assert_eq!(rc_debts.len(), 0);

    let (merged_collaterals, merged_debts, merged_balances) = merge_collaterals_and_debts(
        &[&provider_collaterals, &liquidatee_collaterals, &liquidator_collaterals, &rc_collaterals],
        &[&provider_debts, &liquidatee_debts, &liquidator_debts, &rc_debts],
    );

    // check if users collaterals and debts are equal to markets scaled amounts
    let markets = red_bank.query_markets(&mut mock_env);
    assert_eq!(markets.len(), 2);
    let osmo_market = markets.get("uosmo").unwrap();
    let atom_market = markets.get("uatom").unwrap();
    assert_eq!(merged_collaterals.get_or_default("uosmo"), osmo_market.collateral_total_scaled);
    assert_eq!(merged_debts.get_or_default("uosmo"), osmo_market.debt_total_scaled);
    assert_eq!(merged_collaterals.get_or_default("uatom"), atom_market.collateral_total_scaled);
    assert_eq!(merged_debts.get_or_default("uatom"), atom_market.debt_total_scaled);

    // check red bank underlying balances
    let balances = mock_env.query_all_balances(&red_bank.contract_addr);
    assert_eq!(merged_balances.get("uosmo"), balances.get("uosmo"));
    assert_eq!(merged_balances.get("uatom"), balances.get("uatom"));

    // check liquidator account balance
    let usdc_liquidator_balance = mock_env.query_balance(&liquidator, "uosmo").unwrap();
    assert_eq!(usdc_liquidator_balance.amount.u128(), funded_amt - osmo_repay_amt + 20); // 20 refunded

    // liquidatee hf degradated
    let liquidatee_position = red_bank.query_user_position(&mut mock_env, &liquidatee);
    let liq_threshold_hf = liq_threshold_hf(&liquidatee_position);
    assert!(liq_threshold_hf < prev_liq_threshold_hf);
}

#[test]
fn mdr_negative() {
    let mut mock_env = MockEnvBuilder::new(None, Addr::unchecked("owner"))
        .target_health_factor(Decimal::from_ratio(104u128, 100u128))
        .build();

    let red_bank = mock_env.red_bank.clone();
    let params = mock_env.params.clone();
    let oracle = mock_env.oracle.clone();
    let rewards_collector = mock_env.rewards_collector.clone();

    let funded_amt = 1_000_000_000_000u128;
    let provider = Addr::unchecked("provider"); // provides collateral to be borrowed by others
    let liquidatee = Addr::unchecked("liquidatee");
    let liquidator = Addr::unchecked("liquidator");

    // setup red-bank
    let (market_params, asset_params) = _default_asset_params_with(
        "uosmo",
        Decimal::percent(70),
        Decimal::percent(98),
        LiquidationBonus {
            starting_lb: Decimal::percent(10),
            slope: Decimal::from_str("2.0").unwrap(),
            min_lb: Decimal::percent(10),
            max_lb: Decimal::percent(10),
        },
    );
    red_bank.init_asset(&mut mock_env, &asset_params.denom, market_params);
    params.init_params(&mut mock_env, asset_params);
    let (market_params, asset_params) =
        default_asset_params_with("ujake", Decimal::percent(50), Decimal::percent(55));
    red_bank.init_asset(&mut mock_env, &asset_params.denom, market_params);
    params.init_params(&mut mock_env, asset_params);
    let (market_params, asset_params) =
        default_asset_params_with("uusdc", Decimal::percent(82), Decimal::percent(90));
    red_bank.init_asset(&mut mock_env, &asset_params.denom, market_params);
    params.init_params(&mut mock_env, asset_params);

    // setup oracle
    oracle.set_price_source_fixed(&mut mock_env, "uosmo", Decimal::from_ratio(3u128, 1u128));
    oracle.set_price_source_fixed(&mut mock_env, "ujake", Decimal::one());
    oracle.set_price_source_fixed(&mut mock_env, "uusdc", Decimal::from_ratio(2u128, 1u128));

    // fund accounts
    mock_env.fund_accounts(
        &[&provider, &liquidatee, &liquidator],
        funded_amt,
        &["uosmo", "ujake", "uusdc"],
    );

    // provider deposits collaterals
    red_bank.deposit(&mut mock_env, &provider, coin(1000000, "uusdc")).unwrap();

    // liquidatee deposits and borrows
    red_bank.deposit(&mut mock_env, &liquidatee, coin(10000, "uosmo")).unwrap();
    red_bank.deposit(&mut mock_env, &liquidatee, coin(2000, "ujake")).unwrap();
    red_bank.borrow(&mut mock_env, &liquidatee, "uusdc", 3000).unwrap();

    // change price to be able to liquidate
    oracle.set_price_source_fixed(&mut mock_env, "uusdc", Decimal::from_ratio(12u128, 1u128));

    // liquidatee should be liquidatable
    let liquidatee_position = red_bank.query_user_position(&mut mock_env, &liquidatee);
    let prev_liq_threshold_hf = liq_threshold_hf(&liquidatee_position);

    // liquidate user
    let usdc_repay_amt = 3000;
    red_bank
        .liquidate(
            &mut mock_env,
            &liquidator,
            &liquidatee,
            "uosmo",
            &[coin(usdc_repay_amt, "uusdc")],
        )
        .unwrap();

    // check provider positions
    let provider_collaterals = red_bank.query_user_collaterals(&mut mock_env, &provider);
    assert_eq!(provider_collaterals.len(), 1);
    assert_eq!(provider_collaterals.get("uusdc").unwrap().amount.u128(), 1000000);
    let provider_debts = red_bank.query_user_debts(&mut mock_env, &provider);
    assert_eq!(provider_debts.len(), 0);

    // check liquidatee positions
    let liquidatee_collaterals = red_bank.query_user_collaterals(&mut mock_env, &liquidatee);
    assert_eq!(liquidatee_collaterals.len(), 2);
    assert_eq!(liquidatee_collaterals.get("uosmo").unwrap().amount.u128(), 4);
    assert_eq!(liquidatee_collaterals.get("ujake").unwrap().amount.u128(), 2000);
    let liquidatee_debts = red_bank.query_user_debts(&mut mock_env, &liquidatee);
    assert_eq!(liquidatee_debts.len(), 1);
    assert_eq!(liquidatee_debts.get("uusdc").unwrap().amount.u128(), 728);

    // check liquidator positions
    let liquidator_collaterals = red_bank.query_user_collaterals(&mut mock_env, &liquidator);
    assert_eq!(liquidator_collaterals.len(), 1);
    assert_eq!(liquidator_collaterals.get("uosmo").unwrap().amount.u128(), 9978);
    let liquidator_debts = red_bank.query_user_debts(&mut mock_env, &liquidator);
    assert_eq!(liquidator_debts.len(), 0);

    // check rewards-collector positions (protocol fee)
    let rc_collaterals =
        red_bank.query_user_collaterals(&mut mock_env, &rewards_collector.contract_addr);
    assert_eq!(rc_collaterals.len(), 1);
    assert_eq!(rc_collaterals.get("uosmo").unwrap().amount.u128(), 18);
    let rc_debts = red_bank.query_user_debts(&mut mock_env, &rewards_collector.contract_addr);
    assert_eq!(rc_debts.len(), 0);

    let (merged_collaterals, merged_debts, merged_balances) = merge_collaterals_and_debts(
        &[&provider_collaterals, &liquidatee_collaterals, &liquidator_collaterals, &rc_collaterals],
        &[&provider_debts, &liquidatee_debts, &liquidator_debts, &rc_debts],
    );

    // check if users collaterals and debts are equal to markets scaled amounts
    let markets = red_bank.query_markets(&mut mock_env);
    assert_eq!(markets.len(), 3);
    let osmo_market = markets.get("uosmo").unwrap();
    let jake_market = markets.get("ujake").unwrap();
    let usdc_market = markets.get("uusdc").unwrap();
    assert_eq!(merged_collaterals.get_or_default("uosmo"), osmo_market.collateral_total_scaled);
    assert_eq!(merged_debts.get_or_default("uosmo"), osmo_market.debt_total_scaled);
    assert_eq!(merged_collaterals.get_or_default("ujake"), jake_market.collateral_total_scaled);
    assert_eq!(merged_debts.get_or_default("ujake"), jake_market.debt_total_scaled);
    assert_eq!(merged_collaterals.get_or_default("uusdc"), usdc_market.collateral_total_scaled);
    assert_eq!(merged_debts.get_or_default("uusdc"), usdc_market.debt_total_scaled);

    // check red bank underlying balances
    let balances = mock_env.query_all_balances(&red_bank.contract_addr);
    assert_eq!(merged_balances.get("uosmo"), balances.get("uosmo"));
    assert_eq!(merged_balances.get("ujake"), balances.get("ujake"));
    assert_eq!(merged_balances.get("uusdc"), balances.get("uusdc"));

    // check liquidator account balance
    let usdc_liquidator_balance = mock_env.query_balance(&liquidator, "uusdc").unwrap();
    assert_eq!(usdc_liquidator_balance.amount.u128(), funded_amt - usdc_repay_amt + 728); // 728 refunded

    // liquidatee hf degradated
    let liquidatee_position = red_bank.query_user_position(&mut mock_env, &liquidatee);
    let liq_threshold_hf = liq_threshold_hf(&liquidatee_position);
    assert!(liq_threshold_hf < prev_liq_threshold_hf);
}

#[test]
fn response_verification() {
    let provider = Addr::unchecked("provider"); // provides collateral to be borrowed by others
    let liquidatee = Addr::unchecked("liquidatee");
    let liquidator = Addr::unchecked("liquidator");

    let mut deps = th_setup(&[]);

    let env = mock_env_at_block_time(100_000);
    let info = mock_info("owner", &[]);

    let (market_params, asset_params) =
        default_asset_params_with("uosmo", Decimal::percent(70), Decimal::percent(78));
    execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        ExecuteMsg::InitAsset {
            denom: asset_params.denom.clone(),
            params: market_params,
        },
    )
    .unwrap();
    deps.querier.set_redbank_params(&asset_params.denom.clone(), asset_params);
    let (market_params, asset_params) =
        default_asset_params_with("uatom", Decimal::percent(82), Decimal::percent(90));
    execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        ExecuteMsg::InitAsset {
            denom: asset_params.denom.clone(),
            params: market_params,
        },
    )
    .unwrap();
    deps.querier.set_redbank_params(&asset_params.denom.clone(), asset_params);
    let (market_params, asset_params) =
        default_asset_params_with("uusdc", Decimal::percent(90), Decimal::percent(95));
    execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        ExecuteMsg::InitAsset {
            denom: asset_params.denom.clone(),
            params: market_params,
        },
    )
    .unwrap();
    deps.querier.set_redbank_params(&asset_params.denom.clone(), asset_params);
    let (market_params, asset_params) =
        default_asset_params_with("untrn", Decimal::percent(90), Decimal::percent(96));
    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::InitAsset {
            denom: asset_params.denom.clone(),
            params: market_params,
        },
    )
    .unwrap();
    deps.querier.set_redbank_params(&asset_params.denom.clone(), asset_params);

    deps.querier.set_oracle_price("uosmo", Decimal::from_ratio(4u128, 1u128));
    deps.querier.set_oracle_price("uatom", Decimal::from_ratio(82u128, 10u128));
    deps.querier.set_oracle_price("uusdc", Decimal::from_ratio(68u128, 10u128));
    deps.querier.set_oracle_price("untrn", Decimal::from_ratio(55u128, 10u128));

    // no deposit yet, initialize total deposit to zero
    deps.querier.set_total_deposit("uosmo", Uint128::zero());
    deps.querier.set_total_deposit("uatom", Uint128::zero());
    deps.querier.set_total_deposit("uusdc", Uint128::zero());
    deps.querier.set_total_deposit("untrn", Uint128::zero());

    // provider deposits collaterals
    execute(
        deps.as_mut(),
        env.clone(),
        mock_info(provider.as_str(), &[coin(1000000, "uusdc")]),
        ExecuteMsg::Deposit {
            account_id: None,
            on_behalf_of: None,
        },
    )
    .unwrap();
    execute(
        deps.as_mut(),
        env.clone(),
        mock_info(provider.as_str(), &[coin(1000000, "untrn")]),
        ExecuteMsg::Deposit {
            account_id: None,
            on_behalf_of: None,
        },
    )
    .unwrap();

    // liquidatee deposits and borrows
    execute(
        deps.as_mut(),
        env.clone(),
        mock_info(liquidatee.as_str(), &[coin(10000, "uosmo")]),
        ExecuteMsg::Deposit {
            account_id: None,
            on_behalf_of: None,
        },
    )
    .unwrap();
    execute(
        deps.as_mut(),
        env.clone(),
        mock_info(liquidatee.as_str(), &[coin(900, "uatom")]),
        ExecuteMsg::Deposit {
            account_id: None,
            on_behalf_of: None,
        },
    )
    .unwrap();
    execute(
        deps.as_mut(),
        env.clone(),
        mock_info(liquidatee.as_str(), &[]),
        ExecuteMsg::Borrow {
            denom: "uusdc".to_string(),
            amount: Uint128::from(3000u128),
            recipient: None,
        },
    )
    .unwrap();
    execute(
        deps.as_mut(),
        env,
        mock_info(liquidatee.as_str(), &[]),
        ExecuteMsg::Borrow {
            denom: "untrn".to_string(),
            amount: Uint128::from(1200u128),
            recipient: None,
        },
    )
    .unwrap();

    // change price to be able to liquidate
    deps.querier.set_oracle_price("uosmo", Decimal::from_ratio(2u128, 1u128));

    let collateral_market: Market = th_query(
        deps.as_ref(),
        QueryMsg::Market {
            denom: "uosmo".to_string(),
        },
    );
    let debt_market: Market = th_query(
        deps.as_ref(),
        QueryMsg::Market {
            denom: "uusdc".to_string(),
        },
    );
    let liquidatee_collateral: UserCollateralResponse = th_query(
        deps.as_ref(),
        QueryMsg::UserCollateral {
            user: liquidatee.to_string(),
            account_id: None,
            denom: "uosmo".to_string(),
        },
    );
    let liquidatee_debt: UserDebtResponse = th_query(
        deps.as_ref(),
        QueryMsg::UserDebt {
            user: liquidatee.to_string(),
            denom: "uusdc".to_string(),
        },
    );

    let debt_to_repay = 2883_u128;
    let block_time = 500_000;
    let env = mock_env_at_block_time(block_time);
    let info = mock_info(liquidator.as_str(), &[coin(debt_to_repay, "uusdc")]);
    let res = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::Liquidate {
            user: liquidatee.to_string(),
            collateral_denom: "uosmo".to_string(),
            recipient: None,
        },
    )
    .unwrap();

    let expected_debt_rates = th_get_expected_indices_and_rates(
        &debt_market,
        block_time,
        TestUtilizationDeltaInfo {
            less_debt: Uint128::new(2883u128),
            user_current_debt_scaled: liquidatee_debt.amount_scaled,
            ..Default::default()
        },
    );

    let expected_collateral_rates = th_get_expected_indices_and_rates(
        &collateral_market,
        block_time,
        TestUtilizationDeltaInfo::default(),
    );

    let debt_market_after: Market = th_query(
        deps.as_ref(),
        QueryMsg::Market {
            denom: "uusdc".to_string(),
        },
    );

    assert_eq!(debt_market_after.borrow_index, expected_debt_rates.borrow_index);
    assert_eq!(debt_market_after.liquidity_index, expected_debt_rates.liquidity_index);

    mars_testing::assert_eq_vec(
        res.attributes,
        vec![
            attr("action", "liquidate"),
            attr("user", liquidatee.as_str()),
            attr("liquidator", liquidator.as_str()),
            attr("recipient", liquidator.as_str()),
            attr("collateral_denom", "uosmo"),
            attr("collateral_amount", Uint128::new(9998u128)),
            attr(
                "collateral_amount_scaled",
                th_get_scaled_liquidity_amount(
                    Uint128::new(9998u128),
                    expected_collateral_rates.liquidity_index,
                ),
            ),
            attr("debt_denom", "uusdc"),
            attr("debt_amount", Uint128::new(2883u128)),
            attr("debt_amount_scaled", expected_debt_rates.less_debt_scaled),
        ],
    );

    assert_eq!(res.events, vec![th_build_interests_updated_event("uusdc", &expected_debt_rates)]);

    let expected_msgs = expected_messages(
        &liquidatee,
        &liquidator,
        liquidatee_collateral.amount_scaled,
        Uint128::zero(),
        &collateral_market,
        &debt_market,
    );
    assert_eq!(res.messages, expected_msgs);
}

#[test]
fn liquidation_uses_correct_price_kind() {
    let mut mock_env = MockEnvBuilder::new(None, Addr::unchecked("owner"))
        .target_health_factor(Decimal::from_ratio(12u128, 10u128))
        .build();

    let red_bank = mock_env.red_bank.clone();
    let oracle = mock_env.oracle.clone();
    let pyth = mock_env.pyth.clone();

    let (_funded_amt, provider, liquidatee, liquidator) = setup_env(&mut mock_env);

    // change price to be able to liquidate
    oracle.set_price_source_fixed(&mut mock_env, "usd", Decimal::from_str("1000000").unwrap());
    oracle.set_price_source_pyth(
        &mut mock_env,
        "uusdc",
        pyth.to_string(),
        Decimal::percent(10u64),
        Decimal::percent(15u64),
    );

    // liquidation should succeed because it uses simpler pricing for Pyth
    red_bank
        .liquidate(&mut mock_env, &liquidator, &liquidatee, "uosmo", &[coin(120, "uusdc")])
        .unwrap();

    // confidence is higher than max_confidence so borrow will fail
    red_bank.borrow(&mut mock_env, &provider, "uusdc", 300).unwrap_err();
}

// recipient - can be liquidator or another address which can receive collateral
fn expected_messages(
    user_addr: &Addr,
    recipient_addr: &Addr,
    user_collateral_scaled: Uint128,
    recipient_collateral_scaled: Uint128,
    collateral_market: &Market,
    debt_market: &Market,
) -> Vec<SubMsg> {
    // there should be up to three messages updating indices at the incentives contract, in the
    // order:
    // - collateral denom, user
    // - collateral denom, liquidator
    // - debt denom, rewards collector (if rewards accrued > 0)
    //
    // NOTE that we don't expect a message to update rewards collector's index of the
    // **collateral** asset, because the liquidation action does NOT change the collateral
    // asset's utilization rate, it's interest rate does not need to be updated.
    vec![
        SubMsg::new(WasmMsg::Execute {
            contract_addr: MarsAddressType::Incentives.to_string(),
            msg: to_json_binary(&incentives::ExecuteMsg::BalanceChange {
                user_addr: user_addr.clone(),
                account_id: None,
                denom: collateral_market.denom.clone(),
                user_amount_scaled_before: user_collateral_scaled,
                total_amount_scaled_before: collateral_market.collateral_total_scaled,
            })
            .unwrap(),
            funds: vec![],
        }),
        SubMsg::new(WasmMsg::Execute {
            contract_addr: MarsAddressType::Incentives.to_string(),
            msg: to_json_binary(&incentives::ExecuteMsg::BalanceChange {
                user_addr: recipient_addr.clone(),
                account_id: None,
                denom: collateral_market.denom.clone(),
                user_amount_scaled_before: recipient_collateral_scaled,
                total_amount_scaled_before: collateral_market.collateral_total_scaled,
            })
            .unwrap(),
            funds: vec![],
        }),
        SubMsg::new(WasmMsg::Execute {
            contract_addr: MarsAddressType::Incentives.to_string(),
            msg: to_json_binary(&incentives::ExecuteMsg::BalanceChange {
                user_addr: Addr::unchecked(MarsAddressType::RewardsCollector.to_string()),
                account_id: None,
                denom: collateral_market.denom.clone(),
                user_amount_scaled_before: Uint128::zero(),
                total_amount_scaled_before: collateral_market.collateral_total_scaled,
            })
            .unwrap(),
            funds: vec![],
        }),
        SubMsg::new(WasmMsg::Execute {
            contract_addr: MarsAddressType::Incentives.to_string(),
            msg: to_json_binary(&incentives::ExecuteMsg::BalanceChange {
                user_addr: Addr::unchecked(MarsAddressType::RewardsCollector.to_string()),
                account_id: None,
                denom: debt_market.denom.clone(),
                user_amount_scaled_before: Uint128::zero(),
                total_amount_scaled_before: debt_market.collateral_total_scaled,
            })
            .unwrap(),
            funds: vec![],
        }),
    ]
}

fn setup_env(mock_env: &mut MockEnv) -> (u128, Addr, Addr, Addr) {
    let funded_amt = 1_000_000_000_000u128;
    let provider = Addr::unchecked("provider"); // provides collateral to be borrowed by others
    let liquidatee = Addr::unchecked("liquidatee");
    let liquidator = Addr::unchecked("liquidator");

    // setup red-bank
    let red_bank = mock_env.red_bank.clone();
    let params = mock_env.params.clone();
    let (market_params, asset_params) =
        default_asset_params_with("uosmo", Decimal::percent(70), Decimal::percent(78));
    red_bank.init_asset(mock_env, &asset_params.denom, market_params);
    params.init_params(mock_env, asset_params);
    let (market_params, asset_params) =
        default_asset_params_with("ujake", Decimal::percent(50), Decimal::percent(55));
    red_bank.init_asset(mock_env, &asset_params.denom, market_params);
    params.init_params(mock_env, asset_params);
    let (market_params, asset_params) =
        default_asset_params_with("uatom", Decimal::percent(82), Decimal::percent(90));
    red_bank.init_asset(mock_env, &asset_params.denom, market_params);
    params.init_params(mock_env, asset_params);
    let (market_params, asset_params) =
        default_asset_params_with("uusdc", Decimal::percent(90), Decimal::percent(95));
    red_bank.init_asset(mock_env, &asset_params.denom, market_params);
    params.init_params(mock_env, asset_params);
    let (market_params, asset_params) =
        default_asset_params_with("untrn", Decimal::percent(90), Decimal::percent(96));
    red_bank.init_asset(mock_env, &asset_params.denom, market_params);
    params.init_params(mock_env, asset_params);

    // setup oracle
    let oracle = mock_env.oracle.clone();
    oracle.set_price_source_fixed(mock_env, "uosmo", Decimal::from_ratio(22u128, 10u128));
    oracle.set_price_source_fixed(mock_env, "ujake", Decimal::one());
    oracle.set_price_source_fixed(mock_env, "uatom", Decimal::from_ratio(82u128, 10u128));
    oracle.set_price_source_fixed(mock_env, "uusdc", Decimal::one());
    oracle.set_price_source_fixed(mock_env, "untrn", Decimal::from_ratio(55u128, 10u128));

    // fund accounts
    mock_env.fund_accounts(
        &[&provider, &liquidatee, &liquidator],
        funded_amt,
        &["uosmo", "ujake", "uatom", "uusdc", "untrn", "other"],
    );

    // provider deposits collaterals
    red_bank.deposit(mock_env, &provider, coin(1000000, "uusdc")).unwrap();
    red_bank.deposit(mock_env, &provider, coin(1000000, "untrn")).unwrap();

    // liquidatee deposits and borrows
    red_bank.deposit(mock_env, &liquidatee, coin(10000, "uosmo")).unwrap();
    red_bank.deposit(mock_env, &liquidatee, coin(2000, "ujake")).unwrap();
    red_bank.deposit(mock_env, &liquidatee, coin(900, "uatom")).unwrap();
    red_bank.borrow(mock_env, &liquidatee, "uusdc", 3000).unwrap();
    red_bank.borrow(mock_env, &liquidatee, "untrn", 1200).unwrap();

    (funded_amt, provider, liquidatee, liquidator)
}

fn assert_users_and_markets_scaled_amounts(
    mock_env: &mut MockEnv,
    merged_collaterals: HashMap<String, Uint128>,
    merged_debts: HashMap<String, Uint128>,
) {
    let red_bank = mock_env.red_bank.clone();

    let markets = red_bank.query_markets(mock_env);
    assert_eq!(markets.len(), 5);
    let osmo_market = markets.get("uosmo").unwrap();
    let jake_market = markets.get("ujake").unwrap();
    let atom_market = markets.get("uatom").unwrap();
    let usdc_market = markets.get("uusdc").unwrap();
    let ntrn_market = markets.get("untrn").unwrap();
    assert_eq!(merged_collaterals.get_or_default("uosmo"), osmo_market.collateral_total_scaled);
    assert_eq!(merged_debts.get_or_default("uosmo"), osmo_market.debt_total_scaled);
    assert_eq!(merged_collaterals.get_or_default("ujake"), jake_market.collateral_total_scaled);
    assert_eq!(merged_debts.get_or_default("ujake"), jake_market.debt_total_scaled);
    assert_eq!(merged_collaterals.get_or_default("uatom"), atom_market.collateral_total_scaled);
    assert_eq!(merged_debts.get_or_default("uatom"), atom_market.debt_total_scaled);
    assert_eq!(merged_collaterals.get_or_default("uusdc"), usdc_market.collateral_total_scaled);
    assert_eq!(merged_debts.get_or_default("uusdc"), usdc_market.debt_total_scaled);
    assert_eq!(merged_collaterals.get_or_default("untrn"), ntrn_market.collateral_total_scaled);
    assert_eq!(merged_debts.get_or_default("untrn"), ntrn_market.debt_total_scaled);
}

fn assert_underlying_balances(mock_env: &MockEnv, merged_balances: HashMap<String, Uint128>) {
    let red_bank = mock_env.red_bank.clone();

    let balances = mock_env.query_all_balances(&red_bank.contract_addr);
    assert_eq!(merged_balances.get("uosmo"), balances.get("uosmo"));
    assert_eq!(merged_balances.get("ujake"), balances.get("ujake"));
    assert_eq!(merged_balances.get("uatom"), balances.get("uatom"));
    assert_eq!(merged_balances.get("uusdc"), balances.get("uusdc"));
    assert_eq!(merged_balances.get("untrn"), balances.get("untrn"));
}

fn default_asset_params_with(
    denom: &str,
    max_loan_to_value: Decimal,
    liquidation_threshold: Decimal,
) -> (InitOrUpdateAssetParams, AssetParams) {
    _default_asset_params_with(
        denom,
        max_loan_to_value,
        liquidation_threshold,
        LiquidationBonus {
            starting_lb: Decimal::percent(1),
            slope: Decimal::from_str("2.0").unwrap(),
            min_lb: Decimal::percent(2),
            max_lb: Decimal::percent(10),
        },
    )
}

fn _default_asset_params_with(
    denom: &str,
    max_loan_to_value: Decimal,
    liquidation_threshold: Decimal,
    liquidation_bonus: LiquidationBonus,
) -> (InitOrUpdateAssetParams, AssetParams) {
    let market_params = InitOrUpdateAssetParams {
        reserve_factor: Some(Decimal::percent(20)),
        interest_rate_model: Some(InterestRateModel {
            optimal_utilization_rate: Decimal::percent(10),
            base: Decimal::percent(30),
            slope_1: Decimal::percent(25),
            slope_2: Decimal::percent(30),
        }),
    };
    let asset_params = AssetParams {
        denom: denom.to_string(),
        credit_manager: CmSettings {
            whitelisted: false,
            hls: None,
        },
        red_bank: RedBankSettings {
            deposit_enabled: true,
            borrow_enabled: true,
        },
        max_loan_to_value,
        liquidation_threshold,
        liquidation_bonus,
        protocol_liquidation_fee: Decimal::percent(2),
        deposit_cap: Uint128::MAX,
    };
    (market_params, asset_params)
}

trait MapDefaultValue {
    fn get_or_default(&self, key: &str) -> Uint128;
}

impl MapDefaultValue for HashMap<String, Uint128> {
    fn get_or_default(&self, key: &str) -> Uint128 {
        self.get(key).cloned().unwrap_or(Uint128::zero())
    }
}
