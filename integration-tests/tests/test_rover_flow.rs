use crate::helpers::{assert_err, default_asset_params};
use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use mars_outpost::red_bank::UserHealthStatus;
use mars_red_bank::error::ContractError;
use mars_testing::integration::mock_env::MockEnvBuilder;

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
    red_bank.init_asset(&mut mock_env, "uosmo", default_asset_params());
    red_bank.init_asset(&mut mock_env, "uusdc", default_asset_params());
    red_bank.init_asset(&mut mock_env, "uatom", default_asset_params());

    let rover = Addr::unchecked("rover");

    // fund red-bank and set credit line for rover
    let rover_uusdc_limit = 1_000_000_000_000u128;
    mock_env.fund_account(&red_bank.contract_addr, &[coin(rover_uusdc_limit, "uusdc")]);
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
    assert_err(res_err, ContractError::BorrowAmountExceedsUncollateralizedLoanLimit {});

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
    assert_err(res_err, ContractError::BorrowAmountExceedsUncollateralizedLoanLimit {});

    // rover should be healthy (NotBorrowing because uncollateralized debt is not included in HF calculation)
    let position = red_bank.query_user_position(&mut mock_env, &rover);
    assert_eq!(position.health_status, UserHealthStatus::NotBorrowing);

    // remove credit line for rover
    red_bank
        .update_uncollateralized_loan_limit(&mut mock_env, &owner, &rover, "uusdc", Uint128::zero())
        .unwrap();
    let debt = red_bank.query_user_debt(&mut mock_env, &rover, "uusdc");
    assert!(!debt.uncollateralized);
    assert_eq!(debt.amount.u128(), rover_uusdc_limit);

    // rover should be unhealthy (max_ltv_hf < 1, liq_threshold_hf < 1), suddenly have an "collateralized" debt (debt.uncollateralized = false)
    let position = red_bank.query_user_position(&mut mock_env, &rover);
    assert_eq!(
        position.health_status,
        UserHealthStatus::Borrowing {
            max_ltv_hf: Decimal::zero(),
            liq_threshold_hf: Decimal::zero()
        }
    );

    // can't increase credit line for rover (rover has an outstanding debt)
    let res_err = red_bank.update_uncollateralized_loan_limit(
        &mut mock_env,
        &owner,
        &rover,
        "uusdc",
        Uint128::from(10_000_000u128),
    );
    assert_err(res_err, ContractError::UserHasCollateralizedDebt {});

    // rover deposits some atom
    let deposited_atom = 15_000_000_000u128;
    mock_env.fund_account(&rover, &[coin(deposited_atom, "uatom")]);
    red_bank.deposit(&mut mock_env, &rover, coin(deposited_atom, "uatom")).unwrap();
    let balance = mock_env.query_balance(&rover, "uatom").unwrap();
    assert_eq!(balance.amount.u128(), 0u128);
    let collateral = red_bank.query_user_collateral(&mut mock_env, &rover, "uatom");
    assert_eq!(collateral.amount.u128(), deposited_atom);

    // rover should be unhealthy (max_ltv_hf < 1, liq_threshold_hf < 1)
    let position = red_bank.query_user_position(&mut mock_env, &rover);
    assert_eq!(
        position.health_status,
        UserHealthStatus::Borrowing {
            max_ltv_hf: Decimal::from_ratio(216u128, 1000u128),
            liq_threshold_hf: Decimal::from_ratio(288u128, 1000u128)
        }
    );

    // can't borrow because position is unhealthy
    let res_err = red_bank.borrow(&mut mock_env, &rover, "uusdc", 1u128);
    assert_err(res_err, ContractError::BorrowAmountExceedsGivenCollateral {});

    // rover repay full debt
    red_bank.repay(&mut mock_env, &rover, coin(rover_uusdc_limit, "uusdc")).unwrap();
    let debt = red_bank.query_user_debt(&mut mock_env, &rover, "uusdc");
    assert!(!debt.uncollateralized);
    assert_eq!(debt.amount.u128(), 0u128);

    // after debt repayment rover is able to borrow (using deposited collateral)
    red_bank.borrow(&mut mock_env, &rover, "uusdc", 1u128).unwrap();

    // can't increase credit line for rover (rover has an outstanding debt - collateralized debt)
    let res_err = red_bank.update_uncollateralized_loan_limit(
        &mut mock_env,
        &owner,
        &rover,
        "uusdc",
        Uint128::from(10_000_000u128),
    );
    assert_err(res_err, ContractError::UserHasCollateralizedDebt {});
}
