use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use mars_red_bank_types::red_bank::UserHealthStatus;
use mars_testing::integration::mock_env::MockEnvBuilder;
use mars_utils::math;

use crate::helpers::{default_asset_params, default_asset_params_with, is_user_liquidatable};

mod helpers;

#[test]
fn liquidate_collateralized_loan() {
    let close_factor = Decimal::percent(40);
    let atom_price = Decimal::from_ratio(12u128, 1u128);
    let osmo_price = Decimal::from_ratio(15u128, 10u128);
    let atom_max_ltv = Decimal::percent(60);
    let osmo_max_ltv = Decimal::percent(80);
    let atom_liq_threshold = Decimal::percent(75);
    let osmo_liq_threshold = Decimal::percent(90);
    let atom_liq_bonus = Decimal::percent(2);
    let osmo_liq_bonus = Decimal::percent(5);

    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).close_factor(close_factor).build();

    // setup oracle and red-bank
    let oracle = mock_env.oracle.clone();
    oracle.set_price_source_fixed(&mut mock_env, "uatom", atom_price);
    oracle.set_price_source_fixed(&mut mock_env, "uosmo", osmo_price);
    oracle.set_price_source_fixed(&mut mock_env, "uusdc", Decimal::one());
    let red_bank = mock_env.red_bank.clone();
    let params = mock_env.params.clone();
    let (market_params, asset_params) =
        default_asset_params_with(atom_max_ltv, atom_liq_threshold, atom_liq_bonus);
    red_bank.init_asset(&mut mock_env, "uatom", market_params);
    params.init_params(&mut mock_env, "uatom", asset_params);
    let (market_params, asset_params) =
        default_asset_params_with(osmo_max_ltv, osmo_liq_threshold, osmo_liq_bonus);
    red_bank.init_asset(&mut mock_env, "uosmo", market_params);
    params.init_params(&mut mock_env, "uosmo", asset_params);
    let (market_params, asset_params) = default_asset_params();
    red_bank.init_asset(&mut mock_env, "uusdc", market_params);
    params.init_params(&mut mock_env, "uusdc", asset_params);

    // fund provider account with usdc
    let provider = Addr::unchecked("provider");
    let funded_usdc = 1_000_000_000_000u128;
    mock_env.fund_account(&provider, &[coin(funded_usdc, "uusdc")]);

    // fund borrow account with atom and osmo
    let borrower = Addr::unchecked("borrower");
    let funded_atom = 1_250_000_000u128;
    let funded_osmo = 15_200_000_000u128;
    mock_env.fund_account(&borrower, &[coin(funded_atom, "uatom")]);
    mock_env.fund_account(&borrower, &[coin(funded_osmo, "uosmo")]);

    // fund liquidator account with usdc
    let liquidator = Addr::unchecked("liquidator");
    mock_env.fund_account(&liquidator, &[coin(1_000_000_000_000u128, "uusdc")]);

    // deposits collaterals
    red_bank.deposit(&mut mock_env, &provider, coin(funded_usdc, "uusdc")).unwrap();
    red_bank.deposit(&mut mock_env, &borrower, coin(funded_atom, "uatom")).unwrap();
    red_bank.deposit(&mut mock_env, &borrower, coin(funded_osmo, "uosmo")).unwrap();

    // check HF for borrower
    let borrower_position = red_bank.query_user_position(&mut mock_env, &borrower);
    assert_eq!(borrower_position.health_status, UserHealthStatus::NotBorrowing);

    // try to borrow more than max LTV, should fail
    let max_borrow = atom_max_ltv * (atom_price * Uint128::from(funded_atom))
        + osmo_max_ltv * (osmo_price * Uint128::from(funded_osmo));
    red_bank.borrow(&mut mock_env, &borrower, "uusdc", max_borrow.u128() + 1).unwrap_err();

    // borrow max allowed amount
    red_bank.borrow(&mut mock_env, &borrower, "uusdc", max_borrow.u128()).unwrap();
    let borrower_position = red_bank.query_user_position(&mut mock_env, &borrower);
    assert!(!is_user_liquidatable(&borrower_position));

    // decrease atom price
    let atom_price = Decimal::from_ratio(6u128, 1u128);
    oracle.set_price_source_fixed(&mut mock_env, "uatom", atom_price);

    // check HF after atom price decrease, should be < 1
    let borrower_position = red_bank.query_user_position(&mut mock_env, &borrower);
    assert!(is_user_liquidatable(&borrower_position));

    // values before liquidation
    let redbank_osmo_balance_before =
        mock_env.query_balance(&red_bank.contract_addr, "uosmo").unwrap();
    let redbank_usdc_balance_before =
        mock_env.query_balance(&red_bank.contract_addr, "uusdc").unwrap();
    let liquidator_osmo_balance_before = mock_env.query_balance(&liquidator, "uosmo").unwrap();
    let liquidator_usdc_balance_before = mock_env.query_balance(&liquidator, "uusdc").unwrap();
    let market_osmo_before = red_bank.query_market(&mut mock_env, "uosmo");
    let market_usdc_before = red_bank.query_market(&mut mock_env, "uusdc");
    let borrower_osmo_collateral_before =
        red_bank.query_user_collateral(&mut mock_env, &borrower, "uosmo");
    let borrower_usdc_debt_before = red_bank.query_user_debt(&mut mock_env, &borrower, "uusdc");
    let liquidator_osmo_collateral_before =
        red_bank.query_user_collateral(&mut mock_env, &liquidator, "uosmo");
    let borrower_position_before = red_bank.query_user_position(&mut mock_env, &borrower);

    // liquidate borrower (more than close factor in order to get refund)
    let max_amount_to_repay =
        Uint128::one() * (close_factor * borrower_position_before.total_collateralized_debt);
    let osmo_amount_to_liquidate = math::divide_uint128_by_decimal(
        max_amount_to_repay * (Decimal::one() + osmo_liq_bonus),
        osmo_price,
    )
    .unwrap();
    let refund_amount = 15_000_000u128;
    red_bank
        .liquidate(
            &mut mock_env,
            &liquidator,
            &borrower,
            "uosmo",
            coin(max_amount_to_repay.u128() + refund_amount, "uusdc"),
        )
        .unwrap();

    // redbank usdc balance is increased by repayed amount
    let redbank_usdc_balance = mock_env.query_balance(&red_bank.contract_addr, "uusdc").unwrap();
    assert_eq!(
        redbank_usdc_balance.amount,
        redbank_usdc_balance_before.amount + max_amount_to_repay
    );
    // redbank osmo balance is the same - we need to withdraw funds (collateral) manually
    let redbank_osmo_balance = mock_env.query_balance(&red_bank.contract_addr, "uosmo").unwrap();
    assert_eq!(redbank_osmo_balance.amount, redbank_osmo_balance_before.amount);

    // liquidator usdc balance should be decreased by repayed amount
    let liquidator_usdc_balance = mock_env.query_balance(&liquidator, "uusdc").unwrap();
    assert_eq!(
        liquidator_usdc_balance.amount,
        liquidator_usdc_balance_before.amount - max_amount_to_repay
    );
    // liquidator osmo balance is the same - we need to withdraw funds (collateral) manually
    let liquidator_osmo_balance = mock_env.query_balance(&liquidator, "uosmo").unwrap();
    assert_eq!(liquidator_osmo_balance.amount, liquidator_osmo_balance_before.amount);

    // usdc debt market is decreased by scaled repayed amount
    let market_usdc = red_bank.query_market(&mut mock_env, "uusdc");
    let scaled_max_amount_to_repay =
        red_bank.query_scaled_debt_amount(&mut mock_env, coin(max_amount_to_repay.u128(), "uusdc"));
    assert_eq!(
        market_usdc.debt_total_scaled,
        market_usdc_before.debt_total_scaled - scaled_max_amount_to_repay
    );
    // osmo collateral market is the same - we need to withdraw funds (collateral) manually
    let market_osmo = red_bank.query_market(&mut mock_env, "uosmo");
    assert_eq!(market_osmo.collateral_total_scaled, market_osmo_before.collateral_total_scaled);

    // borrower usdc debt is decreased by repayed amount
    let borrower_usdc_debt = red_bank.query_user_debt(&mut mock_env, &borrower, "uusdc");
    assert_eq!(borrower_usdc_debt.amount, borrower_usdc_debt_before.amount - max_amount_to_repay);
    // borrower osmo collateral is decreased by liquidated amount
    let borrower_osmo_collateral =
        red_bank.query_user_collateral(&mut mock_env, &borrower, "uosmo");
    assert_eq!(
        borrower_osmo_collateral.amount,
        borrower_osmo_collateral_before.amount - osmo_amount_to_liquidate
    );
    // liquidator osmo collateral is increased by liquidated amount
    let liquidator_osmo_collateral =
        red_bank.query_user_collateral(&mut mock_env, &liquidator, "uosmo");
    assert_eq!(
        liquidator_osmo_collateral.amount,
        liquidator_osmo_collateral_before.amount + osmo_amount_to_liquidate
    );

    // withdraw collateral for liquidator
    red_bank.withdraw(&mut mock_env, &liquidator, "uosmo", None).unwrap();
    // redbank osmo balance is decreased by liquidated amount
    let redbank_osmo_balance = mock_env.query_balance(&red_bank.contract_addr, "uosmo").unwrap();
    assert_eq!(
        redbank_osmo_balance.amount,
        redbank_osmo_balance_before.amount - osmo_amount_to_liquidate
    );
    // liquidator osmo balance is increased by liquidated amount
    let liquidator_osmo_balance = mock_env.query_balance(&liquidator, "uosmo").unwrap();
    assert_eq!(
        liquidator_osmo_balance.amount,
        liquidator_osmo_balance_before.amount + osmo_amount_to_liquidate
    );
    // liquidator osmo collateral after withdraw is the same as before liquidation
    let liquidator_osmo_collateral =
        red_bank.query_user_collateral(&mut mock_env, &liquidator, "uosmo");
    assert_eq!(liquidator_osmo_collateral.amount, liquidator_osmo_collateral_before.amount);
    // osmo collateral market is decreased by liquidated amount
    let market_osmo = red_bank.query_market(&mut mock_env, "uosmo");
    let scaled_amount_to_liquidate = red_bank.query_scaled_liquidity_amount(
        &mut mock_env,
        coin(osmo_amount_to_liquidate.u128(), "uosmo"),
    );
    assert_eq!(
        market_osmo.collateral_total_scaled,
        market_osmo_before.collateral_total_scaled - scaled_amount_to_liquidate
    );
}

