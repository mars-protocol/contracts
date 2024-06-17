use cosmwasm_std::{coin, Addr, Decimal};
use mars_testing::integration::mock_env::MockEnvBuilder;
use mars_types::red_bank::UserHealthStatus;

use crate::helpers::default_asset_params;

mod helpers;

#[test]
fn rover_flow() {
    let owner = Addr::unchecked("owner");
    let acc_id = "12";
    let mut mock_env = MockEnvBuilder::new(None, owner.clone()).build();

    // setup oracle and red-bank
    let oracle = mock_env.oracle.clone();
    oracle.set_price_source_fixed(&mut mock_env, "uosmo", Decimal::one());
    oracle.set_price_source_fixed(&mut mock_env, "uusdc", Decimal::from_ratio(5u128, 10u128));
    oracle.set_price_source_fixed(&mut mock_env, "uatom", Decimal::from_ratio(12u128, 1u128));
    let red_bank = mock_env.red_bank.clone();
    let params = mock_env.params.clone();
    let (market_params, asset_params) = default_asset_params("uusdc");
    red_bank.init_asset(&mut mock_env, "uusdc", market_params);
    params.init_params(&mut mock_env, asset_params);
    let (market_params, asset_params) = default_asset_params("uosmo");
    red_bank.init_asset(&mut mock_env, "uosmo", market_params);
    params.init_params(&mut mock_env, asset_params);
    let (market_params, asset_params) = default_asset_params("uatom");
    red_bank.init_asset(&mut mock_env, "uatom", market_params);
    params.init_params(&mut mock_env, asset_params);

    let rover = mock_env.credit_manager.clone();

    // deposit more than borrowed
    let uusdc_borrowed = 1_000_000_000_000u128;
    let uusdc_deposited = uusdc_borrowed + 100_000u128;
    mock_env.fund_account(&rover, &[coin(uusdc_deposited, "uusdc")]);
    red_bank
        .deposit_with_acc_id(
            &mut mock_env,
            &rover.clone(),
            coin(uusdc_deposited, "uusdc"),
            Some(acc_id.to_string()),
        )
        .unwrap();

    // rover borrows more than deposited
    let balance = mock_env.query_balance(&rover, "uusdc").unwrap();
    assert_eq!(balance.amount.u128(), 0u128);
    red_bank
        .borrow_v2(&mut mock_env, &rover, Some(acc_id.to_string()), "uusdc", uusdc_borrowed)
        .unwrap();

    // fd
    let balance = mock_env.query_balance(&rover, "uusdc").unwrap();
    assert_eq!(balance.amount.u128(), uusdc_borrowed);
    // asdf
    let debt = red_bank.query_user_debt_v2(&mut mock_env, &rover, acc_id, "uusdc");
    assert!(debt.uncollateralized);
    assert_eq!(debt.amount.u128(), uusdc_borrowed);

    // rover should be healthy (NotBorrowing because uncollateralized debt is not included in HF calculation)
    let position = red_bank.query_user_position(&mut mock_env, &rover);
    assert_eq!(position.health_status, UserHealthStatus::NotBorrowing);

    // rover deposits some atom
    let deposited_atom = 15_000_000_000u128;
    mock_env.fund_account(&rover, &[coin(deposited_atom, "uatom")]);
    red_bank
        .deposit_with_acc_id(
            &mut mock_env,
            &rover,
            coin(deposited_atom, "uatom"),
            Some(acc_id.to_string()),
        )
        .unwrap();
    let balance = mock_env.query_balance(&rover, "uatom").unwrap();
    assert_eq!(balance.amount.u128(), 0u128);
    let collateral = red_bank.query_user_collateral_with_acc_id(
        &mut mock_env,
        &rover,
        Some(acc_id.to_string()),
        "uatom",
    );
    assert_eq!(collateral.amount.u128(), deposited_atom);

    // rover repay full debt
    red_bank
        .repay_v2(&mut mock_env, &rover, coin(uusdc_borrowed, "uusdc"), Some(acc_id.to_string()))
        .unwrap();
    let debt = red_bank.query_user_debt_v2(&mut mock_env, &rover, acc_id, "uusdc");
    assert!(!debt.uncollateralized);
    assert_eq!(debt.amount.u128(), 0u128);

    // after debt repayment rover is able to borrow (using deposited collateral)
    red_bank.borrow_v2(&mut mock_env, &rover, Some(acc_id.to_string()), "uusdc", 1u128).unwrap();
    let debt = red_bank.query_user_debt_v2(&mut mock_env, &rover, acc_id, "uusdc");
    assert!(debt.uncollateralized);
    assert_eq!(debt.amount.u128(), 1u128);
}
