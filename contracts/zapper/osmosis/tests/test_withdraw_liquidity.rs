use cosmwasm_std::{coin, Coin, Uint128};
use cw_dex::CwDexError;
use cw_utils::PaymentError;
use mars_zapper_base::{ContractError, ExecuteMsg, QueryMsg};
use osmosis_testing::{Account, Bank, Gamm, Module, OsmosisTestApp, Wasm};

use crate::helpers::{assert_err, instantiate_contract, query_balance};

pub mod helpers;

#[test]
fn withdraw_liquidity_without_funds() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "gamm/pool/1"), coin(1_000_000_000_000, "uosmo")])
        .unwrap();

    let contract_addr = instantiate_contract(&wasm, &signer);

    let res_err = wasm
        .execute(
            &contract_addr,
            &ExecuteMsg::WithdrawLiquidity {
                recipient: None,
            },
            &[],
            &signer,
        )
        .unwrap_err();
    assert_err(res_err, ContractError::PaymentError(PaymentError::NoFunds {}));
}

#[test]
fn withdraw_liquidity_with_more_than_one_coin_sent() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "gamm/pool/1"), coin(1_000_000_000_000, "uosmo")])
        .unwrap();

    let contract_addr = instantiate_contract(&wasm, &signer);

    let res_err = wasm
        .execute(
            &contract_addr,
            &ExecuteMsg::WithdrawLiquidity {
                recipient: None,
            },
            &[coin(1_000_000, "gamm/pool/1"), coin(2_000_000, "uosmo")],
            &signer,
        )
        .unwrap_err();
    assert_err(res_err, ContractError::PaymentError(PaymentError::MultipleDenoms {}));
}

#[test]
fn withdraw_liquidity_with_invalid_lp_token() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app.init_account(&[coin(1_000_000_000_000, "uosmo")]).unwrap();

    let contract_addr = instantiate_contract(&wasm, &signer);

    let res_err = wasm
        .execute(
            &contract_addr,
            &ExecuteMsg::WithdrawLiquidity {
                recipient: None,
            },
            &[coin(1_000_000, "uosmo")],
            &signer,
        )
        .unwrap_err();
    assert_err(res_err, CwDexError::NotLpToken {});
}

#[test]
fn withdraw_liquidity_successfully() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let uatom_acc_balance = 1_000_000_000_000u128;
    let uosmo_acc_balance = 1_000_000_000_000u128;
    let accs = app
        .init_accounts(&[coin(uatom_acc_balance, "uatom"), coin(uosmo_acc_balance, "uosmo")], 2)
        .unwrap();
    let owner = &accs[0];
    let user = &accs[1];

    let gamm = Gamm::new(&app);
    let pool_id = gamm
        .create_basic_pool(&[coin(20_000_000, "uatom"), coin(40_000_000, "uosmo")], owner)
        .unwrap()
        .data
        .pool_id;
    let pool_denom = format!("gamm/pool/{pool_id}");

    let contract_addr = instantiate_contract(&wasm, owner);

    let bank = Bank::new(&app);

    let user_pool_balance = query_balance(&bank, &user.address(), &pool_denom);
    assert_eq!(user_pool_balance, 0u128);

    let uatom_liquidity_amount = 5_000_000u128;
    let uosmo_liquidity_amount = 10_000_000u128;
    wasm.execute(
        &contract_addr,
        &ExecuteMsg::ProvideLiquidity {
            lp_token_out: pool_denom.clone(),
            recipient: None,
            minimum_receive: Uint128::one(),
        },
        &[coin(uatom_liquidity_amount, "uatom"), coin(uosmo_liquidity_amount, "uosmo")],
        user,
    )
    .unwrap();

    let user_pool_balance = query_balance(&bank, &user.address(), &pool_denom);
    assert_eq!(user_pool_balance, 25000000000000000000u128);
    let user_uatom_balance_before = query_balance(&bank, &user.address(), "uatom");
    let user_uosmo_balance_before = query_balance(&bank, &user.address(), "uosmo");

    let estimate_coins: Vec<Coin> = wasm
        .query(
            &contract_addr,
            &QueryMsg::EstimateWithdrawLiquidity {
                coin_in: coin(user_pool_balance, &pool_denom),
            },
        )
        .unwrap();
    let uatom_estimate_amount =
        estimate_coins.iter().find(|c| c.denom == "uatom").unwrap().amount.u128();
    let uosmo_estimate_amount =
        estimate_coins.iter().find(|c| c.denom == "uosmo").unwrap().amount.u128();
    assert_eq!(uatom_estimate_amount, 4950000u128);
    assert_eq!(uosmo_estimate_amount, 9900000u128);

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::WithdrawLiquidity {
            recipient: None,
        },
        &[coin(user_pool_balance, &pool_denom)],
        user,
    )
    .unwrap();

    let contract_pool_balance = query_balance(&bank, &contract_addr, &pool_denom);
    assert_eq!(contract_pool_balance, 0u128);
    let contract_uatom_balance = query_balance(&bank, &contract_addr, "uatom");
    assert_eq!(contract_uatom_balance, 0u128);
    let contract_uosmo_balance = query_balance(&bank, &contract_addr, "uosmo");
    assert_eq!(contract_uosmo_balance, 0u128);

    let user_pool_balance = query_balance(&bank, &user.address(), &pool_denom);
    assert_eq!(user_pool_balance, 0u128);
    let user_uatom_balance = query_balance(&bank, &user.address(), "uatom");
    assert_eq!(user_uatom_balance, user_uatom_balance_before + uatom_estimate_amount);
    let user_uosmo_balance = query_balance(&bank, &user.address(), "uosmo");
    assert_eq!(user_uosmo_balance, user_uosmo_balance_before + uosmo_estimate_amount);
}

