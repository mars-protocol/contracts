use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use mars_testing::integration::mock_env::{MockEnv, MockEnvBuilder, RedBank};

use crate::helpers::default_asset_params;

mod helpers;

#[test]
fn user_flow() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    // setup oracle and red-bank
    let oracle = mock_env.oracle.clone();
    oracle.set_price_source_fixed(&mut mock_env, "uatom", Decimal::from_ratio(12u128, 1u128));
    oracle.set_price_source_fixed(&mut mock_env, "uusdc", Decimal::one());
    let red_bank = mock_env.red_bank.clone();
    let params = mock_env.params.clone();
    let (market_params, asset_params) = default_asset_params();
    red_bank.init_asset(&mut mock_env, "uatom", market_params.clone());
    red_bank.init_asset(&mut mock_env, "uusdc", market_params);
    params.init_params(&mut mock_env, "uatom", asset_params.clone());
    params.init_params(&mut mock_env, "uusdc", asset_params);

    // fund user_1 account with atom
    let user_1 = Addr::unchecked("user_1");
    let funded_atom = 100_000_000u128;
    mock_env.fund_account(&user_1, &[coin(funded_atom, "uatom")]);
    let balance = mock_env.query_balance(&user_1, "uatom").unwrap();
    assert_eq!(balance.amount.u128(), funded_atom);

    // fund user_2 account with usdc
    let user_2 = Addr::unchecked("user_2");
    let funded_usdc = 200_000_000u128;
    mock_env.fund_account(&user_2, &[coin(funded_usdc, "uusdc")]);
    let balance = mock_env.query_balance(&user_2, "uusdc").unwrap();
    assert_eq!(balance.amount.u128(), funded_usdc);

    // user_1 deposits some atom
    let deposited_atom = 65_000_000u128;
    red_bank.deposit(&mut mock_env, &user_1, coin(deposited_atom, "uatom")).unwrap();
    let balance = mock_env.query_balance(&user_1, "uatom").unwrap();
    assert_eq!(balance.amount.u128(), funded_atom - deposited_atom);
    let collateral = red_bank.query_user_collateral(&mut mock_env, &user_1, "uatom");
    assert_eq!(collateral.amount.u128(), deposited_atom);

    // user_2 deposits all usdc balance
    let deposited_usdc = funded_usdc;
    red_bank.deposit(&mut mock_env, &user_2, coin(deposited_usdc, "uusdc")).unwrap();
    let balance = mock_env.query_balance(&user_2, "uusdc").unwrap();
    assert_eq!(balance.amount, Uint128::zero());
    let user_2_usdc_collateral = red_bank.query_user_collateral(&mut mock_env, &user_2, "uusdc");
    assert_eq!(user_2_usdc_collateral.amount.u128(), deposited_usdc);

    // move few blocks
    mock_env.increment_by_blocks(10);

    // user_1 borrows some usdc (no usdc in the account before)
    let borrowed_usdc = 125_000_000u128;
    red_bank.borrow(&mut mock_env, &user_1, "uusdc", borrowed_usdc).unwrap();
    let balance = mock_env.query_balance(&user_1, "uusdc").unwrap();
    assert_eq!(balance.amount.u128(), borrowed_usdc);
    let user_1_usdc_debt = red_bank.query_user_debt(&mut mock_env, &user_1, "uusdc");
    assert_eq!(user_1_usdc_debt.amount.u128(), borrowed_usdc);

    // move few blocks
    mock_env.increment_by_blocks(100);

    // add more usdc to user_1 account to repay full debt
    mock_env.fund_account(&user_1, &[coin(10_000_000u128, "uusdc")]);

    // few blocks passed, debt should increase for user_1
    let debt = red_bank.query_user_debt(&mut mock_env, &user_1, "uusdc");
    assert!(debt.amount > user_1_usdc_debt.amount);
    assert_eq!(debt.amount_scaled, user_1_usdc_debt.amount_scaled);

    // repay full debt for user_1
    let balance_before = mock_env.query_balance(&user_1, "uusdc").unwrap();
    let repayed = debt.amount;
    red_bank.repay(&mut mock_env, &user_1, coin(repayed.u128(), "uusdc")).unwrap();
    let balance = mock_env.query_balance(&user_1, "uusdc").unwrap();
    assert_eq!(balance.amount, balance_before.amount - repayed);
    let debt = red_bank.query_user_debt(&mut mock_env, &user_1, "uusdc");
    assert_eq!(debt.amount, Uint128::zero());
    assert_eq!(debt.amount_scaled, Uint128::zero());

    // few blocks passed, collateral should increase for user_2
    let collateral = red_bank.query_user_collateral(&mut mock_env, &user_2, "uusdc");
    assert!(collateral.amount > user_2_usdc_collateral.amount);
    assert_eq!(collateral.amount_scaled, user_2_usdc_collateral.amount_scaled);

    // withdraw full collateral for user_2
    let balance_before = mock_env.query_balance(&user_2, "uusdc").unwrap();
    red_bank.withdraw(&mut mock_env, &user_2, "uusdc", None).unwrap();
    let balance = mock_env.query_balance(&user_2, "uusdc").unwrap();
    assert_eq!(balance.amount, balance_before.amount + collateral.amount);
    let collateral = red_bank.query_user_collateral(&mut mock_env, &user_2, "uusdc");
    assert_eq!(collateral.amount, Uint128::zero());
    assert_eq!(collateral.amount_scaled, Uint128::zero());
}

