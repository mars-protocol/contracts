use std::ops::{Mul, Sub};

use cosmwasm_std::{Addr, Coin, Uint128};
use cw20::{BalanceResponse, Cw20Coin, Cw20ExecuteMsg};
use cw20_base::msg::QueryMsg::Balance;
use cw_asset::{AssetInfoUnchecked, AssetUnchecked};
use cw_multi_test::{App, AppResponse, BankSudo, BasicApp, Executor, SudoMsg};

use credit_manager::borrow::DEFAULT_DEBT_UNITS_PER_ASSET_BORROWED;
use mock_red_bank::msg::QueryMsg::UserAssetDebt;
use mock_red_bank::msg::UserAssetDebtResponse;
use rover::error::ContractError;
use rover::msg::execute::Action::Borrow;
use rover::msg::query::TotalDebtSharesResponse;
use rover::msg::ExecuteMsg::UpdateCreditAccount;
use rover::msg::QueryMsg;

use crate::helpers::{
    assert_err, deploy_mock_cw20, get_token_id, mock_app, mock_create_credit_account, query_config,
    query_position, setup_credit_manager,
};

pub mod helpers;

#[test]
fn test_only_token_owner_can_borrow() {
    let mut app = mock_app();
    let owner = Addr::unchecked("owner");
    let info = AssetInfoUnchecked::native("uosmo");
    let asset = AssetUnchecked::new(info.clone(), Uint128::zero());

    let contract_addr = setup_credit_manager(&mut app, &owner, vec![info.clone()]);
    let res =
        mock_create_credit_account(&mut app, &contract_addr, &Addr::unchecked("user")).unwrap();
    let token_id = get_token_id(res);

    let another_user = Addr::unchecked("another_user");
    let res = app.execute_contract(
        another_user.clone(),
        contract_addr.clone(),
        &UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Borrow(asset)],
        },
        &[],
    );

    assert_err(
        res,
        ContractError::NotTokenOwner {
            user: another_user.into(),
            token_id,
        },
    )
}

#[test]
fn test_can_only_borrow_what_is_whitelisted() {
    let mut app = mock_app();
    let owner = Addr::unchecked("owner");
    let info = AssetInfoUnchecked::native("uosmo");

    let contract_addr = setup_credit_manager(&mut app, &owner, vec![info.clone()]);
    let user = Addr::unchecked("user");
    let res = mock_create_credit_account(&mut app, &contract_addr, &user).unwrap();
    let token_id = get_token_id(res);

    let res = app.execute_contract(
        user.clone(),
        contract_addr.clone(),
        &UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Borrow(AssetUnchecked::new(
                AssetInfoUnchecked::native("usomething"),
                Uint128::from(234u128),
            ))],
        },
        &[],
    );

    assert_err(
        res,
        ContractError::NotWhitelisted(String::from("native:usomething")),
    )
}

#[test]
fn test_borrowing_zero_does_nothing() {
    let mut app = mock_app();
    let info = AssetInfoUnchecked::native("uosmo");

    let contract_addr =
        setup_credit_manager(&mut app, &Addr::unchecked("owner"), vec![info.clone()]);
    let user = Addr::unchecked("user");
    let res = mock_create_credit_account(&mut app, &contract_addr, &user).unwrap();
    let token_id = get_token_id(res);

    let res = app.execute_contract(
        user.clone(),
        contract_addr.clone(),
        &UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Borrow(AssetUnchecked::new(info, Uint128::zero()))],
        },
        &[],
    );

    assert_err(res, ContractError::NoAmount {});

    let position = query_position(&mut app, &contract_addr, &token_id);
    assert_eq!(position.assets.len(), 0);
    assert_eq!(position.debt_shares.len(), 0);
}

