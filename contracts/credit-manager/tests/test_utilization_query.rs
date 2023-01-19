use cosmwasm_std::{Addr, Decimal, Uint128};
use mars_rover::msg::execute::{
    Action::{Deposit, EnterVault},
    ActionAmount::Exact,
    ActionCoin,
};

use crate::helpers::{
    ujake_info, unlocked_vault_info, uosmo_info, AccountToFund, CoinInfo, MockEnv, VaultTestInfo,
};

pub mod helpers;

#[test]
fn test_utilization_is_zero() {
    let mock = MockEnv::new().vault_configs(&[unlocked_vault_info()]).build().unwrap();
    let vault_infos = mock.query_vault_configs(None, None);
    assert_eq!(1, vault_infos.len());
    let vault = vault_infos.first().unwrap();
    assert_eq!(Uint128::zero(), vault.utilization.amount);
    assert_eq!(vault.config.deposit_cap.denom, vault.utilization.denom);
}

#[test]
fn test_utilization_if_cap_is_base_denom() {
    let user = Addr::unchecked("user");
    let base_info = CoinInfo {
        denom: "base_denom".to_string(),
        price: Decimal::from_atomics(1u128, 0).unwrap(),
        max_ltv: Default::default(),
        liquidation_threshold: Default::default(),
        liquidation_bonus: Default::default(),
    };

    let leverage_vault = VaultTestInfo {
        vault_token_denom: "uleverage".to_string(),
        base_token_denom: base_info.denom.clone(),
        lockup: None,
        deposit_cap: base_info.to_coin(100),
        max_ltv: Default::default(),
        liquidation_threshold: Default::default(),
        whitelisted: true,
    };

    let mut mock = MockEnv::new()
        .allowed_coins(&[base_info.clone()])
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
                vault,
                coin: ActionCoin {
                    denom: base_info.denom.clone(),
                    amount: Exact(Uint128::new(50)),
                },
            },
        ],
        &[base_info.to_coin(50)],
    )
    .unwrap();

    let vault_infos = mock.query_vault_configs(None, None);
    let vault = vault_infos.first().unwrap();
    assert_eq!(vault.config.deposit_cap.denom, vault.utilization.denom);
    assert_eq!(Uint128::new(50), vault.utilization.amount);
}

/*
    Vault deposit cap: 100 uosmo
                       price: .25
    Current vault deposits: 1_000_000 uleverage (vault tokens) // 1_000_000 ujake underlying
                            price: 2.3654 (underlying / # vault tokens * price of underlying)
    Utilization denominated in uosmo = 1_000_000 * 2.3654 / .25 ---> 9461600
*/
#[test]
fn test_utilization_in_other_denom() {
    let osmo_info = uosmo_info();
    let jake_info = ujake_info();

    let leverage_vault = VaultTestInfo {
        vault_token_denom: "uleverage".to_string(),
        base_token_denom: jake_info.denom.clone(),
        lockup: None,
        deposit_cap: osmo_info.to_coin(50_000_000),
        max_ltv: Default::default(),
        liquidation_threshold: Default::default(),
        whitelisted: true,
    };

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[jake_info.clone(), osmo_info])
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
                vault,
                coin: ActionCoin {
                    denom: jake_info.denom.clone(),
                    amount: Exact(Uint128::new(1_000_000)),
                },
            },
        ],
        &[jake_info.to_coin(1_000_000)],
    )
    .unwrap();

    let vault_infos = mock.query_vault_configs(None, None);
    let vault = vault_infos.first().unwrap();
    assert_eq!(vault.config.deposit_cap.denom, vault.utilization.denom);
    assert_eq!(Uint128::new(9461600), vault.utilization.amount);
}
