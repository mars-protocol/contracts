use cosmwasm_std::Addr;
use mars_rover::{
    error::ContractError,
    msg::execute::{
        Action::{Deposit, EnterVault},
        CallbackMsg,
    },
};

use crate::helpers::{assert_err, lp_token_info, unlocked_vault_info, AccountToFund, MockEnv};

pub mod helpers;

#[test]
fn reentrancy_guard_protects_against_evil_vault() {
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
        .evil_vault("2")
        .build()
        .unwrap();

    // Evil vault creates a credit account that will be used to attempt reentrancy
    let vault = mock.get_vault(&leverage_vault);
    mock.create_credit_account(&Addr::unchecked(vault.address.clone())).unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();
    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(lp_token.to_coin(200)),
            EnterVault {
                vault,
                coin: lp_token.to_action_coin(23),
            },
        ],
        &[lp_token.to_coin(200)],
    );

    assert_err(res, ContractError::ReentrancyGuard("Reentrancy guard is active".to_string()));
}

#[test]
fn only_credit_manager_can_remove_guard() {
    let mut mock = MockEnv::new().build().unwrap();
    let external_user = Addr::unchecked("external_user");

    let res = mock.execute_callback(&external_user, CallbackMsg::RemoveReentrancyGuard {});
    assert_err(res, ContractError::ExternalInvocation);
}

#[test]
fn removing_while_inactive() {
    let mut mock = MockEnv::new().build().unwrap();
    let res = mock.execute_callback(&mock.rover.clone(), CallbackMsg::RemoveReentrancyGuard {});
    assert_err(
        res,
        ContractError::ReentrancyGuard("Invalid reentrancy guard state transition".to_string()),
    );
}