#[test]
fn test_success_when_new_debt_asset() {
    let user = Addr::unchecked("user");
    let funds = Coin::new(300u128, "uosmo");
    let info = AssetInfoUnchecked::native("uosmo");
    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &user, vec![funds])
            .unwrap();
    });

    let contract_addr =
        setup_credit_manager(&mut app, &Addr::unchecked("owner"), vec![info.clone()]);
    let res = mock_create_credit_account(&mut app, &contract_addr, &user).unwrap();
    let token_id = get_token_id(res);

    let config = query_config(&mut app, &contract_addr.clone());

    fund_red_bank_native(
        &mut app,
        config.red_bank.clone(),
        vec![Coin::new(1000u128, "uosmo")],
    );

    let position = query_position(&mut app, &contract_addr, &token_id);
    assert_eq!(position.assets.len(), 0);
    assert_eq!(position.debt_shares.len(), 0);

    app.execute_contract(
        user,
        contract_addr.clone(),
        &UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Borrow(AssetUnchecked::new(
                info.clone(),
                Uint128::from(42u128),
            ))],
        },
        &[],
    )
    .unwrap();

    let position = query_position(&mut app, &contract_addr, &token_id);
    assert_eq!(position.assets.len(), 1);
    assert_eq!(
        position.assets.first().unwrap().amount,
        Uint128::from(42u128)
    );
    assert_eq!(position.assets.first().unwrap().info, info);
    assert_eq!(position.debt_shares.len(), 1);
    assert_eq!(
        position.debt_shares.first().unwrap().amount,
        Uint128::from(42u128).mul(DEFAULT_DEBT_UNITS_PER_ASSET_BORROWED)
    );
    assert_eq!(position.debt_shares.first().unwrap().info, info);

    let coin = app
        .wrap()
        .query_balance(contract_addr.clone(), "uosmo")
        .unwrap();
    assert_eq!(coin.amount, Uint128::from(42u128));

    let coin = app.wrap().query_balance(config.red_bank, "uosmo").unwrap();
    assert_eq!(
        coin.amount,
        Uint128::from(1000u128).sub(Uint128::from(42u128))
    );

    let res: TotalDebtSharesResponse = app
        .wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::TotalDebtShares(info))
        .unwrap();
    assert_eq!(
        res.0.amount,
        Uint128::from(42u128).mul(DEFAULT_DEBT_UNITS_PER_ASSET_BORROWED)
    );
}

#[test]
fn test_debt_shares_with_debt_amount() {
    let user_a = Addr::unchecked("user_a");
    let user_b = Addr::unchecked("user_b");
    let info = AssetInfoUnchecked::native("uosmo");
    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &user_a, vec![Coin::new(300u128, "uosmo")])
            .unwrap();
        router
            .bank
            .init_balance(storage, &user_b, vec![Coin::new(450u128, "uosmo")])
            .unwrap();
    });

    let contract_addr =
        setup_credit_manager(&mut app, &Addr::unchecked("owner"), vec![info.clone()]);
    let res = mock_create_credit_account(&mut app, &contract_addr, &user_a).unwrap();
    let token_id_a = get_token_id(res);
    let res = mock_create_credit_account(&mut app, &contract_addr, &user_b).unwrap();
    let token_id_b = get_token_id(res);

    let config = query_config(&mut app, &contract_addr.clone());

    fund_red_bank_native(
        &mut app,
        config.red_bank.clone(),
        vec![Coin::new(1000u128, "uosmo")],
    );

    app.execute_contract(
        user_a,
        contract_addr.clone(),
        &UpdateCreditAccount {
            token_id: token_id_a.clone(),
            actions: vec![Borrow(AssetUnchecked::new(
                info.clone(),
                Uint128::from(50u128),
            ))],
        },
        &[],
    )
    .unwrap();

    let interim_red_bank_debt: UserAssetDebtResponse = app
        .wrap()
        .query_wasm_smart(
            config.red_bank,
            &UserAssetDebt {
                user_address: contract_addr.clone().into(),
                asset: info.clone(),
            },
        )
        .unwrap();

    app.execute_contract(
        user_b,
        contract_addr.clone(),
        &UpdateCreditAccount {
            token_id: token_id_b.clone(),
            actions: vec![Borrow(AssetUnchecked::new(
                info.clone(),
                Uint128::from(50u128),
            ))],
        },
        &[],
    )
    .unwrap();

    let token_a_shares = Uint128::from(50u128).mul(DEFAULT_DEBT_UNITS_PER_ASSET_BORROWED);
    let position = query_position(&mut app, &contract_addr, &token_id_a);
    assert_eq!(
        position.debt_shares.first().unwrap().amount,
        token_a_shares.clone()
    );

    let token_b_shares = Uint128::from(50u128)
        .mul(DEFAULT_DEBT_UNITS_PER_ASSET_BORROWED)
        .multiply_ratio(Uint128::from(50u128), interim_red_bank_debt.amount);

    let position = query_position(&mut app, &contract_addr, &token_id_b);
    assert_eq!(
        position.debt_shares.first().unwrap().amount,
        token_b_shares.clone()
    );

    let res: TotalDebtSharesResponse = app
        .wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::TotalDebtShares(info))
        .unwrap();
    assert_eq!(res.0.amount, token_a_shares + token_b_shares);
}

