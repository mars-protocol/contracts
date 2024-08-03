use cosmwasm_std::{coin, Addr, Decimal};
use mars_testing::integration::{
    helpers::{osmo_asset_params, usdc_asset_params},
    mock_env::MockEnvBuilder,
};
use mars_types::red_bank::UserHealthStatus;

#[test]
fn deposit_and_withdraw_for_credit_account_works() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner.clone()).build();

    let red_bank = mock_env.red_bank.clone();
    let params = mock_env.params.clone();
    let oracle = mock_env.oracle.clone();

    let funded_amt = 1_000_000_000_000u128;
    let provider = Addr::unchecked("provider"); // provides collateral to be borrowed by others
    let credit_manager = mock_env.credit_manager.clone();
    let account_id = "111".to_string();

    // setup red-bank
    let (market_params, asset_params) = osmo_asset_params();
    red_bank.init_asset(&mut mock_env, &asset_params.denom, market_params);
    params.init_params(&mut mock_env, asset_params);
    let (market_params, asset_params) = usdc_asset_params();
    red_bank.init_asset(&mut mock_env, &asset_params.denom, market_params);
    params.init_params(&mut mock_env, asset_params);

    // setup oracle
    oracle.set_price_source_fixed(&mut mock_env, "uosmo", Decimal::one());
    oracle.set_price_source_fixed(&mut mock_env, "uusdc", Decimal::from_ratio(2u128, 1u128));

    // fund accounts
    mock_env.fund_accounts(&[&provider, &credit_manager], funded_amt, &["uosmo", "uusdc"]);

    // provider deposits collaterals
    red_bank.deposit(&mut mock_env, &provider, coin(1000000000, "uusdc")).unwrap();

    // credit manager deposits
    let cm_osmo_deposit_amt = 100000000u128;
    red_bank
        .deposit_with_acc_id(
            &mut mock_env,
            &credit_manager,
            coin(cm_osmo_deposit_amt, "uosmo"),
            Some(account_id.clone()),
        )
        .unwrap();

    // credit manager should be able to borrow
    let cm_usdc_borrow_amt = 100000000u128;
    red_bank.borrow(&mut mock_env, &credit_manager, "uusdc", cm_usdc_borrow_amt).unwrap();

    // collateral is not tracked for credit manager (it is per account id). Debt is tracked for credit manager as a whole (not per account id)
    let cm_collaterals = red_bank.query_user_collaterals(&mut mock_env, &credit_manager);
    assert!(cm_collaterals.is_empty());
    let cm_debts = red_bank.query_user_debts(&mut mock_env, &credit_manager);
    assert_eq!(cm_debts.len(), 1);
    let cm_usdc_debt = cm_debts.get("uusdc").unwrap();
    assert!(cm_usdc_debt.uncollateralized);
    assert_eq!(cm_usdc_debt.amount.u128(), cm_usdc_borrow_amt);
    let cm_position = red_bank.query_user_position(&mut mock_env, &credit_manager);
    assert!(cm_position.total_enabled_collateral.is_zero());
    assert!(cm_position.total_collateralized_debt.is_zero());
    assert_eq!(cm_position.health_status, UserHealthStatus::NotBorrowing);

    // collateral is tracked for credit manager account id. Debt is not tracked per account id
    let cm_collaterals = red_bank.query_user_collaterals_with_acc_id(
        &mut mock_env,
        &credit_manager,
        Some(account_id.clone()),
    );
    assert_eq!(cm_collaterals.len(), 1);
    let cm_osmo_collateral = cm_collaterals.get("uosmo").unwrap();
    assert_eq!(cm_osmo_collateral.amount.u128(), cm_osmo_deposit_amt);
    let cm_position = red_bank.query_user_position_with_acc_id(
        &mut mock_env,
        &credit_manager,
        Some(account_id.clone()),
    );
    assert_eq!(cm_position.total_enabled_collateral.u128(), cm_osmo_deposit_amt);
    assert!(cm_position.total_collateralized_debt.is_zero());
    assert_eq!(cm_position.health_status, UserHealthStatus::NotBorrowing);

    // withdraw total collateral for account id
    red_bank
        .withdraw_with_acc_id(
            &mut mock_env,
            &credit_manager,
            "uosmo",
            None,
            Some(account_id.clone()),
            None,
        )
        .unwrap();

    // check collaterals and debts for credit manager account id after withdraw
    let cm_collaterals = red_bank.query_user_collaterals_with_acc_id(
        &mut mock_env,
        &credit_manager,
        Some(account_id.clone()),
    );
    assert!(cm_collaterals.is_empty());
    let cm_position =
        red_bank.query_user_position_with_acc_id(&mut mock_env, &credit_manager, Some(account_id));
    assert!(cm_position.total_enabled_collateral.is_zero());
    assert!(cm_position.total_collateralized_debt.is_zero());
    assert_eq!(cm_position.health_status, UserHealthStatus::NotBorrowing);
}
