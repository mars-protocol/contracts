use cosmwasm_std::{coin, Addr, Coin, Decimal, Uint128};
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
    red_bank
        .borrow_v2(
            &mut mock_env,
            &credit_manager,
            Some(account_id.to_string()),
            "uusdc",
            cm_usdc_borrow_amt,
        )
        .unwrap();

    // collateral is not tracked for credit manager (it is per account id). Debt is tracked for credit manager as a whole (not per account id)
    let cm_collaterals = red_bank.query_user_collaterals(&mut mock_env, &credit_manager);
    assert!(cm_collaterals.is_empty());
    let cm_debts = red_bank.query_user_debts_v2(&mut mock_env, &credit_manager, &account_id);
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

#[test]
fn borrow_and_repay_for_account_works() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner.clone()).build();

    let red_bank = mock_env.red_bank.clone();
    let params = mock_env.params.clone();
    let oracle = mock_env.oracle.clone();
    let credit_manager = mock_env.credit_manager.clone();

    let funded_amt = 1_000_000_000_000u128;
    let provider = Addr::unchecked("provider"); // provides collateral to be borrowed by others
    let user_1_account_id = "10".to_string();
    let user_2_account_id = "11".to_string();

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
    mock_env.fund_accounts(&[&provider], funded_amt, &["uosmo", "uusdc"]);

    // provider deposits collaterals
    red_bank.deposit(&mut mock_env, &provider, coin(1000000000, "uusdc")).unwrap();
    red_bank.deposit(&mut mock_env, &provider, coin(1000000000, "uosmo")).unwrap();

    // borrow funds for both users
    let user_1_borrow_amount = 100000000u128;
    let user_2_borrow_amount = 3000000u128;
    red_bank
        .borrow_v2(
            &mut mock_env,
            &credit_manager,
            Some(user_1_account_id.clone()),
            "uusdc",
            user_1_borrow_amount,
        )
        .unwrap();
    red_bank
        .borrow_v2(
            &mut mock_env,
            &credit_manager,
            Some(user_2_account_id.clone()),
            "uusdc",
            user_2_borrow_amount,
        )
        .unwrap();
    red_bank
        .borrow_v2(
            &mut mock_env,
            &credit_manager,
            Some(user_2_account_id.clone()),
            "uosmo",
            user_2_borrow_amount,
        )
        .unwrap();

    let user_1_debt =
        red_bank.query_user_debt_v2(&mut mock_env, &credit_manager, &user_1_account_id, "uusdc");
    let user_2_debt =
        red_bank.query_user_debt_v2(&mut mock_env, &credit_manager, &user_2_account_id, "uusdc");
    let cm_debt = red_bank.query_user_debt_v2(&mut mock_env, &credit_manager, "", "uusdc");

    // Only users accumulated debt, not credit manager
    assert_eq!(user_1_debt.amount, Uint128::from(user_1_borrow_amount));
    assert_eq!(user_2_debt.amount, Uint128::from(user_2_borrow_amount));
    assert_eq!(user_1_debt.denom, "uusdc");
    assert_eq!(user_2_debt.denom, "uusdc");
    assert_eq!(cm_debt.amount, Uint128::zero());
    assert_eq!(cm_debt.denom, "uusdc");

    // check if total debt is correct
    let market = red_bank.query_market_v2(&mut mock_env, "uusdc");
    assert_eq!(
        market.debt_total_amount,
        Uint128::from(user_1_borrow_amount + user_2_borrow_amount)
    );
    assert_eq!(market.market.denom, "uusdc");

    // repay position partially
    let user_1_repay_amount = 75000000u128;
    red_bank
        .repay_v2(
            &mut mock_env,
            &credit_manager,
            Coin {
                denom: "uusdc".to_string(),
                amount: Uint128::new(user_1_repay_amount),
            },
            Some(user_1_account_id.clone()),
        )
        .unwrap();
    let user_1_debt =
        red_bank.query_user_debt_v2(&mut mock_env, &credit_manager, &user_1_account_id, "uusdc");
    assert_eq!(user_1_debt.amount, Uint128::from(user_1_borrow_amount - user_1_repay_amount));
    assert_eq!(user_1_debt.denom, "uusdc");

    // check if total debt is correct
    let market = red_bank.query_market_v2(&mut mock_env, "uusdc");
    assert_eq!(
        market.debt_total_amount,
        Uint128::from(user_1_borrow_amount - user_1_repay_amount + user_2_borrow_amount)
    );
    assert_eq!(market.market.denom, "uusdc");

    // repay position fully
    let user_1_repay_amount = 25000000u128;
    red_bank
        .repay_v2(
            &mut mock_env,
            &credit_manager,
            Coin {
                denom: "uusdc".to_string(),
                amount: Uint128::new(user_1_repay_amount),
            },
            Some(user_1_account_id.clone()),
        )
        .unwrap();
    let user_1_debt =
        red_bank.query_user_debt_v2(&mut mock_env, &credit_manager, &user_1_account_id, "uusdc");
    assert_eq!(user_1_debt.amount, Uint128::zero());
    assert_eq!(user_1_debt.denom, "uusdc");

    // check if total debt is correct
    let market = red_bank.query_market_v2(&mut mock_env, "uusdc");
    assert_eq!(market.debt_total_amount, Uint128::from(user_2_borrow_amount));
    assert_eq!(market.market.denom, "uusdc");

    let user_1_debts =
        red_bank.query_user_debts_v2(&mut mock_env, &credit_manager, &user_1_account_id);
    assert!(user_1_debts.is_empty());

    let user_2_debts =
        red_bank.query_user_debts_v2(&mut mock_env, &credit_manager, &user_2_account_id);
    assert_eq!(user_2_debts.len(), 2);
    assert_eq!(user_2_debts.get("uusdc").unwrap().amount, Uint128::from(user_2_borrow_amount));
    assert_eq!(user_2_debts.get("uosmo").unwrap().amount, Uint128::from(user_2_borrow_amount));
}