#[test]
fn test_can_borrow_cw20() {
    let owner = Addr::unchecked("owner");
    let user = Addr::unchecked("user");
    let mut app = mock_app();

    let cw20_contract = deploy_mock_cw20(
        &mut app,
        "jakecoin",
        vec![Cw20Coin {
            address: owner.clone().into(),
            amount: Uint128::from(1000u128),
        }],
    );
    let cw20_info = AssetInfoUnchecked::cw20(cw20_contract.clone());

    let contract_addr = setup_credit_manager(&mut app, &owner, vec![cw20_info.clone()]);
    let res = mock_create_credit_account(&mut app, &contract_addr, &user).unwrap();
    let token_id = get_token_id(res);

    let config = query_config(&mut app, &contract_addr.clone());
    fund_red_bank_cw20(
        &mut app,
        owner,
        config.red_bank.clone(),
        cw20_contract.clone(),
        Uint128::from(1000u128),
    );

    app.execute_contract(
        user,
        contract_addr.clone(),
        &UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Borrow(AssetUnchecked::new(
                cw20_info.clone(),
                Uint128::from(42u128),
            ))],
        },
        &[],
    )
    .unwrap();

    let position = query_position(&mut app, &contract_addr, &token_id);
    assert_eq!(position.assets.len(), 1);
    assert_eq!(
        position.assets.first().unwrap().amount,
        Uint128::from(42u128)
    );
    assert_eq!(position.assets.first().unwrap().info, cw20_info);
    assert_eq!(position.debt_shares.len(), 1);
    assert_eq!(
        position.debt_shares.first().unwrap().amount,
        Uint128::from(42u128).mul(DEFAULT_DEBT_UNITS_PER_ASSET_BORROWED)
    );
    assert_eq!(position.debt_shares.first().unwrap().info, cw20_info);

    let balance = query_cw20_balance(
        &mut app,
        cw20_contract.clone(),
        contract_addr.clone().into(),
    );
    assert_eq!(balance, Uint128::from(42u128));

    let balance = query_cw20_balance(&mut app, cw20_contract, config.red_bank);
    assert_eq!(balance, Uint128::from(1000u128).sub(Uint128::from(42u128)));

    let res: TotalDebtSharesResponse = app
        .wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::TotalDebtShares(cw20_info))
        .unwrap();
    assert_eq!(
        res.0.amount,
        Uint128::from(42u128).mul(DEFAULT_DEBT_UNITS_PER_ASSET_BORROWED)
    );
}

// TODO: After health check implemented
#[test]
fn test_cannot_borrow_more_than_healthy() {}

fn fund_red_bank_native(app: &mut BasicApp, red_bank_addr: String, funds: Vec<Coin>) {
    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: red_bank_addr,
        amount: funds,
    }))
    .unwrap();
}

fn fund_red_bank_cw20(
    app: &mut BasicApp,
    sender: Addr,
    red_bank_addr: String,
    cw20_contract: Addr,
    amount: Uint128,
) -> AppResponse {
    let res = app
        .execute_contract(
            sender,
            cw20_contract.clone(),
            &Cw20ExecuteMsg::Transfer {
                recipient: red_bank_addr.clone(),
                amount,
            },
            &[],
        )
        .unwrap();
    let balance = query_cw20_balance(app, cw20_contract, red_bank_addr);
    assert_eq!(balance, amount);
    res
}

fn query_cw20_balance(
    app: &mut BasicApp,
    cw20_contract: Addr,
    address_to_query: String,
) -> Uint128 {
    let res: BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            cw20_contract.clone(),
            &Balance {
                address: address_to_query,
            },
        )
        .unwrap();
    res.balance
}
