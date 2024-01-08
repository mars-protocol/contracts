use std::{cmp::min, str::FromStr};

use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use mars_red_bank::error::ContractError;
use mars_testing::integration::mock_env::MockEnvBuilder;
use mars_types::{
    params::{AssetParams, CmSettings, LiquidationBonus, RedBankSettings},
    red_bank::{InitOrUpdateAssetParams, InterestRateModel},
};

use crate::tests::helpers::assert_err;

#[test]
fn inflated_collateral() {
    let atom_price = Decimal::from_ratio(1133u128, 100u128); // 1 ATOM = 11.33 USD
    let osmo_price = Decimal::from_ratio(163u128, 100u128); // 1 OSMO = 1.63 USD

    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();
    let oracle = mock_env.oracle.clone();
    oracle.set_price_source_fixed(&mut mock_env, "uatom", atom_price);
    oracle.set_price_source_fixed(&mut mock_env, "uosmo", osmo_price);

    let red_bank = mock_env.red_bank.clone();
    let params = mock_env.params.clone();

    let (atom_market_params, atom_asset_params) = atom_asset_params("uatom");
    red_bank.init_asset(&mut mock_env, &atom_asset_params.denom, atom_market_params);
    params.init_params(&mut mock_env, atom_asset_params.clone());

    let (osmo_market_params, osmo_asset_params) = osmo_asset_params("uosmo");
    red_bank.init_asset(&mut mock_env, &osmo_asset_params.denom, osmo_market_params);
    params.init_params(&mut mock_env, osmo_asset_params.clone());

    let borrower = Addr::unchecked("borrower");
    let borrower2 = Addr::unchecked("borrower2");
    let other_depositor = Addr::unchecked("otherdepositor");

    let funded_atom = 1024000000;
    let donated_atom = 450 * funded_atom;
    // max_atom_borrow_amt = osmo_collateral * osmo_price * osmo_ltv / atom_price
    // osmo_collateral = max_atom_borrow_amt * atom_price / (osmo_price * osmo_ltv)
    let osmo_collateral = Uint128::new(donated_atom)
        .checked_mul_floor(atom_price)
        .unwrap()
        .checked_div_floor(osmo_price)
        .unwrap()
        .checked_div_floor(osmo_asset_params.max_loan_to_value)
        .unwrap()
        .u128();
    let osmo_other_deposit = 3_059_483_963_978_u128; // OSMO deposit from someone else (OSMO funds already deposited in the market)

    // Deposit ATOM collateral
    mock_env.fund_account(&borrower, &[coin(funded_atom, "uatom")]);
    red_bank.deposit(&mut mock_env, &borrower, coin(funded_atom, "uatom")).unwrap();

    // Donate ATOM to the protocol (which will be borrowed later) to inflate the collateral
    mock_env.fund_account(&red_bank.contract_addr, &[coin(donated_atom, "uatom")]);

    // From another account, deposit max allowed OSMO to fill the caps
    let left_osmo_deposit = osmo_asset_params.deposit_cap - Uint128::new(osmo_other_deposit);
    let osmo_collateral = min(osmo_collateral, left_osmo_deposit.u128());
    mock_env.fund_account(&borrower2, &[coin(osmo_collateral, "uosmo")]);
    red_bank.deposit(&mut mock_env, &borrower2, coin(osmo_collateral, "uosmo")).unwrap();
    assert_eq!(osmo_collateral, 4_387_649_382_300_u128);

    // Fund OSMO market (representing OSMO funds from other depositors)
    mock_env.fund_account(&other_depositor, &[coin(osmo_other_deposit, "uosmo")]);
    red_bank.deposit(&mut mock_env, &other_depositor, coin(osmo_other_deposit, "uosmo")).unwrap();

    // Borrow available liquidity
    red_bank.borrow(&mut mock_env, &borrower2, "uatom", funded_atom).unwrap();
    let error_res = red_bank.borrow(&mut mock_env, &borrower2, "uatom", funded_atom);
    assert_err(
        error_res,
        ContractError::InvalidBorrowAmount {
            denom: "uatom".to_string(),
        },
    );

    // borrower2 received only part of the initially donated ATOM
    let atom_balance_borrower2 = mock_env.query_balance(&borrower2, "uatom").unwrap();
    assert_eq!(atom_balance_borrower2.amount.u128(), funded_atom);
    let left_atom_donated = Uint128::new(donated_atom) - atom_balance_borrower2.amount;

    // Validate that borrower2 has no OSMO left in balance
    let uosmo_balance_borrower2 = mock_env.query_balance(&borrower2, "uosmo").unwrap();
    assert_eq!(uosmo_balance_borrower2.amount.u128(), 0);

    // Wait 4 hours
    let seconds_to_wait: u64 = 60 * 60 * 4;
    mock_env.app.update_block(|b| b.time = b.time.plus_seconds(seconds_to_wait));

    // Validate ATOM market borrow and liquidity rates aren't hyper inflated
    let market = red_bank.query_market(&mut mock_env, "uatom");
    assert_eq!(market.borrow_rate, Decimal::percent(320));
    assert_eq!(market.liquidity_rate, Decimal::percent(288));

    let new_user_res = red_bank.query_user_collateral(&mut mock_env, &borrower, "uatom");

    let uosmo_borrow_amount = new_user_res
        .amount
        .checked_mul_floor(atom_price)
        .unwrap()
        .checked_mul_floor(atom_asset_params.max_loan_to_value)
        .unwrap()
        .checked_div_floor(osmo_price)
        .unwrap();

    // Borrows OSMO against ATOM collateral
    let uosmo_borrow_amount =
        min(uosmo_borrow_amount, Uint128::new(osmo_collateral + osmo_other_deposit)); // Borrow max allowed OSMO based on available liquidity
    red_bank.borrow(&mut mock_env, &borrower, "uosmo", uosmo_borrow_amount.u128()).unwrap();

    // Validate borrower has received borrowed OSMO
    let uosmo_balance_borrower = mock_env.query_balance(&borrower, "uosmo").unwrap();
    assert_eq!(uosmo_balance_borrower.amount.u128(), uosmo_borrow_amount.u128());

    // Validate borrower has OSMO debt
    let uosmo_debt_borrower = red_bank.query_user_debt(&mut mock_env, &borrower, "uosmo");
    assert_eq!(uosmo_debt_borrower.amount.u128(), uosmo_borrow_amount.u128());

    // Calculate profit
    let usd_funded_osmo =
        Uint128::new(osmo_collateral).checked_mul_floor(osmo_price).unwrap().u128() as i128;
    let usd_uosmo_borrow_amount =
        uosmo_borrow_amount.checked_mul_floor(osmo_price).unwrap().u128() as i128;
    let usd_left_atom_donated =
        left_atom_donated.checked_mul_floor(atom_price).unwrap().u128() as i128;

    let usd_profit = usd_uosmo_borrow_amount - usd_funded_osmo - usd_left_atom_donated;
    assert!(usd_profit.checked_div(1_000_000).unwrap() < 0);
}

