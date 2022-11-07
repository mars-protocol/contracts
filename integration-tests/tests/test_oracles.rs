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

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::SetPriceSource {
            denom: "umars_uatom_lp".to_string(),
            price_source: OsmosisPriceSource::XykLiquidityToken {
                pool_id: pool_mars_atom,
            },
        },
        &[],
        &signer,
    )
    .unwrap();

    // Should fail - missing price for umars and uatom
    wasm.query::<QueryMsg, PriceResponse>(
        &contract_addr,
        &QueryMsg::Price {
            denom: "umars_uatom_lp".to_string(),
        },
    )
    .unwrap_err();
}

#[test]
fn querying_xyk_lp_price_success() {
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
        .create_basic_pool(
            &[coin(31_500_000, "umars"), coin(1_500_000, "uatom")], // 1 atom = 21 mars
            &signer,
        )
        .unwrap()
        .data
        .pool_id;
    let pool_mars_osmo = gamm
        .create_basic_pool(
            &[coin(6_000_000, "umars"), coin(1_500_000, "uosmo")], // 1 mars = 0.25 osmo
            &signer,
        )
        .unwrap()
        .data
        .pool_id;
    let pool_atom_osmo = gamm
        .create_basic_pool(
            &[coin(1_000_000, "uatom"), coin(10_000_000, "uosmo")], // 1 atom = 10 osmo
            &signer,
        )
        .unwrap()
        .data
        .pool_id;

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::SetPriceSource {
            denom: "umars_uatom_lp".to_string(),
            price_source: OsmosisPriceSource::XykLiquidityToken {
                pool_id: pool_mars_atom,
            },
        },
        &[],
        &signer,
    )
    .unwrap();
    wasm.execute(
        &contract_addr,
        &ExecuteMsg::SetPriceSource {
            denom: "umars".to_string(),
            price_source: OsmosisPriceSource::Spot {
                pool_id: pool_mars_osmo,
            },
        },
        &[],
        &signer,
    )
    .unwrap();
    wasm.execute(
        &contract_addr,
        &ExecuteMsg::SetPriceSource {
            denom: "uatom".to_string(),
            price_source: OsmosisPriceSource::Spot {
                pool_id: pool_atom_osmo,
            },
        },
        &[],
        &signer,
    )
    .unwrap();

    // Mars price: 0.25 osmo
    // Mars depth: 31_500_000
    // Atom price: 10 osmo
    // Atom depth: 1_500_000
    // pool value: 2 * sqrt((0.25 * 31_500_000) * (10 * 1_500_000))
    // LP token price: pool value / lp shares
    let mars_price = Decimal::from_ratio(1u128, 4u128);
    let atom_price = Decimal::from_ratio(10u128, 1u128);
    let mars_value = Uint128::from(31_500_000u128) * mars_price;
    let atom_value = Uint128::from(1_500_000u128) * atom_price;
    let pool_value = Uint128::from(2u8) * (mars_value * atom_value).isqrt();
    let pool = gamm.query_pool(pool_mars_atom).unwrap();
    let mars_atom_lp_shares = Uint128::from_str(&pool.total_shares.unwrap().amount).unwrap();
    let lp_price = Decimal::from_ratio(pool_value, mars_atom_lp_shares);
    let res: PriceResponse = wasm
        .query(
            &contract_addr,
            &QueryMsg::Price {
                denom: "umars_uatom_lp".to_string(),
            },
        )
        .unwrap();
    assert_eq!(res.price, lp_price);
}
