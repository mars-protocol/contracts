use crate::helpers::osmosis::instantiate_contract;
use cosmwasm_std::{coin, Decimal, Isqrt, Uint128};
use mars_oracle_osmosis::OsmosisPriceSource;
use mars_outpost::oracle::{ExecuteMsg, InstantiateMsg, PriceResponse, QueryMsg};
use osmosis_testing::{Account, Gamm, Module, OsmosisTestApp, Wasm};
use std::str::FromStr;

mod helpers;

const OSMOSIS_ORACLE_CONTRACT_NAME: &str = "mars-oracle-osmosis";

#[test]
fn querying_xyk_lp_price_if_no_price_for_tokens() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[
            coin(1_000_000_000_000, "uosmo"),
            coin(1_000_000_000_000, "umars"),
            coin(1_000_000_000_000, "uatom"),
        ])
        .unwrap();

    let contract_addr = instantiate_contract(
        &wasm,
        &signer,
        OSMOSIS_ORACLE_CONTRACT_NAME,
        &InstantiateMsg {
            owner: signer.address(),
            base_denom: "uosmo".to_string(),
        },
    );

    let gamm = Gamm::new(&app);
    let pool_mars_atom = gamm
        .create_basic_pool(&[coin(31_500_000, "umars"), coin(1_500_000, "uatom")], &signer)
        .unwrap()
        .data
        .pool_id;

use osmosis_std::types::osmosis::gamm::v1beta1::{Pool, QueryPoolResponse, QuerySpotPriceResponse};
use osmosis_std::types::osmosis::twap::v1beta1::ArithmeticTwapToNowResponse;

use mars_outpost::oracle::InstantiateMsg;

#[test]
fn spot_test() {
    let app = OsmosisTestApp::new();

    let accs = app
        .init_accounts(
            &[Coin::new(1_000_000_000_000, "uatom"), Coin::new(1_000_000_000_000, "uosmo")],
            2,
        )
        .unwrap();

    let user_a = &accs[0];
    let user_b = &accs[1];
}

#[test]
#[ignore] // FIXME: TWAP doesn't work on osmosis-testing - fix in progress
          //assert oracle was correctly set to TWAP and assert prices are queried correctly

fn set_twap_price() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "uosmo"), coin(1_000_000_000_000, "uatom")])
        .unwrap();

    //compile and instantiate oracle contract
    let wasm_byte_code = std::fs::read("../artifacts/mars_oracle_osmosis.wasm").unwrap();
    let code_id = wasm.store_code(&wasm_byte_code, None, &signer).unwrap().data.code_id;
    let oracle_addr = wasm
        .instantiate(
            code_id,
            &InstantiateMsg {
                owner: signer.address(),
                base_denom: "uosmo".to_string(),
            },
            None,
            None,
            &[],
            &signer,
        )
        .unwrap()
        .data
        .address;

    //set up osmo-atom pool
    let gamm = Gamm::new(&app);
    let pool_liquidity = vec![Coin::new(1_000_000, "uatom"), Coin::new(1_000_000, "uosmo")];
    let pool_id = gamm.create_basic_pool(&pool_liquidity, &signer).unwrap().data.pool_id;

    let osmo_atom_pool = gamm.query_pool(pool_id).unwrap().pool_assets;

    assert_eq!(
        pool_liquidity
            .into_iter()
            .map(|c| c.into())
            .collect::<Vec<osmosis_testing::osmosis_std::types::cosmos::base::v1beta1::Coin>>(),
        osmo_atom_pool
            .into_iter()
            .map(|a| a.token.unwrap())
            .collect::<Vec<osmosis_testing::osmosis_std::types::cosmos::base::v1beta1::Coin>>(),
    );

    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::SetPriceSource {
            denom: "uosmo".to_string(),
            price_source: OsmosisPriceSource::Twap {
                pool_id,
                window_size: 1800, //30min
            },
        },
        &[],
        &signer,
    )
    .unwrap();

    let price_source: PriceSourceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::PriceSource {
                denom: "uosmo".to_string(),
            },
        )
        .unwrap();

    assert_eq!(
        price_source.price_source,
        (OsmosisPriceSource::Twap {
            pool_id,
            window_size: 1800,
        })
    );

    let price: PriceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::Price {
                denom: "uosmo".to_string(),
            },
        )
        .unwrap();

    assert_eq!(price.price, Decimal::one()); //why does this work??

    assert_eq!(price.denom, "uosmo".to_string());
}

#[test]
fn test_redbank_oracle() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "uosmo"), coin(1_000_000_000_000, "uatom")])
        .unwrap();

    //compile and instantiate redbank contract
    let wasm_byte_code = std::fs::read("../artifacts/mars_red_bank.wasm").unwrap();
    let code_id = wasm.store_code(&wasm_byte_code, None, &signer).unwrap().data.code_id;
    let oracle_addr = wasm
        .instantiate(
            code_id,
            &InstantiateMsg {
                owner: signer.address(),
                base_denom: "uosmo".to_string(),
            },
            None,
            None,
            &[],
            &signer,
        )
        .unwrap()
        .data
        .address;

    //compile and instantiate oracle contract
    let wasm_byte_code = std::fs::read("../artifacts/mars_oracle_osmosis.wasm").unwrap();
    let code_id = wasm.store_code(&wasm_byte_code, None, &signer).unwrap().data.code_id;
    let oracle_addr = wasm
        .instantiate(
            code_id,
            &InstantiateMsg {
                owner: signer.address(),
                base_denom: "uosmo".to_string(),
            },
            None,
            None,
            &[],
            &signer,
        )
        .unwrap()
        .data
        .address;

    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::SetPriceSource {
            denom: "uosmo".to_string(),
            price_source: OsmosisPriceSource::Spot {
                pool_id,
            },
        },
        &[],
        &signer,
    )
    .unwrap();

    let price_source: PriceSourceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::PriceSource {
                denom: "uosmo".to_string(),
            },
        )
        .unwrap();

    assert_eq!(
        price_source.price_source,
        (OsmosisPriceSource::Spot {
            pool_id
        })
    );

    //add in red bank actions and test for oracle error msg
}
