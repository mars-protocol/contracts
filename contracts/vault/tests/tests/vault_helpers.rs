use anyhow::Result as AnyResult;
use cosmwasm_std::{Addr, Coin, Uint128};
use cw_multi_test::{AppResponse, Executor};
use cw_paginate::PaginationResponse;
use mars_vault::{
    msg::{
        ExecuteMsg, ExtensionExecuteMsg, ExtensionQueryMsg, QueryMsg, VaultInfoResponseExt,
        VaultUnlock,
    },
    performance_fee::{PerformanceFeeConfig, PerformanceFeeState},
};

use super::helpers::MockEnv;

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

pub fn execute_unlock(
    mock_env: &mut MockEnv,
    sender: &Addr,
    vault: &Addr,
    amount: Uint128,
    funds: &[Coin],
) -> AnyResult<AppResponse> {
    mock_env.app.execute_contract(
        sender.clone(),
        vault.clone(),
        &ExecuteMsg::VaultExtension(ExtensionExecuteMsg::Unlock {
            amount,
        }),
        funds,
    )
}

pub fn execute_withdraw_performance_fee(
    mock_env: &mut MockEnv,
    sender: &Addr,
    vault: &Addr,
    new_performance_fee_config: Option<PerformanceFeeConfig>,
) -> AnyResult<AppResponse> {
    mock_env.app.execute_contract(
        sender.clone(),
        vault.clone(),
        &ExecuteMsg::VaultExtension(ExtensionExecuteMsg::WithdrawPerformanceFee {
            new_performance_fee_config,
        }),
        &[],
    )
}

pub fn query_vault_info(mock_env: &MockEnv, vault: &Addr) -> VaultInfoResponseExt {
    mock_env
        .app
        .wrap()
        .query_wasm_smart(
            vault.to_string(),
            &QueryMsg::VaultExtension(ExtensionQueryMsg::VaultInfo {}),
        )
        .unwrap()
}

pub fn query_total_assets(mock_env: &MockEnv, vault: &Addr) -> Uint128 {
    mock_env.app.wrap().query_wasm_smart(vault.to_string(), &QueryMsg::TotalAssets {}).unwrap()
}

pub fn query_total_vault_token_supply(mock_env: &MockEnv, vault: &Addr) -> Uint128 {
    mock_env
        .app
        .wrap()
        .query_wasm_smart(vault.to_string(), &QueryMsg::TotalVaultTokenSupply {})
        .unwrap()
}

pub fn query_user_unlocks(mock_env: &MockEnv, vault: &Addr, user_addr: &Addr) -> Vec<VaultUnlock> {
    mock_env
        .app
        .wrap()
        .query_wasm_smart(
            vault.to_string(),
            &QueryMsg::VaultExtension(ExtensionQueryMsg::UserUnlocks {
                user_address: user_addr.to_string(),
            }),
        )
        .unwrap()
}

pub fn query_all_unlocks(
    mock_env: &MockEnv,
    vault: &Addr,
    start_after: Option<(String, u64)>,
    limit: Option<u32>,
) -> PaginationResponse<VaultUnlock> {
    mock_env
        .app
        .wrap()
        .query_wasm_smart(
            vault.to_string(),
            &QueryMsg::VaultExtension(ExtensionQueryMsg::AllUnlocks {
                start_after,
                limit,
            }),
        )
        .unwrap()
}

pub fn query_convert_to_assets(mock_env: &MockEnv, vault: &Addr, vault_tokens: Uint128) -> Uint128 {
    mock_env
        .app
        .wrap()
        .query_wasm_smart(
            vault.to_string(),
            &QueryMsg::ConvertToAssets {
                amount: vault_tokens,
            },
        )
        .unwrap()
}

pub fn query_convert_to_shares(mock_env: &MockEnv, vault: &Addr, base_tokens: Uint128) -> Uint128 {
    mock_env
        .app
        .wrap()
        .query_wasm_smart(
            vault.to_string(),
            &QueryMsg::ConvertToShares {
                amount: base_tokens,
            },
        )
        .unwrap()
}

pub fn query_performance_fee(mock_env: &MockEnv, vault: &Addr) -> PerformanceFeeState {
    mock_env
        .app
        .wrap()
        .query_wasm_smart(
            vault.to_string(),
            &QueryMsg::VaultExtension(ExtensionQueryMsg::PerformanceFeeState {}),
        )
        .unwrap()
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
