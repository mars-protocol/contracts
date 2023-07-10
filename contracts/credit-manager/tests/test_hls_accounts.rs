use cosmwasm_std::{coins, Addr, Decimal, Uint128};
use mars_params::{msg::AssetParamsUpdate::AddOrUpdate, types::hls::HlsAssetType};
use mars_rover::{
    error::ContractError,
    msg::execute::Action::{Borrow, Deposit, EnterVault, Lend},
};
use mars_rover_health_types::{AccountKind, HealthValuesResponse};

use crate::helpers::{
    assert_err, lp_token_info, uatom_info, ujake_info, unlocked_vault_info, AccountToFund, MockEnv,
};

pub mod helpers;

#[test]
fn queries_return_the_expected_kind() {
    let mut mock = MockEnv::new().build().unwrap();
    let user = Addr::unchecked("user");

    let account_id = mock.create_hls_account(&user);
    let kind = mock.query_account_kind(&account_id);
    assert_eq!(AccountKind::HighLeveredStrategy, kind);

    let account_id = mock.create_credit_account(&user).unwrap();
    let kind = mock.query_account_kind(&account_id);
    assert_eq!(AccountKind::Default, kind);
}

#[test]
fn more_than_one_debt_does_not_qualify() {
    let atom_info = uatom_info();
    let jake_info = ujake_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[atom_info.clone(), jake_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, atom_info.denom.clone()),
        })
        .build()
        .unwrap();

    let account_id = mock.create_hls_account(&user);

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(atom_info.to_coin(300)),
            Borrow(atom_info.to_coin(10)),
            Borrow(jake_info.to_coin(1)),
        ],
        &[atom_info.to_coin(300)],
    );

    assert_err(
        res,
        ContractError::HLS {
            reason: "Account has more than one debt denom".to_string(),
        },
    )
}

#[test]
fn hls_allows_zero_debts_is_ok() {
    let atom_info = uatom_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[atom_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, atom_info.denom.clone()),
        })
        .build()
        .unwrap();

    let account_id = mock.create_hls_account(&user);

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Deposit(atom_info.to_coin(300))],
        &[atom_info.to_coin(300)],
    )
    .unwrap();

    // No error raised
}

#[test]
fn debt_denom_is_not_an_hls_asset() {
    let mut atom_info = uatom_info();
    atom_info.hls = None;

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[atom_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, atom_info.denom.clone()),
        })
        .build()
        .unwrap();

    let account_id = mock.create_hls_account(&user);

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![Deposit(atom_info.to_coin(300)), Borrow(atom_info.to_coin(10))],
        &[atom_info.to_coin(300)],
    );

    assert_err(
        res,
        ContractError::HLS {
            reason: format!("{} does not have HLS parameters", atom_info.denom),
        },
    )
}

#[test]
fn wrong_correlations_does_not_qualify() {
    let atom_info = uatom_info();
    let jake_info = ujake_info();
    let lp_token = lp_token_info();
    let leverage_vault = unlocked_vault_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[atom_info.clone(), jake_info.clone(), lp_token.clone()])
        .vault_configs(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![jake_info.to_coin(300), atom_info.to_coin(300), lp_token.to_coin(300)],
        })
        .build()
        .unwrap();

    let account_id = mock.create_hls_account(&user);

    // Case #1 - Collateral asset is not in correlations list

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![Deposit(jake_info.to_coin(300)), Borrow(atom_info.to_coin(1))],
        &[jake_info.to_coin(300)],
    );

    assert_err(
        res,
        ContractError::HLS {
            reason: format!(
                "{} deposit is not a correlated asset to debt {}",
                jake_info.denom, atom_info.denom
            ),
        },
    );

    // Case #2 - Some collateral assets are not in correlations list

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(jake_info.to_coin(300)),
            Deposit(atom_info.to_coin(300)),
            Borrow(atom_info.to_coin(1)),
        ],
        &[jake_info.to_coin(300), atom_info.to_coin(300)],
    );

    assert_err(
        res,
        ContractError::HLS {
            reason: format!(
                "{} deposit is not a correlated asset to debt {}",
                jake_info.denom, atom_info.denom
            ),
        },
    );

    // Case #3 - Lend asset types are checked

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Lend(jake_info.to_coin(50)),
            Deposit(jake_info.to_coin(50)),
            Deposit(atom_info.to_coin(300)),
            Borrow(atom_info.to_coin(1)),
        ],
        &[atom_info.to_coin(300), jake_info.to_coin(50)],
    );

    assert_err(
        res,
        ContractError::HLS {
            reason: format!(
                "{} lend is not a correlated asset to debt {}",
                jake_info.denom, atom_info.denom
            ),
        },
    );

    // Case #4 - Vault asset types are checked

    let vault = mock.get_vault(&leverage_vault);
    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(lp_token.to_coin(300)),
            EnterVault {
                vault: vault.clone(),
                coin: lp_token.to_action_coin(23),
            },
            Borrow(atom_info.to_coin(1)),
        ],
        &[lp_token.to_coin(300)],
    );

    assert_err(
        res,
        ContractError::HLS {
            reason: format!(
                "{} vault is not a correlated asset to debt {}",
                vault.address, atom_info.denom
            ),
        },
    );
}

