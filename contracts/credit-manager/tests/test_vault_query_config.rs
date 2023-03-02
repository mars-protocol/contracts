use cosmwasm_std::{Addr, StdError};
use mars_rover::{
    adapters::vault::VaultUnchecked,
    msg::execute::Action::{Deposit, EnterVault},
};

use crate::helpers::{lp_token_info, unlocked_vault_info, AccountToFund, MockEnv};

pub mod helpers;

#[test]
fn raises_if_vault_not_in_config() {
    let mock = MockEnv::new().build().unwrap();
    let err = mock.query_vault_config(&VaultUnchecked::new("abc".to_string())).unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err(
            "Querier contract error: mars_rover::adapters::vault::config::VaultConfig not found"
                .to_string()
        )
    );
}

#[test]
fn successfully_queries_with_utilization() {
    let lp_token = lp_token_info();
    let leverage_vault = unlocked_vault_info();

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
                coin: lp_token.to_action_coin(23),
            },
        ],
        &[lp_token.to_coin(200)],
    )
    .unwrap();

    let res = mock.query_vault_config(&vault).unwrap();
    assert_eq!(res.vault, vault);
    assert_eq!(res.config.deposit_cap, leverage_vault.deposit_cap);
    assert_eq!(res.config.max_ltv, leverage_vault.max_ltv);
    assert_eq!(res.config.liquidation_threshold, leverage_vault.liquidation_threshold);
    assert_eq!(res.config.whitelisted, leverage_vault.whitelisted);
}