fn atom_asset_params(denom: &str) -> (InitOrUpdateAssetParams, AssetParams) {
    let market_params = InitOrUpdateAssetParams {
        reserve_factor: Some(Decimal::percent(10)),
        interest_rate_model: Some(InterestRateModel {
            optimal_utilization_rate: Decimal::percent(80),
            base: Decimal::percent(0),
            slope_1: Decimal::percent(20),
            slope_2: Decimal::percent(300),
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
        max_loan_to_value: Decimal::percent(74),
        liquidation_threshold: Decimal::percent(75),
        liquidation_bonus: LiquidationBonus {
            starting_lb: Decimal::zero(),
            slope: Decimal::from_str("1").unwrap(),
            min_lb: Decimal::percent(5),
            max_lb: Decimal::percent(20),
        },
        protocol_liquidation_fee: Decimal::percent(25),
        deposit_cap: Uint128::from(700000000000u128),
    };
    (market_params, asset_params)
}

fn osmo_asset_params(denom: &str) -> (InitOrUpdateAssetParams, AssetParams) {
    let market_params = InitOrUpdateAssetParams {
        reserve_factor: Some(Decimal::percent(10)),
        interest_rate_model: Some(InterestRateModel {
            optimal_utilization_rate: Decimal::percent(60),
            base: Decimal::percent(0),
            slope_1: Decimal::percent(15),
            slope_2: Decimal::percent(300),
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
        max_loan_to_value: Decimal::percent(73),
        liquidation_threshold: Decimal::percent(75),
        liquidation_bonus: LiquidationBonus {
            starting_lb: Decimal::zero(),
            slope: Decimal::from_str("1").unwrap(),
            min_lb: Decimal::percent(5),
            max_lb: Decimal::percent(20),
        },
        protocol_liquidation_fee: Decimal::percent(25),
        deposit_cap: Uint128::from(10000000000000u128),
    };
    (market_params, asset_params)
}
