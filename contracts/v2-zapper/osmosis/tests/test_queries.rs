use std::{ops::Div, str::FromStr};

use cosmwasm_std::{coin, Coin, Uint128};
use cw_dex::CwDexError;
use mars_v2_zapper_base::QueryMsg;
use osmosis_test_tube::{Gamm, Module, OsmosisTestApp, Wasm};

use crate::helpers::{assert_err, instantiate_contract};

pub mod helpers;

#[test]
fn estimate_provide_liquidity_with_invalid_lp_token() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app.init_account(&[coin(1_000_000_000_000, "uosmo")]).unwrap();

    let contract_addr = instantiate_contract(&wasm, &signer);

    let res_err = wasm
        .query::<QueryMsg, Uint128>(
            &contract_addr,
            &QueryMsg::EstimateProvideLiquidity {
                lp_token_out: "INVALID_POOL".to_string(),
                coins_in: vec![coin(500_000, "uatom"), coin(2_000_000, "uosmo")],
            },
        )
        .unwrap_err();
    assert_err(res_err, CwDexError::NotLpToken {});
}

#[test]
fn estimate_provide_liquidity_with_invalid_coins() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "uatom"), coin(1_000_000_000_000, "uosmo")])
        .unwrap();

    let gamm = Gamm::new(&app);
    let pool_id = gamm
        .create_basic_pool(&[coin(2_000_000, "uatom"), coin(4_000_000, "uosmo")], &signer)
        .unwrap()
        .data
        .pool_id;

    let pool = gamm.query_pool(pool_id).unwrap();
    let lp_token = pool.total_shares.unwrap().denom;
    assert_eq!(lp_token, "gamm/pool/1".to_string());

    let contract_addr = instantiate_contract(&wasm, &signer);

    // Generic error: Querier contract error: codespace: undefined, code: 1: execute wasm contract failed
    wasm.query::<QueryMsg, Uint128>(
        &contract_addr,
        &QueryMsg::EstimateProvideLiquidity {
            lp_token_out: lp_token,
            coins_in: vec![],
        },
    )
    .unwrap_err();
}

#[test]
fn estimate_provide_liquidity_successfully() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "uatom"), coin(1_000_000_000_000, "uosmo")])
        .unwrap();

    let gamm = Gamm::new(&app);
    let pool_id = gamm
        .create_basic_pool(&[coin(2_000_000, "uatom"), coin(4_000_000, "uosmo")], &signer)
        .unwrap()
        .data
        .pool_id;

    let pool = gamm.query_pool(pool_id).unwrap();
    let total_shares = pool.total_shares.unwrap();
    let total_amount = Uint128::from_str(&total_shares.amount).unwrap();
    assert_eq!(total_amount, Uint128::from(100000000000000000000u128));

    let contract_addr = instantiate_contract(&wasm, &signer);

    let amount: Uint128 = wasm
        .query(
            &contract_addr,
            &QueryMsg::EstimateProvideLiquidity {
                lp_token_out: total_shares.denom,
                coins_in: vec![coin(1_000_000, "uatom"), coin(2_000_000, "uosmo")],
            },
        )
        .unwrap();
    let expected_amount = total_amount.div(Uint128::from(2u8));
    assert_eq!(amount, expected_amount);
}

#[test]
fn estimate_withdraw_liquidity_with_invalid_lp_token() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app.init_account(&[coin(1_000_000_000_000, "uosmo")]).unwrap();

    let contract_addr = instantiate_contract(&wasm, &signer);

    let res_err = wasm
        .query::<QueryMsg, Uint128>(
            &contract_addr,
            &QueryMsg::EstimateWithdrawLiquidity {
                coin_in: coin(500_000, "INVALID_POOL"),
            },
        )
        .unwrap_err();
    assert_err(res_err, CwDexError::NotLpToken {});
}

#[test]
fn estimate_withdraw_liquidity_successfully() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "uatom"), coin(1_000_000_000_000, "uosmo")])
        .unwrap();

    let gamm = Gamm::new(&app);
    let pool_id = gamm
        .create_basic_pool(&[coin(2_000_000, "uatom"), coin(4_000_000, "uosmo")], &signer)
        .unwrap()
        .data
        .pool_id;

    let pool = gamm.query_pool(pool_id).unwrap();
    let total_shares = pool.total_shares.unwrap();
    let total_amount = Uint128::from_str(&total_shares.amount).unwrap();
    assert_eq!(total_amount, Uint128::from(100000000000000000000u128));

    let contract_addr = instantiate_contract(&wasm, &signer);

    let withdraw_amount = total_amount.div(Uint128::from(2u8));
    let coins: Vec<Coin> = wasm
        .query(
            &contract_addr,
            &QueryMsg::EstimateWithdrawLiquidity {
                coin_in: coin(withdraw_amount.u128(), total_shares.denom),
            },
        )
        .unwrap();
    assert_eq!(coins, vec![coin(1000000, "uatom"), coin(2000000, "uosmo")])
}
