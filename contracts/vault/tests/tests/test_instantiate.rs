use std::str::FromStr;

use anyhow::Result as AnyResult;
use cosmwasm_std::{coin, Addr, Decimal};
use cw_multi_test::Executor;
use mars_vault::{
    error::ContractError,
    msg::{InstantiateMsg, VaultInfoResponseExt},
    performance_fee::PerformanceFeeConfig,
};

use super::helpers::{mock_managed_vault_contract, AccountToFund, MockEnv};
use crate::tests::{helpers::deploy_managed_vault, vault_helpers::query_vault_info};

#[test]
fn instantiate_with_empty_metadata() {
    let fund_manager = Addr::unchecked("fund-manager");
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![coin(1_000_000_000, "untrn")],
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
            vault_account_id: None,
            cooldown_period: 60,
            performance_fee_config: PerformanceFeeConfig {
                fee_rate: Decimal::zero(),
                withdrawal_interval: 0
            }
        }
    )
}

#[test]
fn instantiate_with_metadata() {
    let fund_manager = Addr::unchecked("fund-manager");
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![coin(1_000_000_000, "untrn")],
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
                cooldown_period: 8521,
                performance_fee_config: PerformanceFeeConfig {
                    fee_rate: Decimal::from_str("0.000046287042457349").unwrap(),
                    withdrawal_interval: 1563,
                },
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
            vault_account_id: None,
            cooldown_period: 8521,
            performance_fee_config: PerformanceFeeConfig {
                fee_rate: Decimal::from_str("0.000046287042457349").unwrap(),
                withdrawal_interval: 1563,
            }
        }
    )
}

#[test]
fn cannot_instantiate_with_invalid_performance_fee() {
    let fund_manager = Addr::unchecked("fund-manager");
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![coin(1_000_000_000, "untrn")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    let contract_code_id = mock.app.store_code(mock_managed_vault_contract());
    let res = mock.app.instantiate_contract(
        contract_code_id,
        fund_manager,
        &InstantiateMsg {
            base_token: "uusdc".to_string(),
            vault_token_subdenom: "fund".to_string(),
            title: None,
            subtitle: None,
            description: None,
            credit_manager: credit_manager.to_string(),
            cooldown_period: 8521,
            performance_fee_config: PerformanceFeeConfig {
                fee_rate: Decimal::from_str("0.000046287042457350").unwrap(),
                withdrawal_interval: 1563,
            },
        },
        &[coin(10_000_000, "untrn")], // Token Factory fee for minting new denom. Configured in the Token Factory module in `mars-testing` package.
        "mock-managed-vault",
        None,
    );

    assert_vault_err(
        res,
        ContractError::InvalidPerformanceFee {
            expected: Decimal::from_str("0.000046287042457349").unwrap(),
            actual: Decimal::from_str("0.000046287042457350").unwrap(),
        },
    );
}

fn assert_vault_err(res: AnyResult<Addr>, err: ContractError) {
    match res {
        Ok(_) => panic!("Result was not an error"),
        Err(generic_err) => {
            let contract_err: ContractError = generic_err.downcast().unwrap();
            assert_eq!(contract_err, err);
        }
    }
}
