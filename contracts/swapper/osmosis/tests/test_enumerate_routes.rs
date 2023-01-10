extern crate core;

use std::collections::HashMap;

use cosmwasm_std::coin;
use mars_rover::adapters::swap::{ExecuteMsg, QueryMsg, RouteResponse};
use mars_swapper_osmosis::route::OsmosisRoute;
use osmosis_std::types::osmosis::gamm::v1beta1::SwapAmountInRoute;
use osmosis_testing::{Gamm, Module, OsmosisTestApp, SigningAccount, Wasm};

use crate::helpers::instantiate_contract;

pub mod helpers;

#[test]
fn test_enumerating_routes() {
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

    let routes = create_pools_and_routes(&app, &signer);

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::SetRoute {
            denom_in: "uatom".to_string(),
            denom_out: "umars".to_string(),
            route: routes.get(&("uatom", "umars")).unwrap().clone(),
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
            route: routes.get(&("uatom", "uusdc")).unwrap().clone(),
        },
        &[],
        &signer,
    )
    .unwrap();

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::SetRoute {
            denom_in: "uosmo".to_string(),
            denom_out: "umars".to_string(),
            route: routes.get(&("uosmo", "umars")).unwrap().clone(),
        },
        &[],
        &signer,
    )
    .unwrap();

    // NOTE: the response is ordered alphabetically
    let expected = vec![
        RouteResponse {
            denom_in: "uatom".to_string(),
            denom_out: "umars".to_string(),
            route: routes.get(&("uatom", "umars")).unwrap().clone(),
        },
        RouteResponse {
            denom_in: "uatom".to_string(),
            denom_out: "uusdc".to_string(),
            route: routes.get(&("uatom", "uusdc")).unwrap().clone(),
        },
        RouteResponse {
            denom_in: "uosmo".to_string(),
            denom_out: "umars".to_string(),
            route: routes.get(&("uosmo", "umars")).unwrap().clone(),
        },
    ];

    let res: Vec<RouteResponse<OsmosisRoute>> = wasm
        .query(
            &contract_addr,
            &QueryMsg::Routes {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(res, expected);

    let res: Vec<RouteResponse<OsmosisRoute>> = wasm
        .query(
            &contract_addr,
            &QueryMsg::Routes {
                start_after: None,
                limit: Some(1),
            },
        )
        .unwrap();
    assert_eq!(res, expected[..1]);

    let res: Vec<RouteResponse<OsmosisRoute>> = wasm
        .query(
            &contract_addr,
            &QueryMsg::Routes {
                start_after: Some(("uatom".to_string(), "uosmo".to_string())),
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(res, expected[1..]);
}

fn create_pools_and_routes(
    app: &OsmosisTestApp,
    signer: &SigningAccount,
) -> HashMap<(&'static str, &'static str), OsmosisRoute> {
    let gamm = Gamm::new(app);

    let pool_atom_osmo = gamm
        .create_basic_pool(&[coin(6_000_000, "uatom"), coin(1_500_000, "uosmo")], signer)
        .unwrap()
        .data
        .pool_id;
    let pool_osmo_mars = gamm
        .create_basic_pool(&[coin(100_000, "uosmo"), coin(1_000_000, "umars")], signer)
        .unwrap()
        .data
        .pool_id;
    let pool_osmo_usdc = gamm
        .create_basic_pool(&[coin(100_000, "uosmo"), coin(1_000_000, "uusdc")], signer)
        .unwrap()
        .data
        .pool_id;

    let mut map = HashMap::new();

    // uosmo -> umars
    map.insert(
        ("uosmo", "umars"),
        OsmosisRoute(vec![SwapAmountInRoute {
            pool_id: pool_osmo_mars,
            token_out_denom: "umars".to_string(),
        }]),
    );

    // uatom -> uosmo -> umars
    map.insert(
        ("uatom", "umars"),
        OsmosisRoute(vec![
            SwapAmountInRoute {
                pool_id: pool_atom_osmo,
                token_out_denom: "uosmo".to_string(),
            },
            SwapAmountInRoute {
                pool_id: pool_osmo_mars,
                token_out_denom: "umars".to_string(),
            },
        ]),
    );

    // uatom -> uosmo -> uusdc
    map.insert(
        ("uatom", "uusdc"),
        OsmosisRoute(vec![
            SwapAmountInRoute {
                pool_id: pool_atom_osmo,
                token_out_denom: "uosmo".to_string(),
            },
            SwapAmountInRoute {
                pool_id: pool_osmo_usdc,
                token_out_denom: "uusdc".to_string(),
            },
        ]),
    );

    map
}
