use cosmwasm_std::{coin, Addr};
use cw_multi_test::Executor;
use mars_testing::multitest::modules::token_factory::CustomApp;
use mars_vault::msg::{ExtensionQueryMsg, InstantiateMsg, QueryMsg, VaultInfoResponseExt};

use super::helpers::{mock_managed_vault_contract, AccountToFund, MockEnv};

#[test]
fn instantiate_with_empty_metadata_succeded() {
    let fund_manager = Addr::unchecked("fund-manager");
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![coin(1_000_000_000, "untrn"), coin(1_000_000_000, "uusdc")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    let managed_vault_addr = deploy_managed_vault(&mut mock.app, &fund_manager, &credit_manager);

    let vault_info_res = query_vault_info(&mock, &managed_vault_addr);
    assert_eq!(
        vault_info_res,
        VaultInfoResponseExt {
            base_token: "uusdc".to_string(),
            vault_token: format!("factory/{}/vault", managed_vault_addr),
            title: None,
            subtitle: None,
            description: None,
            credit_manager: credit_manager.to_string(),
            fund_manager_account_id: None,
        }
    )
}

#[test]
fn instantiate_with_metadata_succeded() {
    let fund_manager = Addr::unchecked("fund-manager");
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![coin(1_000_000_000, "untrn"), coin(1_000_000_000, "uusdc")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    let contract_code_id = mock.app.store_code(mock_managed_vault_contract());
    let managed_vault_addr = mock
        .app
        .instantiate_contract(
            contract_code_id,
            fund_manager,
            &InstantiateMsg {
                base_token: "uusdc".to_string(),
                vault_token_subdenom: "fund".to_string(),
                title: Some("random TITLE".to_string()),
                subtitle: Some("random subTITLE".to_string()),
                description: Some("The vault manages others money !!!".to_string()),
                credit_manager: credit_manager.to_string(),
            },
            &[coin(10_000_000, "untrn")], // Token Factory fee for minting new denom. Configured in the Token Factory module in `mars-testing` package.
            "mock-managed-vault",
            None,
        )
        .unwrap();

    let vault_info_res = query_vault_info(&mock, &managed_vault_addr);
    assert_eq!(
        vault_info_res,
        VaultInfoResponseExt {
            base_token: "uusdc".to_string(),
            vault_token: format!("factory/{}/fund", managed_vault_addr),
            title: Some("random TITLE".to_string()),
            subtitle: Some("random subTITLE".to_string()),
            description: Some("The vault manages others money !!!".to_string()),
            credit_manager: credit_manager.to_string(),
            fund_manager_account_id: None,
        }
    )
}

fn deploy_managed_vault(app: &mut CustomApp, sender: &Addr, credit_manager: &Addr) -> Addr {
    let contract_code_id = app.store_code(mock_managed_vault_contract());
    app.instantiate_contract(
        contract_code_id,
        sender.clone(),
        &InstantiateMsg {
            base_token: "uusdc".to_string(),
            vault_token_subdenom: "vault".to_string(),
            title: None,
            subtitle: None,
            description: None,
            credit_manager: credit_manager.to_string(),
        },
        &[coin(10_000_000, "untrn")], // Token Factory fee for minting new denom. Configured in the Token Factory module in `mars-testing` package.
        "mock-managed-vault",
        None,
    )
    .unwrap()
}

fn query_vault_info(mock_env: &MockEnv, vault: &Addr) -> VaultInfoResponseExt {
    mock_env
        .app
        .wrap()
        .query_wasm_smart(
            vault.to_string(),
            &QueryMsg::VaultExtension(ExtensionQueryMsg::VaultInfo),
        )
        .unwrap()
}