#[test]
fn borrow_exact_liquidity() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    // setup oracle and red-bank
    let oracle = mock_env.oracle.clone();
    oracle.set_price_source_fixed(&mut mock_env, "uatom", Decimal::from_ratio(12u128, 1u128));
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
    let funded_usdc = 10_000_000_000_u128;
    mock_env.fund_account(&provider, &[coin(funded_usdc, "uusdc")]);

    // fund borrow account with large amount of atom
    let borrower = Addr::unchecked("borrower");
    let funded_atom = 1_000_000_000_000u128;
    mock_env.fund_account(&borrower, &[coin(funded_atom, "uatom")]);

    // provider deposits usdc
    red_bank.deposit(&mut mock_env, &provider, coin(funded_usdc, "uusdc")).unwrap();
    let balance = mock_env.query_balance(&provider, "uusdc").unwrap();
    assert_eq!(balance.amount, Uint128::zero());
    let provider_collateral = red_bank.query_user_collateral(&mut mock_env, &provider, "uusdc");
    assert_eq!(provider_collateral.amount.u128(), funded_usdc);

    // borrower deposits atom
    red_bank.deposit(&mut mock_env, &borrower, coin(funded_atom, "uatom")).unwrap();
    let balance = mock_env.query_balance(&borrower, "uatom").unwrap();
    assert_eq!(balance.amount, Uint128::zero());
    let borrower_collateral = red_bank.query_user_collateral(&mut mock_env, &borrower, "uatom");
    assert_eq!(borrower_collateral.amount.u128(), funded_atom);

    // check current red-bank balance
    let usdc_balance = mock_env.query_balance(&red_bank.contract_addr, "uusdc").unwrap();
    assert_eq!(usdc_balance.amount.u128(), funded_usdc);
    let atom_balance = mock_env.query_balance(&red_bank.contract_addr, "uatom").unwrap();
    assert_eq!(atom_balance.amount.u128(), funded_atom);

    // check markets before borrowing full liquidity
    let usdc_market_before = red_bank.query_market(&mut mock_env, "uusdc");
    assert_eq!(usdc_market_before.collateral_total_scaled, provider_collateral.amount_scaled);
    assert_eq!(usdc_market_before.debt_total_scaled, Uint128::zero());
    let atom_market_before = red_bank.query_market(&mut mock_env, "uatom");
    assert_eq!(atom_market_before.collateral_total_scaled, borrower_collateral.amount_scaled);
    assert_eq!(atom_market_before.debt_total_scaled, Uint128::zero());

    // borrower borrows full liquidity
    red_bank.borrow(&mut mock_env, &borrower, "uusdc", funded_usdc).unwrap();
    let balance = mock_env.query_balance(&borrower, "uusdc").unwrap();
    assert_eq!(balance.amount.u128(), funded_usdc);
    let borrower_debt = red_bank.query_user_debt(&mut mock_env, &borrower, "uusdc");
    assert_eq!(borrower_debt.amount.u128(), funded_usdc);

    // check markets after borrowing full liquidity
    let atom_market = red_bank.query_market(&mut mock_env, "uatom");
    assert_eq!(atom_market.collateral_total_scaled, atom_market_before.collateral_total_scaled);
    assert_eq!(atom_market.debt_total_scaled, atom_market_before.debt_total_scaled);
    let usdc_market = red_bank.query_market(&mut mock_env, "uusdc");
    assert_eq!(usdc_market.collateral_total_scaled, usdc_market_before.collateral_total_scaled);
    assert_eq!(usdc_market.debt_total_scaled, borrower_debt.amount_scaled);

    // check current red-bank balance
    let usdc_balance = mock_env.query_balance(&red_bank.contract_addr, "uusdc").unwrap();
    assert_eq!(usdc_balance.amount, Uint128::zero());
    let atom_balance = mock_env.query_balance(&red_bank.contract_addr, "uatom").unwrap();
    assert_eq!(atom_balance.amount.u128(), funded_atom);

    // borrowing more should fail
    red_bank.borrow(&mut mock_env, &borrower, "uusdc", 1u128).unwrap_err();
}