#[test]
fn liquidate_uncollateralized_loan() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner.clone()).build();

    // setup oracle and red-bank
    let oracle = mock_env.oracle.clone();
    oracle.set_price_source_fixed(&mut mock_env, "uatom", Decimal::from_ratio(14u128, 1u128));
    oracle.set_price_source_fixed(&mut mock_env, "uusdc", Decimal::one());
    let red_bank = mock_env.red_bank.clone();
    let params = mock_env.params.clone();
    let (market_params, asset_params) = default_asset_params();
    red_bank.init_asset(&mut mock_env, "uatom", market_params.clone());
    red_bank.init_asset(&mut mock_env, "uusdc", market_params);
    params.init_params(&mut mock_env, "uatom", asset_params.clone());
    params.init_params(&mut mock_env, "uusdc", asset_params);

    // fund provider account with usdc
    let provider = Addr::unchecked("provider");
    let funded_usdc = 1_000_000_000_000u128;
    mock_env.fund_account(&provider, &[coin(1_000_000_000_000u128, "uusdc")]);

    // fund provider account with usdc
    let liquidator = Addr::unchecked("liquidator");
    mock_env.fund_account(&liquidator, &[coin(1_000_000_000_000u128, "uusdc")]);

    // deposits usdc to redbank
    red_bank.deposit(&mut mock_env, &provider, coin(funded_usdc, "uusdc")).unwrap();

    let borrower = Addr::unchecked("borrower");

    // set uncollateralized loan limit for borrower
    red_bank
        .update_uncollateralized_loan_limit(
            &mut mock_env,
            &owner,
            &borrower,
            "uusdc",
            Uint128::from(10_000_000_000u128),
        )
        .unwrap();

    // borrower borrows usdc
    let borrow_amount = 98_000_000u128;
    red_bank.borrow(&mut mock_env, &borrower, "uusdc", borrow_amount).unwrap();
    let balance = mock_env.query_balance(&borrower, "uusdc").unwrap();
    assert_eq!(balance.amount.u128(), borrow_amount);

    // try to liquidate, should fail because there are no collateralized loans
    red_bank
        .liquidate(&mut mock_env, &liquidator, &borrower, "uatom", coin(borrow_amount, "uusdc"))
        .unwrap_err();
}
