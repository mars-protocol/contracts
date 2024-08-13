use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use mars_vault::{
    error::ContractError, msg::VaultInfoResponseExt, performance_fee::PerformanceFeeConfig,
};

use super::{
    helpers::{AccountToFund, MockEnv},
    vault_helpers::{assert_vault_err, execute_bind_credit_manager_account},
};
use crate::tests::{helpers::deploy_managed_vault, vault_helpers::query_vault_info};

#[test]
fn only_credit_manager_can_bind_account() {
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

    let res = execute_bind_credit_manager_account(
        &mut mock,
        &Addr::unchecked("anyone"),
        &managed_vault_addr,
        "2024",
    );
    assert_vault_err(res, ContractError::NotCreditManager {});

    let vault_acc_id = mock.create_fund_manager_account(&fund_manager, &managed_vault_addr);
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
            vault_account_id: Some(vault_acc_id),
            cooldown_period: 60,
            performance_fee_config: PerformanceFeeConfig {
                fee_rate: Decimal::zero(),
                withdrawal_interval: 0
            },
            total_base_tokens: Uint128::zero(),
            total_vault_tokens: Uint128::zero(),
            share_price: None,
        }
    )
}

#[test]
fn only_one_binding_allowed() {
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

    mock.create_fund_manager_account(&fund_manager, &managed_vault_addr);

    let random_existing_acc_id = mock.create_credit_account(&fund_manager).unwrap();
    let res = execute_bind_credit_manager_account(
        &mut mock,
        &credit_manager,
        &managed_vault_addr,
        &random_existing_acc_id,
    );
    assert_vault_err(res, ContractError::VaultAccountExists {});
}

#[test]
fn account_owner_can_not_be_different_than_contract_owner() {
    let random_wallet = Addr::unchecked("random-wallet");
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

    let random_existing_acc_id = mock.create_credit_account(&random_wallet).unwrap();
    let res = execute_bind_credit_manager_account(
        &mut mock,
        &credit_manager,
        &managed_vault_addr,
        &random_existing_acc_id,
    );
    assert_vault_err(
        res,
        ContractError::NotTokenOwner {
            user: fund_manager.to_string(),
            account_id: random_existing_acc_id,
        },
    );
}
