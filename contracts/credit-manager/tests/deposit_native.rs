extern crate core;

use cosmwasm_std::{Addr, Coin, Uint128};
use cw20::Cw20Coin;
use cw_asset::{AssetInfo, AssetInfoUnchecked, AssetList, AssetUnchecked};
use cw_multi_test::{App, Executor};
use rover::error::ContractError::{
    ExtraFundsReceived, FundsMismatch, NotTokenOwner, NotWhitelisted,
};

use rover::msg::execute::Action;
use rover::msg::ExecuteMsg;

use crate::helpers::{
    assert_err, deploy_mock_cw20, get_token_id, mock_app, mock_create_credit_account,
    query_position, setup_credit_manager,
};

pub mod helpers;

#[test]
fn test_only_owner_of_token_can_deposit() {
    let mut app = mock_app();
    let info = AssetInfoUnchecked::native("uosmo");
    let asset = AssetUnchecked::new(info.clone(), Uint128::zero());

    let contract_addr =
        setup_credit_manager(&mut app, &Addr::unchecked("owner"), vec![info.clone()]);

    let user = Addr::unchecked("user");
    let res = mock_create_credit_account(&mut app, &contract_addr, &user).unwrap();
    let token_id = get_token_id(res);

    let another_user = Addr::unchecked("another_user");
    let res = app.execute_contract(
        another_user.clone(),
        contract_addr.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Action::NativeDeposit(asset)],
        },
        &[],
    );

    assert_err(
        res,
        NotTokenOwner {
            user: another_user.into(),
            token_id,
        },
    )
}

#[test]
fn test_deposit_nothing() {
    let mut app = mock_app();
    let info = AssetInfoUnchecked::native("uosmo");
    let asset = AssetUnchecked::new(info.clone(), Uint128::zero());
    let contract_addr =
        setup_credit_manager(&mut app, &Addr::unchecked("owner"), vec![info.clone()]);

    let user = Addr::unchecked("user");
    let res = mock_create_credit_account(&mut app, &contract_addr, &user).unwrap();
    let token_id = get_token_id(res);

    let res = query_position(&app, &contract_addr, &token_id);
    assert_eq!(res.assets.len(), 0);

    app.execute_contract(
        user.clone(),
        contract_addr.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Action::NativeDeposit(asset)],
        },
        &[],
    )
    .unwrap();

    let res = query_position(&app, &contract_addr, &token_id);
    assert_eq!(res.assets.len(), 0);
}

#[test]
fn test_deposit_but_no_funds() {
    let mut app = mock_app();
    let info = AssetInfoUnchecked::native("uosmo");
    let amount = Uint128::from(234u128);
    let asset = AssetUnchecked::new(info.clone(), amount);
    let contract_addr = setup_credit_manager(&mut app, &Addr::unchecked("owner"), vec![info]);

    let user = Addr::unchecked("user");
    let res = mock_create_credit_account(&mut app, &contract_addr, &user).unwrap();
    let token_id = get_token_id(res);

    let res = app.execute_contract(
        user.clone(),
        contract_addr.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Action::NativeDeposit(asset.clone())],
        },
        &[],
    );

    assert_err(
        res,
        FundsMismatch {
            expected: asset.amount,
            received: Uint128::zero(),
        },
    );

    let res = query_position(&app, &contract_addr, &token_id);
    assert_eq!(res.assets.len(), 0);
}

#[test]
fn test_deposit_but_not_enough_funds() {
    let user = Addr::unchecked("user");
    let funds = Coin::new(300u128, "uosmo");
    let info = AssetInfoUnchecked::native("uosmo");
    let amount = Uint128::from(350u128);
    let asset = AssetUnchecked::new(info.clone(), amount);

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

    let res = app.execute_contract(
        user.clone(),
        contract_addr.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Action::NativeDeposit(asset.clone())],
        },
        &[Coin::new(250u128, "uosmo")],
    );

    assert_err(
        res,
        FundsMismatch {
            expected: asset.amount,
            received: Uint128::from(250u128),
        },
    );
}