#[test]
fn withdraw_liquidity_with_different_recipient_successfully() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let uatom_acc_balance = 1_000_000_000_000u128;
    let uosmo_acc_balance = 1_000_000_000_000u128;
    let accs = app
        .init_accounts(&[coin(uatom_acc_balance, "uatom"), coin(uosmo_acc_balance, "uosmo")], 3)
        .unwrap();
    let owner = &accs[0];
    let user = &accs[1];
    let recipient = &accs[2];

    let gamm = Gamm::new(&app);
    let pool_id = gamm
        .create_basic_pool(&[coin(20_000_000, "uatom"), coin(40_000_000, "uosmo")], owner)
        .unwrap()
        .data
        .pool_id;
    let pool_denom = format!("gamm/pool/{pool_id}");

    let contract_addr = instantiate_contract(&wasm, owner);

    let bank = Bank::new(&app);

    let user_pool_balance = query_balance(&bank, &user.address(), &pool_denom);
    assert_eq!(user_pool_balance, 0u128);

    let uatom_liquidity_amount = 5_000_000u128;
    let uosmo_liquidity_amount = 10_000_000u128;
    wasm.execute(
        &contract_addr,
        &ExecuteMsg::ProvideLiquidity {
            lp_token_out: pool_denom.clone(),
            recipient: None,
            minimum_receive: Uint128::one(),
        },
        &[coin(uatom_liquidity_amount, "uatom"), coin(uosmo_liquidity_amount, "uosmo")],
        user,
    )
    .unwrap();

    let user_pool_balance = query_balance(&bank, &user.address(), &pool_denom);
    assert_eq!(user_pool_balance, 25000000000000000000u128);
    let user_uatom_balance_before = query_balance(&bank, &user.address(), "uatom");
    let user_uosmo_balance_before = query_balance(&bank, &user.address(), "uosmo");

    let recipient_pool_balance = query_balance(&bank, &recipient.address(), &pool_denom);
    assert_eq!(recipient_pool_balance, 0u128);
    let recipient_uatom_balance_before = query_balance(&bank, &recipient.address(), "uatom");
    let recipient_uosmo_balance_before = query_balance(&bank, &recipient.address(), "uosmo");

    let estimate_coins: Vec<Coin> = wasm
        .query(
            &contract_addr,
            &QueryMsg::EstimateWithdrawLiquidity {
                coin_in: coin(user_pool_balance, &pool_denom),
            },
        )
        .unwrap();
    let uatom_estimate_amount =
        estimate_coins.iter().find(|c| c.denom == "uatom").unwrap().amount.u128();
    let uosmo_estimate_amount =
        estimate_coins.iter().find(|c| c.denom == "uosmo").unwrap().amount.u128();
    assert_eq!(uatom_estimate_amount, 4950000u128);
    assert_eq!(uosmo_estimate_amount, 9900000u128);

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::WithdrawLiquidity {
            recipient: Some(recipient.address()),
        },
        &[coin(user_pool_balance, &pool_denom)],
        user,
    )
    .unwrap();

    let contract_pool_balance = query_balance(&bank, &contract_addr, &pool_denom);
    assert_eq!(contract_pool_balance, 0u128);
    let contract_uatom_balance = query_balance(&bank, &contract_addr, "uatom");
    assert_eq!(contract_uatom_balance, 0u128);
    let contract_uosmo_balance = query_balance(&bank, &contract_addr, "uosmo");
    assert_eq!(contract_uosmo_balance, 0u128);

    let user_pool_balance = query_balance(&bank, &user.address(), &pool_denom);
    assert_eq!(user_pool_balance, 0u128);
    let user_uatom_balance = query_balance(&bank, &user.address(), "uatom");
    assert_eq!(user_uatom_balance, user_uatom_balance_before);
    let user_uosmo_balance = query_balance(&bank, &user.address(), "uosmo");
    assert_eq!(user_uosmo_balance, user_uosmo_balance_before);

    let recipient_pool_balance = query_balance(&bank, &recipient.address(), &pool_denom);
    assert_eq!(recipient_pool_balance, 0u128);
    let recipient_uatom_balance = query_balance(&bank, &recipient.address(), "uatom");
    assert_eq!(recipient_uatom_balance, recipient_uatom_balance_before + uatom_estimate_amount);
    let recipient_uosmo_balance = query_balance(&bank, &recipient.address(), "uosmo");
    assert_eq!(recipient_uosmo_balance, recipient_uosmo_balance_before + uosmo_estimate_amount);
}
