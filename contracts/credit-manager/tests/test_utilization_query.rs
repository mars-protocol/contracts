use std::str::FromStr;

use cosmwasm_std::{Addr, Decimal, Uint128};
use mars_params::types::asset::LiquidationBonus;
use mars_rover::{
    adapters::vault::VaultUnchecked,
    msg::execute::{
        Action::{Deposit, EnterVault},
        ActionAmount::Exact,
        ActionCoin,
    },
};

use crate::helpers::{
    ujake_info, unlocked_vault_info, uosmo_info, AccountToFund, CoinInfo, MockEnv, VaultTestInfo,
};

pub mod helpers;

#[test]
fn raises_if_vault_not_found() {
    let mock = MockEnv::new().build().unwrap();
    mock.query_vault_utilization(&VaultUnchecked::new("some_vault".to_string())).unwrap_err();
}

#[test]
fn utilization_is_zero() {
    let leverage_vault = unlocked_vault_info();
    let mock = MockEnv::new().vault_configs(&[leverage_vault.clone()]).build().unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let res = mock.query_vault_utilization(&vault).unwrap();

    assert_eq!(Uint128::zero(), res.utilization.amount);
    assert_eq!(leverage_vault.deposit_cap.denom, res.utilization.denom);
}

#[test]
fn utilization_if_cap_is_base_denom() {
    let user = Addr::unchecked("user");
    let base_info = CoinInfo {
        denom: "base_denom".to_string(),
        price: Decimal::from_str("1").unwrap(),
        max_ltv: Decimal::from_str("0.6").unwrap(),
        liquidation_threshold: Decimal::from_str("0.7").unwrap(),
        liquidation_bonus: LiquidationBonus {
            starting_lb: Decimal::percent(1u64),
            slope: Decimal::from_atomics(2u128, 0).unwrap(),
            min_lb: Decimal::percent(2u64),
            max_lb: Decimal::percent(10u64),
        },
        protocol_liquidation_fee: Decimal::percent(2u64),
        whitelisted: true,
        hls: None,
    };

    let leverage_vault = VaultTestInfo {
        vault_token_denom: "uleverage".to_string(),
        base_token_denom: base_info.denom.clone(),
        lockup: None,
        deposit_cap: base_info.to_coin(100),
        max_ltv: Decimal::from_str("0.6").unwrap(),
        liquidation_threshold: Decimal::from_str("0.7").unwrap(),
        whitelisted: true,
        hls: None,
    };

    let mut mock = MockEnv::new()
        .set_params(&[base_info.clone()])
        .vault_configs(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![base_info.to_coin(50)],
        })
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let account_id = mock.create_credit_account(&user).unwrap();

    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(base_info.to_coin(50)),
            EnterVault {
                vault: vault.clone(),
                coin: ActionCoin {
                    denom: base_info.denom.clone(),
                    amount: Exact(Uint128::new(50)),
                },
            },
        ],
        &[base_info.to_coin(50)],
    )
    .unwrap();

    let res = mock.query_vault_utilization(&vault).unwrap();
    assert_eq!(leverage_vault.deposit_cap.denom, res.utilization.denom);
    assert_eq!(Uint128::new(50), res.utilization.amount);
}

/*
    Vault deposit cap: 100 uosmo
                       price: .25
    Current vault deposits: 1_000_000 uleverage (vault tokens) // 1_000_000 ujake underlying
                            price: 2.3654 (underlying / # vault tokens * price of underlying)
    Utilization denominated in uosmo = 1_000_000 * 2.3654 / .25 ---> 9461600
*/
#[test]
fn utilization_in_other_denom() {
    let osmo_info = uosmo_info();
    let jake_info = ujake_info();

    let leverage_vault = VaultTestInfo {
        vault_token_denom: "uleverage".to_string(),
        base_token_denom: jake_info.denom.clone(),
        lockup: None,
        deposit_cap: osmo_info.to_coin(50_000_000),
        max_ltv: Decimal::from_str("0.6").unwrap(),
        liquidation_threshold: Decimal::from_str("0.7").unwrap(),
        whitelisted: true,
        hls: None,
    };

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[jake_info.clone(), osmo_info])
        .vault_configs(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![jake_info.to_coin(1_000_000)],
        })
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let account_id = mock.create_credit_account(&user).unwrap();

    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(jake_info.to_coin(1_000_000)),
            EnterVault {
                vault: vault.clone(),
                coin: ActionCoin {
                    denom: jake_info.denom.clone(),
                    amount: Exact(Uint128::new(1_000_000)),
                },
            },
        ],
        &[jake_info.to_coin(1_000_000)],
    )
    .unwrap();

    let res = mock.query_vault_utilization(&vault).unwrap();
    assert_eq!(leverage_vault.deposit_cap.denom, res.utilization.denom);
    assert_eq!(Uint128::new(9461600), res.utilization.amount);
}