#[test]
fn successful_with_asset_correlations() {
    let atom_info = uatom_info();
    let lp_token = lp_token_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[atom_info.clone(), lp_token.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![lp_token.to_coin(300)],
        })
        .build()
        .unwrap();

    let account_id = mock.create_hls_account(&user);

    let lp_deposit_amount = 300;
    let atom_borrow_amount = 150;

    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(lp_token.to_coin(lp_deposit_amount)),
            Borrow(atom_info.to_coin(atom_borrow_amount)),
        ],
        &[lp_token.to_coin(lp_deposit_amount)],
    )
    .unwrap();

    let hls_health = mock.query_health(&account_id, AccountKind::HighLeveredStrategy);
    let total_debt_value = atom_info.price * Uint128::new(atom_borrow_amount) + Uint128::one();
    let lp_collateral_value = lp_token.price * Uint128::new(lp_deposit_amount);
    let atom_collateral_value = atom_info.price * Uint128::new(atom_borrow_amount);
    let lp_hls_max_ltv = lp_collateral_value * lp_token.hls.as_ref().unwrap().max_loan_to_value;
    let atom_hls_max_ltv =
        atom_collateral_value * atom_info.hls.as_ref().unwrap().max_loan_to_value;
    let lp_hls_liq = lp_collateral_value * lp_token.hls.unwrap().liquidation_threshold;
    let atom_hls_liq = atom_collateral_value * atom_info.hls.unwrap().liquidation_threshold;

    assert_eq!(
        HealthValuesResponse {
            total_debt_value,
            total_collateral_value: lp_collateral_value + atom_collateral_value,
            max_ltv_adjusted_collateral: lp_hls_max_ltv + atom_hls_max_ltv,
            liquidation_threshold_adjusted_collateral: lp_hls_liq + atom_hls_liq,
            max_ltv_health_factor: Some(
                Decimal::checked_from_ratio(lp_hls_max_ltv + atom_hls_max_ltv, total_debt_value)
                    .unwrap()
            ),
            liquidation_health_factor: Some(
                Decimal::checked_from_ratio(lp_hls_liq + atom_hls_liq, total_debt_value).unwrap()
            ),
            liquidatable: false,
            above_max_ltv: false,
        },
        hls_health
    );

    let default_health = mock.query_health(&account_id, AccountKind::Default);
    assert_ne!(hls_health, default_health);
}

#[test]
fn successful_with_vault_correlations() {
    let atom_info = uatom_info();
    let lp_token = lp_token_info();
    let leverage_vault = unlocked_vault_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[atom_info.clone(), lp_token.clone()])
        .vault_configs(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![lp_token.to_coin(300)],
        })
        .build()
        .unwrap();

    // Add vault to correlations of Atom in params contract
    let vault = mock.get_vault(&leverage_vault);
    let mut asset_params = mock.query_asset_params(&atom_info.denom);
    asset_params.credit_manager.hls.as_mut().unwrap().correlations.push(HlsAssetType::Vault {
        addr: Addr::unchecked(vault.address),
    });
    mock.update_asset_params(AddOrUpdate {
        params: asset_params.into(),
    });

    let account_id = mock.create_hls_account(&user);

    let lp_deposit_amount = 300;
    let atom_borrow_amount = 150;

    let vault = mock.get_vault(&leverage_vault);
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(lp_token.to_coin(lp_deposit_amount)),
            EnterVault {
                vault,
                coin: lp_token.to_action_coin(lp_deposit_amount),
            },
            Borrow(atom_info.to_coin(atom_borrow_amount)),
        ],
        &[lp_token.to_coin(lp_deposit_amount)],
    )
    .unwrap();

    let hls_health = mock.query_health(&account_id, AccountKind::HighLeveredStrategy);
    let total_debt_value = atom_info.price * Uint128::new(atom_borrow_amount) + Uint128::one();
    let lp_collateral_value = lp_token.price * Uint128::new(lp_deposit_amount);
    let atom_collateral_value = atom_info.price * Uint128::new(atom_borrow_amount);
    let lp_hls_max_ltv = lp_collateral_value * lp_token.hls.as_ref().unwrap().max_loan_to_value;
    let atom_hls_max_ltv =
        atom_collateral_value * atom_info.hls.as_ref().unwrap().max_loan_to_value;
    let lp_hls_liq = lp_collateral_value * lp_token.hls.unwrap().liquidation_threshold;
    let atom_hls_liq = atom_collateral_value * atom_info.hls.unwrap().liquidation_threshold;

    assert_eq!(
        HealthValuesResponse {
            total_debt_value,
            total_collateral_value: lp_collateral_value + atom_collateral_value,
            max_ltv_adjusted_collateral: lp_hls_max_ltv + atom_hls_max_ltv,
            liquidation_threshold_adjusted_collateral: lp_hls_liq + atom_hls_liq,
            max_ltv_health_factor: Some(
                Decimal::checked_from_ratio(lp_hls_max_ltv + atom_hls_max_ltv, total_debt_value)
                    .unwrap()
            ),
            liquidation_health_factor: Some(
                Decimal::checked_from_ratio(lp_hls_liq + atom_hls_liq, total_debt_value).unwrap()
            ),
            liquidatable: false,
            above_max_ltv: false,
        },
        hls_health
    );

    let default_health = mock.query_health(&account_id, AccountKind::Default);
    assert_ne!(hls_health, default_health);
}