#[test]
fn test_can_only_deposit_allowed_assets() {
    let user = Addr::unchecked("user");
    let funds = Coin::new(300u128, "uosmo");

    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &user, vec![funds])
            .unwrap();
    });
    let cw20_contract = deploy_mock_cw20(
        &mut app,
        "jakecoin",
        vec![Cw20Coin {
            address: user.to_string(),
            amount: Uint128::from(500u128),
        }],
    );
    let contract_addr = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![
            AssetInfoUnchecked::native("ucosmos"),
            AssetInfoUnchecked::cw20(cw20_contract),
        ],
    );

    let res = mock_create_credit_account(&mut app, &contract_addr, &user).unwrap();
    let token_id = get_token_id(res);

    let info = AssetInfoUnchecked::native("uosmo");
    let amount = Uint128::from(234u128);
    let asset = AssetUnchecked::new(info.clone(), amount);

    let res = app.execute_contract(
        user.clone(),
        contract_addr.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Action::NativeDeposit(asset.clone())],
        },
        &[Coin::new(234u128, "uosmo")],
    );

    assert_err(res, NotWhitelisted(AssetInfo::native("uosmo").to_string()));

    let res = query_position(&app, &contract_addr, &token_id);
    assert_eq!(res.assets.len(), 0);
}

#[test]
fn test_extra_funds_received() {
    let user = Addr::unchecked("user");

    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(
                storage,
                &user,
                vec![Coin::new(300u128, "uosmo"), Coin::new(50u128, "ucosmos")],
            )
            .unwrap();
    });

    let info = AssetInfoUnchecked::native("uosmo");
    let amount = Uint128::from(234u128);
    let asset = AssetUnchecked::new(info.clone(), amount);

    let contract_addr =
        setup_credit_manager(&mut app, &Addr::unchecked("owner"), vec![info.clone()]);

    let res = mock_create_credit_account(&mut app, &contract_addr, &user).unwrap();
    let token_id = get_token_id(res);

    let extra_funds = Coin::new(25u128, "ucosmos");
    let res = app.execute_contract(
        user.clone(),
        contract_addr.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Action::NativeDeposit(asset.clone())],
        },
        &[Coin::new(234u128, "uosmo"), extra_funds.clone()],
    );

    assert_err(res, ExtraFundsReceived(AssetList::from(vec![extra_funds])));

    let res = query_position(&app, &contract_addr, &token_id);
    assert_eq!(res.assets.len(), 0);
}

#[test]
fn test_native_deposit_success() {
    let user = Addr::unchecked("user");
    let funds = Coin::new(300u128, "uosmo");
    let info = AssetInfoUnchecked::native("uosmo");
    let amount = Uint128::from(234u128);
    let asset = AssetUnchecked::new(info.clone(), amount);

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

    app.execute_contract(
        user.clone(),
        contract_addr.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Action::NativeDeposit(asset.clone())],
        },
        &[Coin::new(234u128, "uosmo")],
    )
    .unwrap();

    let res = query_position(&app, &contract_addr, &token_id);
    assert_eq!(res.assets.len(), 1);
    assert_eq!(res.assets.first().unwrap().amount, amount);
    assert_eq!(res.assets.first().unwrap().info, info);

    let coin = app.wrap().query_balance(contract_addr, "uosmo").unwrap();
    assert_eq!(coin.amount, amount)
}

#[test]
fn test_multiple_deposit_actions() {
    let user = Addr::unchecked("user");
    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(
                storage,
                &user,
                vec![Coin::new(300u128, "uosmo"), Coin::new(50u128, "ucosmos")],
            )
            .unwrap();
    });

    let asset_a = AssetUnchecked::new(AssetInfoUnchecked::native("uosmo"), Uint128::from(234u128));
    let asset_b = AssetUnchecked::new(AssetInfoUnchecked::native("ucosmos"), Uint128::from(25u128));

    let contract_addr = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![asset_a.clone().info, asset_b.clone().info],
    );

    let res = mock_create_credit_account(&mut app, &contract_addr, &user).unwrap();
    let token_id = get_token_id(res);

    app.execute_contract(
        user.clone(),
        contract_addr.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![
                Action::NativeDeposit(asset_a.clone()),
                Action::NativeDeposit(asset_b.clone()),
            ],
        },
        &[Coin::new(234u128, "uosmo"), Coin::new(25u128, "ucosmos")],
    )
    .unwrap();

    let res = query_position(&app, &contract_addr, &token_id);
    assert_eq!(res.assets.len(), 2);

    let coin = app
        .wrap()
        .query_balance(contract_addr.clone(), "uosmo")
        .unwrap();
    assert_eq!(coin.amount, Uint128::from(234u128));

    let coin = app.wrap().query_balance(contract_addr, "ucosmos").unwrap();
    assert_eq!(coin.amount, Uint128::from(25u128));
}
