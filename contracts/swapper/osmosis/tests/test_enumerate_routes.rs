use crate::helpers::{instantiate_contract, mock_osmosis_app};
use cosmwasm_std::{coin, Addr};
use cw_multi_test::Executor;
use osmo_bindings::Step;
use osmo_bindings_test::Pool;
use rover::adapters::swap::{ExecuteMsg, QueryMsg, RouteResponse};
use std::collections::HashMap;
use swapper_osmosis::route::OsmosisRoute;

pub mod helpers;

#[test]
fn test_enumerating_routes() {
    let owner = Addr::unchecked("owner");
    let mut app = mock_osmosis_app();
    let contract_addr = instantiate_contract(&mut app);

    let coin_a = coin(6_000_000, "uatom");
    let coin_b = coin(1_500_000, "uosmo");
    let pool_id_x = 1;
    let pool_x = Pool::new(coin_a, coin_b);

    let coin_c = coin(100_000, "uosmo");
    let coin_d = coin(1_000_000, "umars");
    let pool_id_y = 420;
    let pool_y = Pool::new(coin_c, coin_d);

    let coin_e = coin(100_000, "uosmo");
    let coin_f = coin(1_000_000, "uusdc");
    let pool_id_z = 69;
    let pool_z = Pool::new(coin_e, coin_f);

    app.init_modules(|router, _, storage| {
        router.custom.set_pool(storage, pool_id_x, &pool_x).unwrap();
        router.custom.set_pool(storage, pool_id_y, &pool_y).unwrap();
        router.custom.set_pool(storage, pool_id_z, &pool_z).unwrap();
    });

    let routes = mock_routes();

    app.execute_contract(
        owner.clone(),
        contract_addr.clone(),
        &ExecuteMsg::SetRoute {
            denom_in: "uatom".to_string(),
            denom_out: "umars".to_string(),
            route: routes.get(&("uatom", "umars")).unwrap().clone(),
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        owner.clone(),
        contract_addr.clone(),
        &ExecuteMsg::SetRoute {
            denom_in: "uatom".to_string(),
            denom_out: "uusdc".to_string(),
            route: routes.get(&("uatom", "uusdc")).unwrap().clone(),
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        owner,
        contract_addr.clone(),
        &ExecuteMsg::SetRoute {
            denom_in: "uosmo".to_string(),
            denom_out: "umars".to_string(),
            route: routes.get(&("uosmo", "umars")).unwrap().clone(),
        },
        &[],
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

    let res: Vec<RouteResponse<OsmosisRoute>> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.to_string(),
            &QueryMsg::Routes {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(res, expected);

    let res: Vec<RouteResponse<OsmosisRoute>> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.to_string(),
            &QueryMsg::Routes {
                start_after: None,
                limit: Some(1),
            },
        )
        .unwrap();
    assert_eq!(res, expected[..1]);

    let res: Vec<RouteResponse<OsmosisRoute>> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.to_string(),
            &QueryMsg::Routes {
                start_after: Some(("uatom".to_string(), "uosmo".to_string())),
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(res, expected[1..]);
}

fn mock_routes() -> HashMap<(&'static str, &'static str), OsmosisRoute> {
    let mut map = HashMap::new();

    // uosmo -> umars
    map.insert(
        ("uosmo", "umars"),
        OsmosisRoute {
            steps: vec![Step {
                pool_id: 420,
                denom_out: "umars".to_string(),
            }],
        },
    );

    // uatom -> uosmo -> umars
    map.insert(
        ("uatom", "umars"),
        OsmosisRoute {
            steps: vec![
                Step {
                    pool_id: 1,
                    denom_out: "uosmo".to_string(),
                },
                Step {
                    pool_id: 420,
                    denom_out: "umars".to_string(),
                },
            ],
        },
    );

    // uatom -> uosmo -> uusdc
    map.insert(
        ("uatom", "uusdc"),
        OsmosisRoute {
            steps: vec![
                Step {
                    pool_id: 1,
                    denom_out: "uosmo".to_string(),
                },
                Step {
                    pool_id: 69,
                    denom_out: "uusdc".to_string(),
                },
            ],
        },
    );

    map
}
