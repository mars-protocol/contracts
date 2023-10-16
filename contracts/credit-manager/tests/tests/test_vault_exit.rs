use cosmwasm_std::{coin, Addr, Coin, OverflowError, OverflowOperation::Sub, Uint128};
use mars_credit_manager::error::ContractError::NotTokenOwner;
use mars_mock_vault::contract::STARTING_VAULT_SHARES;
use mars_types::{
    adapters::vault::{VaultBase, VaultError},
    credit_manager::Action::{Deposit, EnterVault, ExitVault},
    params::VaultConfigUpdate,
};

use super::helpers::{
    assert_err, locked_vault_info, lp_token_info, uatom_info, unlocked_vault_info, uosmo_info,
    AccountToFund, MockEnv,
};

#[test]
fn only_owner_of_token_can_withdraw_from_vault() {
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().build().unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.update_credit_account(
        &account_id,
        &bad_guy,
        vec![ExitVault {
            vault: VaultBase::new("some_vault".to_string()),
            amount: STARTING_VAULT_SHARES,
        }],
        &[],
    );

    assert_err(
        res,
        NotTokenOwner {
            user: bad_guy.into(),
            account_id,
        },
    )
}

#[test]
fn no_unlocked_vault_coins_to_withdraw() {
    let uatom = uatom_info();
    let uosmo = uosmo_info();

    let leverage_vault = locked_vault_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[uatom.clone(), uosmo.clone()])
        .vault_configs(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(300, "uatom"), coin(500, "uosmo")],
        })
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(coin(200, uatom.denom)),
            Deposit(coin(200, uosmo.denom)),
            ExitVault {
                vault,
                amount: STARTING_VAULT_SHARES,
            },
        ],
        &[coin(200, "uatom"), coin(200, "uosmo")],
    );

    assert_err(
        res,
        VaultError::from(OverflowError {
            operation: Sub,
            operand1: "0".to_string(),
            operand2: STARTING_VAULT_SHARES.to_string(),
        })
        .into(),
    )
}

#[test]
fn withdraw_with_unlocked_vault_coins() {
    let lp_token = lp_token_info();
    let leverage_vault = unlocked_vault_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[lp_token.clone()])
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
                coin: lp_token.to_action_coin(100),
            },
        ],
        &[lp_token.to_coin(200)],
    )
    .unwrap();

    // Assert token's position
    let res = mock.query_positions(&account_id);
    assert_eq!(res.vaults.len(), 1);
    let v = res.vaults.first().unwrap();
    assert_eq!(v.amount.unlocked(), STARTING_VAULT_SHARES);
    let lp = get_coin(&lp_token.denom, &res.deposits);
    assert_eq!(lp.amount, Uint128::from(100u128));

    // Assert Rover's totals
    let lp = mock.query_balance(&mock.rover, &lp_token.denom);
    assert_eq!(lp.amount, Uint128::from(100u128));

    // Assert Rover has the vault tokens
    let lp_balance = mock.query_balance(&mock.rover, &leverage_vault.vault_token_denom);
    assert_eq!(STARTING_VAULT_SHARES, lp_balance.amount);

    mock.update_credit_account(
        &account_id,
        &user,
        vec![ExitVault {
            vault,
            amount: STARTING_VAULT_SHARES,
        }],
        &[],
    )
    .unwrap();

    // Assert token's updated position
    let res = mock.query_positions(&account_id);
    assert_eq!(res.vaults.len(), 0);
    let lp = get_coin(&lp_token.denom, &res.deposits);
    assert_eq!(lp.amount, Uint128::from(200u128));

    // Assert Rover indeed has those on hand in the bank
    let lp = mock.query_balance(&mock.rover, &lp_token.denom);
    assert_eq!(lp.amount, Uint128::from(200u128));

    // Assert Rover does not have the vault tokens anymore
    let lp_balance = mock.query_balance(&mock.rover, &leverage_vault.vault_token_denom);
    assert_eq!(Uint128::zero(), lp_balance.amount);
}

#[test]
fn exit_vault_if_vault_is_no_longer_whitelisted() {
    let lp_token = lp_token_info();
    let leverage_vault = unlocked_vault_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[lp_token.clone()])
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
                coin: lp_token.to_action_coin(100),
            },
        ],
        &[lp_token.to_coin(200)],
    )
    .unwrap();

    // Assert token's position
    let res = mock.query_positions(&account_id);
    assert_eq!(res.vaults.len(), 1);
    let v = res.vaults.first().unwrap();
    assert_eq!(v.amount.unlocked(), STARTING_VAULT_SHARES);
    let lp = get_coin(&lp_token.denom, &res.deposits);
    assert_eq!(lp.amount, Uint128::from(100u128));

    // Assert Rover's totals
    let lp = mock.query_balance(&mock.rover, &lp_token.denom);
    assert_eq!(lp.amount, Uint128::from(100u128));

    // Assert Rover has the vault tokens
    let lp_balance = mock.query_balance(&mock.rover, &leverage_vault.vault_token_denom);
    assert_eq!(STARTING_VAULT_SHARES, lp_balance.amount);

    // Blacklist vault
    let mut config = mock.query_vault_params(&vault.address);
    config.whitelisted = false;
    mock.update_vault_params(VaultConfigUpdate::AddOrUpdate {
        config: config.into(),
    });

    mock.update_credit_account(
        &account_id,
        &user,
        vec![ExitVault {
            vault,
            amount: STARTING_VAULT_SHARES,
        }],
        &[],
    )
    .unwrap();

    // Assert token's updated position
    let res = mock.query_positions(&account_id);
    assert_eq!(res.vaults.len(), 0);
    let lp = get_coin(&lp_token.denom, &res.deposits);
    assert_eq!(lp.amount, Uint128::from(200u128));

    // Assert Rover indeed has those on hand in the bank
    let lp = mock.query_balance(&mock.rover, &lp_token.denom);
    assert_eq!(lp.amount, Uint128::from(200u128));

    // Assert Rover does not have the vault tokens anymore
    let lp_balance = mock.query_balance(&mock.rover, &leverage_vault.vault_token_denom);
    assert_eq!(Uint128::zero(), lp_balance.amount);
}

fn get_coin(denom: &str, coins: &[Coin]) -> Coin {
    coins.iter().find(|cv| cv.denom == denom).unwrap().clone()
}