#[test]
fn interest_rates_after_repayment() {
    // 1. Repay exact debt
    let (mut mock_env, red_bank, user_1) = prepare_debt_for_repayment();

    // few blocks passed, debt should increase for user_1
    let debt = red_bank.query_user_debt(&mut mock_env, &user_1, "uusdc");

    // repay full debt for user_1
    let repayed = debt.amount; // it should be 5_000_069 uusdc
    assert_eq!(repayed, Uint128::from(5_000_069u128));

    red_bank.repay(&mut mock_env, &user_1, coin(repayed.u128(), "uusdc")).unwrap();

    let debt = red_bank.query_user_debt(&mut mock_env, &user_1, "uusdc");
    assert_eq!(debt.amount, Uint128::zero());
    assert_eq!(debt.amount_scaled, Uint128::zero());

    let exact_debt_repayment_result = red_bank.query_market(&mut mock_env, "uusdc");
    assert_eq!(
        exact_debt_repayment_result.borrow_rate,
        Decimal::from_ratio(716667703657732987u128, 1000000000000000000u128)
    );
    assert_eq!(
        exact_debt_repayment_result.liquidity_rate,
        Decimal::from_ratio(344002281382926748u128, 1000000000000000000u128)
    );

    // 2. Repay full debt with refund
    let (mut mock_env, red_bank, user_1) = prepare_debt_for_repayment();

    // repay full debt for user_1 with a huge excess
    let repayed = 1_000_000_000u128;

    red_bank.repay(&mut mock_env, &user_1, coin(repayed, "uusdc")).unwrap();

    let debt = red_bank.query_user_debt(&mut mock_env, &user_1, "uusdc");
    assert_eq!(debt.amount, Uint128::zero());
    assert_eq!(debt.amount_scaled, Uint128::zero());

    // interest rates should be the same after repaying exact debt or with refund
    let result = red_bank.query_market(&mut mock_env, "uusdc");
    assert_eq!(result.borrow_rate, exact_debt_repayment_result.borrow_rate);
    assert_eq!(result.liquidity_rate, exact_debt_repayment_result.liquidity_rate);
}

fn prepare_debt_for_repayment() -> (MockEnv, RedBank, Addr) {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    // setup oracle and red-bank
    let oracle = mock_env.oracle.clone();
    oracle.set_price_source_fixed(&mut mock_env, "uatom", Decimal::from_ratio(12u128, 1u128));
    oracle.set_price_source_fixed(&mut mock_env, "uusdc", Decimal::one());
    let red_bank = mock_env.red_bank.clone();
    let params = mock_env.params.clone();
    let (market_params, asset_params) = default_asset_params();
    red_bank.init_asset(&mut mock_env, "uatom", market_params.clone());
    red_bank.init_asset(&mut mock_env, "uusdc", market_params);
    params.init_params(&mut mock_env, "uatom", asset_params.clone());
    params.init_params(&mut mock_env, "uusdc", asset_params);

    // fund user_1 account with atom
    let user_1 = Addr::unchecked("user_1");
    let funded_atom = 100_000_000u128;
    mock_env.fund_account(&user_1, &[coin(funded_atom, "uatom")]);

    // fund user_2 account with usdc
    let user_2 = Addr::unchecked("user_2");
    let funded_usdc = 200_000_000u128;
    mock_env.fund_account(&user_2, &[coin(funded_usdc, "uusdc")]);

    // user_1 deposits some atom
    let deposited_atom = 65_000_000u128;
    red_bank.deposit(&mut mock_env, &user_1, coin(deposited_atom, "uatom")).unwrap();

    // user_2 deposits all usdc balance
    let deposited_usdc = funded_usdc;
    red_bank.deposit(&mut mock_env, &user_2, coin(deposited_usdc, "uusdc")).unwrap();

    // move few blocks
    mock_env.increment_by_blocks(10);

    // user_1 borrows some usdc (no usdc in the account before)
    let borrowed_usdc = 5_000_000u128;
    red_bank.borrow(&mut mock_env, &user_1, "uusdc", borrowed_usdc).unwrap();

    // user_2 borrows some usdc
    let borrowed_usdc = 120_000_000u128;
    red_bank.borrow(&mut mock_env, &user_2, "uusdc", borrowed_usdc).unwrap();

    // move few blocks
    mock_env.increment_by_blocks(100);

    // add more usdc to user_1 account to repay full debt
    mock_env.fund_account(&user_1, &[coin(1_000_000_000u128, "uusdc")]);
    (mock_env, red_bank, user_1)
}
