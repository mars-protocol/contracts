use anyhow::Result as AnyResult;
use cosmwasm_std::{coin, Addr, Coin, Uint128};
use cw_multi_test::{AppResponse, Executor};
use mars_types::health::AccountKind;
use mars_vault::msg::{
    ExecuteMsg, ExtensionExecuteMsg, ExtensionQueryMsg, InstantiateMsg, QueryMsg,
    VaultInfoResponseExt,
};

use super::helpers::{mock_managed_vault_contract, AccountToFund, MockEnv};
use crate::tests::helpers::deploy_managed_vault;

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
        }
    )
}

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
    assert_vault_err(res, mars_vault::error::ContractError::NotCreditManager {});

    execute_bind_credit_manager_account(&mut mock, &credit_manager, &managed_vault_addr, "2024")
        .unwrap();
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
            vault_account_id: Some("2024".to_string()),
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

    execute_bind_credit_manager_account(&mut mock, &credit_manager, &managed_vault_addr, "2024")
        .unwrap();
    let res = execute_bind_credit_manager_account(
        &mut mock,
        &credit_manager,
        &managed_vault_addr,
        "2025",
    );
    assert_vault_err(res, mars_vault::error::ContractError::VaultAccountExists {});
}

#[test]
fn deposit_invalid_funds() {
    let fund_manager = Addr::unchecked("fund-manager");
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![coin(1_000_000_000, "untrn")],
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(1_000_000_000, "untrn"), coin(1_000_000_000, "uusdc")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    let managed_vault_addr = deploy_managed_vault(&mut mock.app, &fund_manager, &credit_manager);
    execute_bind_credit_manager_account(&mut mock, &credit_manager, &managed_vault_addr, "2024")
        .unwrap();

    let res = execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[],
    );
    assert_vault_err(
        res,
        mars_vault::error::ContractError::Payment(cw_utils::PaymentError::NoFunds {}),
    );

    let res = execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(1_001, "untrn"), coin(1_002, "uusdc")],
    );
    assert_vault_err(
        res,
        mars_vault::error::ContractError::Payment(cw_utils::PaymentError::MultipleDenoms {}),
    );

    let res = execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(1_001, "untrn")],
    );
    assert_vault_err(
        res,
        mars_vault::error::ContractError::Payment(cw_utils::PaymentError::MissingDenom(
            "uusdc".to_string(),
        )),
    );
}

#[test]
fn deposit_if_credit_manager_account_not_binded() {
    let fund_manager = Addr::unchecked("fund-manager");
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![coin(1_000_000_000, "untrn")],
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(1_000_000_000, "uusdc")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    let managed_vault_addr = deploy_managed_vault(&mut mock.app, &fund_manager, &credit_manager);

    let deposited_amt = Uint128::new(123_000_000);
    let res = execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(deposited_amt.u128(), "uusdc")],
    );
    assert_vault_err(res, mars_vault::error::ContractError::VaultAccountNotFound {});
}

#[test]
fn deposit_and_redeem_succeded() {
    let fund_manager = Addr::unchecked("fund-manager");
    let user = Addr::unchecked("user");
    let user_funded_amt = Uint128::new(1_000_000_000);
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![coin(1_000_000_000, "untrn")],
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(user_funded_amt.u128(), "uusdc")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    let managed_vault_addr = deploy_managed_vault(&mut mock.app, &fund_manager, &credit_manager);
    let vault_info_res = query_vault_info(&mock, &managed_vault_addr);
    let vault_token = vault_info_res.vault_token;

    // there shouldn't be any vault tokens
    let vault_token_balance = mock.query_balance(&managed_vault_addr, &vault_token).amount;
    assert!(vault_token_balance.is_zero());
    let vault_token_balance = mock.query_balance(&user, &vault_token).amount;
    assert!(vault_token_balance.is_zero());

    let account_id = mock
        .create_credit_account_v2(
            &fund_manager,
            AccountKind::FundManager {
                vault_addr: managed_vault_addr.to_string(),
            },
            None,
        )
        .unwrap();

    let deposited_amt = Uint128::new(123_000_000);
    execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(deposited_amt.u128(), "uusdc")],
    )
    .unwrap();

    // check base token balance after deposit
    let user_base_token_balance = mock.query_balance(&user, "uusdc").amount;
    assert_eq!(user_base_token_balance, user_funded_amt - deposited_amt);

    // there should be vault tokens for the user now
    let vault_token_balance = mock.query_balance(&managed_vault_addr, &vault_token).amount;
    assert!(vault_token_balance.is_zero());
    let user_vault_token_balance = mock.query_balance(&user, &vault_token).amount;
    assert!(!user_vault_token_balance.is_zero());
    assert_eq!(user_vault_token_balance, deposited_amt * Uint128::new(1_000_000));

    // there should be a deposit in Fund Manager's account
    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 1);
    let assets_res = res.deposits.first().unwrap();
    assert_eq!(assets_res.amount, deposited_amt);
    assert_eq!(assets_res.denom, "uusdc".to_string());

    execute_redeem(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(user_vault_token_balance.u128(), vault_token.clone())],
    )
    .unwrap();

    // there shouldn't be any vault tokens after redeem
    let vault_token_balance = mock.query_balance(&managed_vault_addr, &vault_token).amount;
    assert!(vault_token_balance.is_zero());
    let vault_token_balance = mock.query_balance(&user, &vault_token).amount;
    assert!(vault_token_balance.is_zero());

    // check base token balance after redeem
    let user_base_token_balance = mock.query_balance(&user, "uusdc").amount;
    assert_eq!(user_base_token_balance, user_funded_amt);

    // check Fund Manager's account after redeem
    let res = mock.query_positions(&account_id);
    assert!(res.deposits.is_empty());
}

#[test]
fn redeem_invalid_funds() {
    let fund_manager = Addr::unchecked("fund-manager");
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![coin(1_000_000_000, "untrn")],
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(1_000_000_000, "untrn"), coin(1_000_000_000, "uusdc")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    let managed_vault_addr = deploy_managed_vault(&mut mock.app, &fund_manager, &credit_manager);
    execute_bind_credit_manager_account(&mut mock, &credit_manager, &managed_vault_addr, "2024")
        .unwrap();

    let res = execute_redeem(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[],
    );
    assert_vault_err(
        res,
        mars_vault::error::ContractError::Payment(cw_utils::PaymentError::NoFunds {}),
    );

    let res = execute_redeem(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(1_001, "untrn"), coin(1_002, "uusdc")],
    );
    assert_vault_err(
        res,
        mars_vault::error::ContractError::Payment(cw_utils::PaymentError::MultipleDenoms {}),
    );

    let res = execute_redeem(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(1_001, "untrn")],
    );
    assert_vault_err(
        res,
        mars_vault::error::ContractError::Payment(cw_utils::PaymentError::MissingDenom(
            "factory/contract10/vault".to_string(),
        )),
    );
}

#[test]
fn redeem_if_credit_manager_account_not_binded() {
    let fund_manager = Addr::unchecked("fund-manager");
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![coin(1_000_000_000, "untrn")],
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(1_000_000_000, "vault")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    let managed_vault_addr = deploy_managed_vault(&mut mock.app, &fund_manager, &credit_manager);

    let deposited_amt = Uint128::new(123_000_000);
    let res = execute_redeem(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(deposited_amt.u128(), "vault")],
    );
    assert_vault_err(res, mars_vault::error::ContractError::VaultAccountNotFound {});
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

fn execute_bind_credit_manager_account(
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

fn execute_deposit(
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

fn execute_redeem(
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
