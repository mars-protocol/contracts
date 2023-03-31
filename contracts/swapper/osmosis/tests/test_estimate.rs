use cosmwasm_std::{coin, Uint128};
use mars_rover::adapters::swap::{EstimateExactInSwapResponse, ExecuteMsg, QueryMsg};
use mars_swapper_osmosis::route::OsmosisRoute;
use osmosis_std::types::osmosis::poolmanager::v1beta1::SwapAmountInRoute;
use osmosis_test_tube::{Gamm, Module, OsmosisTestApp, RunnerResult, Wasm};

use crate::helpers::{
    assert_err, instantiate_contract, query_price_from_pool, swap_to_create_twap_records,
};

pub mod helpers;

#[test]
fn error_on_route_not_found() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);
    let owner = app.init_account(&[coin(1_000_000_000_000, "uosmo")]).unwrap();

    let contract_addr = instantiate_contract(&wasm, &owner);

    let res: RunnerResult<EstimateExactInSwapResponse> = wasm.query(
        &contract_addr,
        &QueryMsg::EstimateExactInSwap {
            coin_in: coin(1000, "jake"),
            denom_out: "mars".to_string(),
        },
    );

    assert_err(res.unwrap_err(), "swapper_osmosis::route::OsmosisRoute not found");
}

#[test]
fn estimate_swap_one_step() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "uatom"), coin(1_000_000_000_000, "uosmo")])
        .unwrap();

    let contract_addr = instantiate_contract(&wasm, &signer);

    let gamm = Gamm::new(&app);
    let pool_atom_osmo = gamm
        .create_basic_pool(&[coin(1_500_000, "uatom"), coin(6_000_000, "uosmo")], &signer)
        .unwrap()
        .data
        .pool_id;

    swap_to_create_twap_records(&app, &signer, pool_atom_osmo, coin(10u128, "uatom"), "uosmo");

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::SetRoute {
            denom_in: "uosmo".to_string(),
            denom_out: "uatom".to_string(),
            route: OsmosisRoute(vec![SwapAmountInRoute {
                pool_id: pool_atom_osmo,
                token_out_denom: "uatom".to_string(),
            }]),
        },
        &[],
        &signer,
    )
    .unwrap();

    let coin_in_amount = Uint128::from(1000u128);
    let uosmo_price = query_price_from_pool(&gamm, pool_atom_osmo, "uosmo");
    let expected_output = coin_in_amount * uosmo_price;

    let res: EstimateExactInSwapResponse = wasm
        .query(
            &contract_addr,
            &QueryMsg::EstimateExactInSwap {
                coin_in: coin(coin_in_amount.u128(), "uosmo"),
                denom_out: "uatom".to_string(),
            },
        )
        .unwrap();
    assert_eq!(res.amount, expected_output);
}

#[test]
fn estimate_swap_multi_step() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[
            coin(1_000_000_000_000, "uatom"),
            coin(1_000_000_000_000, "uosmo"),
            coin(1_000_000_000_000, "umars"),
            coin(1_000_000_000_000, "uusdc"),
        ])
        .unwrap();

    let contract_addr = instantiate_contract(&wasm, &signer);

    let gamm = Gamm::new(&app);
    let pool_atom_osmo = gamm
        .create_basic_pool(&[coin(6_000_000, "uatom"), coin(1_500_000, "uosmo")], &signer)
        .unwrap()
        .data
        .pool_id;
    let pool_osmo_mars = gamm
        .create_basic_pool(&[coin(100_000, "uosmo"), coin(1_000_000, "umars")], &signer)
        .unwrap()
        .data
        .pool_id;
    let pool_osmo_usdc = gamm
        .create_basic_pool(&[coin(100_000, "uosmo"), coin(1_000_000, "uusdc")], &signer)
        .unwrap()
        .data
        .pool_id;

    swap_to_create_twap_records(&app, &signer, pool_atom_osmo, coin(4u128, "uosmo"), "uatom");

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::SetRoute {
            denom_in: "uatom".to_string(),
            denom_out: "umars".to_string(),
            route: OsmosisRoute(vec![
                SwapAmountInRoute {
                    pool_id: pool_atom_osmo,
                    token_out_denom: "uosmo".to_string(),
                },
                SwapAmountInRoute {
                    pool_id: pool_osmo_mars,
                    token_out_denom: "umars".to_string(),
                },
            ]),
        },
        &[],
        &signer,
    )
    .unwrap();

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::SetRoute {
            denom_in: "uatom".to_string(),
            denom_out: "uusdc".to_string(),
            route: OsmosisRoute(vec![
                SwapAmountInRoute {
                    pool_id: pool_atom_osmo,
                    token_out_denom: "uosmo".to_string(),
                },
                SwapAmountInRoute {
                    pool_id: pool_osmo_usdc,
                    token_out_denom: "uusdc".to_string(),
                },
            ]),
        },
        &[],
        &signer,
    )
    .unwrap();

    let coin_in_amount = Uint128::from(1000u128);
    let uatom_price = query_price_from_pool(&gamm, pool_atom_osmo, "uatom");
    let uosmo_price = query_price_from_pool(&gamm, pool_osmo_usdc, "uosmo");
    let expected_output = coin_in_amount * uatom_price * uosmo_price;

    // atom/usdc = (price for atom/osmo) * (price for osmo/usdc)
    // usdc_out_amount = (atom amount) * (price for atom/usdc)
    //
    // 1 osmo = 4 atom => atom/osmo = 0.25
    // 1 osmo = 10 usdc => osmo/usdc = 10
    //
    // atom/usdc = 0.25 * 10 = 2.5
    // usdc_out_amount = 1000 * 2.5 = 2500
    let res: EstimateExactInSwapResponse = wasm
        .query(
            &contract_addr,
            &QueryMsg::EstimateExactInSwap {
                coin_in: coin(coin_in_amount.u128(), "uatom"),
                denom_out: "uusdc".to_string(),
            },
        )
        .unwrap();
    assert_eq!(res.amount, expected_output);
}
