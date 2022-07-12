extern crate core;

use cosmwasm_std::{to_binary, Addr, Uint128};
use cw20::{BalanceResponse, Cw20Coin, Cw20ExecuteMsg, Cw20QueryMsg};
use cw_asset::{AssetInfo, AssetInfoUnchecked};
use cw_multi_test::Executor;

use rover::error::ContractError::{NotTokenOwner, NotWhitelisted};
use rover::msg::execute::ReceiveMsg;

use crate::helpers::{
    assert_err, deploy_mock_cw20, get_token_id, mock_app, mock_create_credit_account,
    query_position, setup_credit_manager,
};

pub mod helpers;

#[test]
fn test_only_token_owner_can_deposit() {
    let mut app = mock_app();
    let user = Addr::unchecked("user");
    let another_user = Addr::unchecked("another_user");

    let cw20_contract = deploy_mock_cw20(
        &mut app,
        "jakecoin",
        vec![Cw20Coin {
            address: another_user.to_string(),
            amount: Uint128::from(500u128),
        }],
    );

    let manager_contract = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![AssetInfoUnchecked::Cw20(cw20_contract.clone().to_string())],
    );
    let res = mock_create_credit_account(&mut app, &manager_contract, &user).unwrap();
    let token_id = get_token_id(res);
    let amount = Uint128::from(300u128);

    let res = app.execute_contract(
        another_user.clone(),
        cw20_contract.clone(),
        &Cw20ExecuteMsg::Send {
            contract: manager_contract.to_string(),
            amount,
            msg: to_binary(&ReceiveMsg::Deposit {
                token_id: token_id.clone(),
            })
            .unwrap(),
        },
        &[],
    );

    assert_err(
        res,
        NotTokenOwner {
            user: another_user.to_string(),
            token_id: token_id.clone(),
        },
    );

    let res = query_position(&app, &manager_contract, &token_id);
    assert_eq!(res.assets.len(), 0);
}

#[test]
fn test_can_only_deposit_allowed_assets() {
    let mut app = mock_app();
    let user = Addr::unchecked("user");
    let cw20_contract_a = deploy_mock_cw20(
        &mut app,
        "jakecoin",
        vec![Cw20Coin {
            address: user.to_string(),
            amount: Uint128::from(500u128),
        }],
    );

    let cw20_contract_b = deploy_mock_cw20(
        &mut app,
        "sparkycoin",
        vec![Cw20Coin {
            address: user.to_string(),
            amount: Uint128::from(500u128),
        }],
    );

    let contract_addr = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![AssetInfoUnchecked::Cw20(cw20_contract_b.to_string())],
    );
    let res = mock_create_credit_account(&mut app, &contract_addr, &user).unwrap();
    let token_id = get_token_id(res);
    let amount = Uint128::from(300u128);

    let res = app.execute_contract(
        user.clone(),
        cw20_contract_a.clone(),
        &Cw20ExecuteMsg::Send {
            contract: contract_addr.to_string(),
            amount,
            msg: to_binary(&ReceiveMsg::Deposit {
                token_id: token_id.clone(),
            })
            .unwrap(),
        },
        &[],
    );

    assert_err(
        res,
        NotWhitelisted(AssetInfo::Cw20(cw20_contract_a).to_string()),
    );

    let res = query_position(&app, &contract_addr, &token_id);
    assert_eq!(res.assets.len(), 0);
}

#[test]
fn test_cw20_deposit_success() {
    let mut app = mock_app();
    let user = Addr::unchecked("user");
    let cw20_contract = deploy_mock_cw20(
        &mut app,
        "jakecoin",
        vec![Cw20Coin {
            address: user.to_string(),
            amount: Uint128::from(500u128),
        }],
    );
    let asset_info = AssetInfoUnchecked::cw20(cw20_contract.clone());

    let contract_addr = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![asset_info.clone()],
    );
    let res = mock_create_credit_account(&mut app, &contract_addr, &user).unwrap();
    let token_id = get_token_id(res);
    let amount = Uint128::from(300u128);

    app.execute_contract(
        user.clone(),
        cw20_contract.clone(),
        &Cw20ExecuteMsg::Send {
            contract: contract_addr.to_string(),
            amount,
            msg: to_binary(&ReceiveMsg::Deposit {
                token_id: token_id.clone(),
            })
            .unwrap(),
        },
        &[],
    )
    .unwrap();

    let res = query_position(&app, &contract_addr, &token_id);
    assert_eq!(res.assets.len(), 1);
    assert_eq!(res.assets.first().unwrap().amount, amount);
    assert_eq!(res.assets.first().unwrap().info, asset_info);

    let res: BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            cw20_contract.clone(),
            &Cw20QueryMsg::Balance {
                address: contract_addr.into(),
            },
        )
        .unwrap();

    assert_eq!(res.balance, amount)
}
