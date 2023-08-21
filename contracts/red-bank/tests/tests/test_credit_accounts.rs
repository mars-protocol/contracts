use std::str::FromStr;

use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use mars_params::types::asset::{AssetParams, CmSettings, LiquidationBonus, RedBankSettings};
use mars_red_bank::error::ContractError;
use mars_red_bank_types::red_bank::{InitOrUpdateAssetParams, InterestRateModel, UserHealthStatus};
use mars_testing::integration::mock_env::MockEnvBuilder;

use super::helpers::assert_err;

#[test]
fn deposit_and_withdraw_for_credit_account_works() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner.clone()).build();

    let red_bank = mock_env.red_bank.clone();
    let params = mock_env.params.clone();
    let oracle = mock_env.oracle.clone();

    let funded_amt = 1_000_000_000_000u128;
    let provider = Addr::unchecked("provider"); // provides collateral to be borrowed by others
    let credit_manager = Addr::unchecked("credit_manager");
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

    // credit manager try to borrow if no credit line set
    let error_res = red_bank.borrow(&mut mock_env, &credit_manager, "uusdc", 100000000);
    assert_err(error_res, ContractError::BorrowAmountExceedsGivenCollateral {});

    // update credit line for credit manager
    red_bank
        .update_uncollateralized_loan_limit(
            &mut mock_env,
            &owner,
            &credit_manager,
            "uusdc",
            Uint128::MAX,
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

fn osmo_asset_params() -> (InitOrUpdateAssetParams, AssetParams) {
    default_asset_params_with("uosmo", Decimal::percent(70), Decimal::percent(78))
}

fn usdc_asset_params() -> (InitOrUpdateAssetParams, AssetParams) {
    default_asset_params_with("uusdc", Decimal::percent(90), Decimal::percent(96))
}

fn default_asset_params_with(
    denom: &str,
    max_loan_to_value: Decimal,
    liquidation_threshold: Decimal,
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
        liquidation_bonus: LiquidationBonus {
            starting_lb: Decimal::percent(1),
            slope: Decimal::from_str("2.0").unwrap(),
            min_lb: Decimal::percent(2),
            max_lb: Decimal::percent(10),
        },
        protocol_liquidation_fee: Decimal::percent(2),
        deposit_cap: Uint128::MAX,
    };
    (market_params, asset_params)
}
