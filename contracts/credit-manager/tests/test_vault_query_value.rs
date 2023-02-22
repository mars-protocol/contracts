use std::ops::Div;

use cosmwasm_std::{Addr, Uint128};
use mars_math::FractionMath;
use mars_mock_vault::contract::STARTING_VAULT_SHARES;
use mars_rover::{
    adapters::vault::{Vault, VaultAmount, VaultPosition, VaultPositionAmount},
    msg::execute::Action::{Deposit, EnterVault, RequestVaultUnlock},
};

use crate::helpers::{
    locked_vault_info, lp_token_info, unlocked_vault_info, AccountToFund, MockEnv,
};

pub mod helpers;

#[test]
fn raises_if_vault_not_available_to_price() {
    let mock = MockEnv::new().build().unwrap();

    let vault_position = VaultPosition {
        vault: Vault::new(Addr::unchecked("xyz")),
        amount: VaultPositionAmount::Unlocked(VaultAmount::new(Uint128::new(213))),
    };

    mock.query_vault_position_value(&vault_position).unwrap_err();
}

#[test]
fn returns_zero_if_vault_empty() {
    let lp_token = lp_token_info();
    let leverage_vault = unlocked_vault_info();

    let mock = MockEnv::new()
        .allowed_coins(&[lp_token])
        .vault_configs(&[leverage_vault.clone()])
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let vault_position = VaultPosition {
        vault: Vault::new(Addr::unchecked(vault.address)),
        amount: VaultPositionAmount::Unlocked(VaultAmount::new(Uint128::new(213))),
    };

    let value = mock.query_vault_position_value(&vault_position).unwrap();
    assert_eq!(value.vault_coin.value, Uint128::zero());
    assert_eq!(value.base_coin.value, Uint128::zero());
}

#[test]
fn accurately_prices() {
    let lp_token = lp_token_info();
    let leverage_vault = locked_vault_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone()])
        .vault_configs(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![lp_token.to_coin(300)],
        })
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let account_id = mock.create_credit_account(&user).unwrap();

    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(lp_token.to_coin(200)),
            EnterVault {
                vault: vault.clone(),
                coin: lp_token.to_action_coin(50),
            },
        ],
        &[lp_token.to_coin(200)],
    )
    .unwrap();

    let positions = mock.query_positions(&account_id);
    let vault_position = positions.vaults.first().unwrap();
    let value = mock.query_vault_position_value(vault_position).unwrap();
    assert_eq!(value.vault_coin.value, Uint128::new(50).checked_mul_floor(lp_token.price).unwrap());
    assert_eq!(value.base_coin.value, Uint128::zero());

    // Check case with unlocking positions
    mock.update_credit_account(
        &account_id,
        &user,
        vec![RequestVaultUnlock {
            vault,
            amount: STARTING_VAULT_SHARES.div(Uint128::new(2)),
        }],
        &[],
    )
    .unwrap();

    let positions = mock.query_positions(&account_id);
    let vault_position = positions.vaults.first().unwrap();
    let value = mock.query_vault_position_value(vault_position).unwrap();
    assert_eq!(value.vault_coin.value, Uint128::new(25).checked_mul_floor(lp_token.price).unwrap());
    assert_eq!(value.base_coin.value, Uint128::new(25).checked_mul_floor(lp_token.price).unwrap());
}
