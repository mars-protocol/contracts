use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use mars_red_bank::error::ContractError;
use mars_testing::integration::mock_env::MockEnvBuilder;
use mars_types::red_bank::UserHealthStatus;

use crate::helpers::{assert_red_bank_err, default_asset_params};

mod helpers;

#[test]
fn rover_flow() {
    let owner = Addr::unchecked("owner");
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

    let rover = Addr::unchecked("rover");

    // deposit more than credit line
    let rover_uusdc_limit = 1_000_000_000_000u128;
    let uusdc_deposited = rover_uusdc_limit + 100_000u128;
    mock_env.fund_account(&owner, &[coin(uusdc_deposited, "uusdc")]);
    red_bank.deposit(&mut mock_env, &owner, coin(uusdc_deposited, "uusdc")).unwrap();

    // set credit line for rover
    red_bank
        .update_uncollateralized_loan_limit(
            &mut mock_env,
            &owner,
            &rover,
            "uusdc",
            Uint128::from(rover_uusdc_limit),
        )
        .unwrap();

    // rover can't borrow above the credit line
    let res_err = red_bank.borrow(&mut mock_env, &rover, "uusdc", rover_uusdc_limit + 1u128);
    assert_red_bank_err(res_err, ContractError::BorrowAmountExceedsUncollateralizedLoanLimit {});

    // rover borrows the entire line of credit
    let balance = mock_env.query_balance(&rover, "uusdc").unwrap();
    assert_eq!(balance.amount.u128(), 0u128);
    red_bank.borrow(&mut mock_env, &rover, "uusdc", rover_uusdc_limit).unwrap();
    let balance = mock_env.query_balance(&rover, "uusdc").unwrap();
    assert_eq!(balance.amount.u128(), rover_uusdc_limit);
    let debt = red_bank.query_user_debt(&mut mock_env, &rover, "uusdc");
    assert!(debt.uncollateralized);
    assert_eq!(debt.amount.u128(), rover_uusdc_limit);

    // should be possible to update the credit line to less than current debt
    let half_rover_uusdc_limit = rover_uusdc_limit / 2u128;
    red_bank
        .update_uncollateralized_loan_limit(
            &mut mock_env,
            &owner,
            &rover,
            "uusdc",
            Uint128::from(half_rover_uusdc_limit),
        )
        .unwrap();

    // can't borrow above the credit line
    let res_err = red_bank.borrow(&mut mock_env, &rover, "uusdc", 1u128);
    assert_red_bank_err(res_err, ContractError::BorrowAmountExceedsUncollateralizedLoanLimit {});

    // rover should be healthy (NotBorrowing because uncollateralized debt is not included in HF calculation)
    let position = red_bank.query_user_position(&mut mock_env, &rover);
    assert_eq!(position.health_status, UserHealthStatus::NotBorrowing);

    // can't remove credit line for rover (rover has an outstanding debt)
    let res_err = red_bank.update_uncollateralized_loan_limit(
        &mut mock_env,
        &owner,
        &rover,
        "uusdc",
        Uint128::zero(),
    );
    assert_red_bank_err(res_err, ContractError::UserHasUncollateralizedDebt {});
    let debt = red_bank.query_user_debt(&mut mock_env, &rover, "uusdc");
    assert!(debt.uncollateralized);
    assert_eq!(debt.amount.u128(), rover_uusdc_limit);

    // rover deposits some atom
    let deposited_atom = 15_000_000_000u128;
    mock_env.fund_account(&rover, &[coin(deposited_atom, "uatom")]);
    red_bank.deposit(&mut mock_env, &rover, coin(deposited_atom, "uatom")).unwrap();
    let balance = mock_env.query_balance(&rover, "uatom").unwrap();
    assert_eq!(balance.amount.u128(), 0u128);
    let collateral = red_bank.query_user_collateral(&mut mock_env, &rover, "uatom");
    assert_eq!(collateral.amount.u128(), deposited_atom);

    // rover repay full debt
    red_bank.repay(&mut mock_env, &rover, coin(rover_uusdc_limit, "uusdc")).unwrap();
    let debt = red_bank.query_user_debt(&mut mock_env, &rover, "uusdc");
    assert!(!debt.uncollateralized);
    assert_eq!(debt.amount.u128(), 0u128);

    // remove credit line for rover
    red_bank
        .update_uncollateralized_loan_limit(&mut mock_env, &owner, &rover, "uusdc", Uint128::zero())
        .unwrap();

    // after debt repayment rover is able to borrow (using deposited collateral)
    red_bank.borrow(&mut mock_env, &rover, "uusdc", 1u128).unwrap();
    let debt = red_bank.query_user_debt(&mut mock_env, &rover, "uusdc");
    assert!(!debt.uncollateralized);
    assert_eq!(debt.amount.u128(), 1u128);

    // can't increase credit line for rover (rover has an outstanding debt - collateralized debt)
    let res_err = red_bank.update_uncollateralized_loan_limit(
        &mut mock_env,
        &owner,
        &rover,
        "uusdc",
        Uint128::from(rover_uusdc_limit),
    );
    assert_red_bank_err(res_err, ContractError::UserHasCollateralizedDebt {});
}
