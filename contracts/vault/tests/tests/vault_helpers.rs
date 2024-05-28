use anyhow::Result as AnyResult;
use cosmwasm_std::{Addr, Coin, Uint128};
use cw_multi_test::{AppResponse, Executor};
use mars_vault::msg::{
    ExecuteMsg, ExtensionExecuteMsg, ExtensionQueryMsg, QueryMsg, VaultInfoResponseExt,
};

use super::helpers::MockEnv;

pub fn query_vault_info(mock_env: &MockEnv, vault: &Addr) -> VaultInfoResponseExt {
    mock_env
        .app
        .wrap()
        .query_wasm_smart(
            vault.to_string(),
            &QueryMsg::VaultExtension(ExtensionQueryMsg::VaultInfo),
        )
        .unwrap()
}

pub fn execute_bind_credit_manager_account(
    mock_env: &mut MockEnv,
    sender: &Addr,
    vault: &Addr,
    account_id: &str,
) -> AnyResult<AppResponse> {
    mock_env.app.execute_contract(
        sender.clone(),
        vault.clone(),
        &ExecuteMsg::VaultExtension(ExtensionExecuteMsg::BindCreditManagerAccount {
            account_id: account_id.to_string(),
        }),
        &[],
    )
}

pub fn execute_deposit(
    mock_env: &mut MockEnv,
    sender: &Addr,
    vault: &Addr,
    amount: Uint128,
    recipient: Option<String>,
    funds: &[Coin],
) -> AnyResult<AppResponse> {
    mock_env.app.execute_contract(
        sender.clone(),
        vault.clone(),
        &ExecuteMsg::Deposit {
            amount,
            recipient,
        },
        funds,
    )
}

pub fn execute_redeem(
    mock_env: &mut MockEnv,
    sender: &Addr,
    vault: &Addr,
    amount: Uint128,
    recipient: Option<String>,
    funds: &[Coin],
) -> AnyResult<AppResponse> {
    mock_env.app.execute_contract(
        sender.clone(),
        vault.clone(),
        &ExecuteMsg::Redeem {
            amount,
            recipient,
        },
        funds,
    )
}

pub fn assert_vault_err(res: AnyResult<AppResponse>, err: mars_vault::error::ContractError) {
    match res {
        Ok(_) => panic!("Result was not an error"),
        Err(generic_err) => {
            let contract_err: mars_vault::error::ContractError = generic_err.downcast().unwrap();
            assert_eq!(contract_err, err);
        }
    }
}
