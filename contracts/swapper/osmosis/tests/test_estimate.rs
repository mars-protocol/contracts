use cosmwasm_std::{coin, Addr, StdError, StdResult, Uint128};
use cw_multi_test::Executor;
use osmo_bindings::Step;
use osmo_bindings_test::{Pool as OsmoPool, Pool};

use rover::adapters::swap::{EstimateExactInSwapResponse, ExecuteMsg, QueryMsg};
use swapper_osmosis::route::OsmosisRoute;

use crate::helpers::instantiate_contract;
use crate::helpers::mock_osmosis_app;

pub mod helpers;

#[test]
fn test_error_on_route_not_found() {
    let mut app = mock_osmosis_app();
    let contract_addr = instantiate_contract(&mut app);
    let res: StdResult<EstimateExactInSwapResponse> = app.wrap().query_wasm_smart(
        contract_addr,
        &QueryMsg::EstimateExactInSwap {
            coin_in: coin(1000, "jake"),
            denom_out: "mars".to_string(),
        },
    );

    match res {
        Ok(_) => panic!("should have thrown an error"),
        Err(err) => assert_eq!(
            err,
            StdError::generic_err(
                "Querier contract error: swapper_osmosis::route::OsmosisRoute not found"
            )
        ),
    }
}

#[test]
fn test_estimate_swap_one_step() {
    let mut app = mock_osmosis_app();

    let coin_a = coin(6_000_000, "osmo");
    let coin_b = coin(1_500_000, "atom");
    let pool_id = 43;
    let pool = OsmoPool::new(coin_a.clone(), coin_b.clone());

    app.init_modules(|router, _, storage| {
        router.custom.set_pool(storage, pool_id, &pool).unwrap();
    });

    let owner = Addr::unchecked("owner");
    let contract_addr = instantiate_contract(&mut app);

    app.execute_contract(
        owner,
        contract_addr.clone(),
        &ExecuteMsg::SetRoute {
            denom_in: "osmo".to_string(),
            denom_out: "atom".to_string(),
            route: OsmosisRoute {
                steps: vec![Step {
                    pool_id,
                    denom_out: "atom".to_string(),
                }],
            },
        },
        &[],
    )
    .unwrap();

    let res: EstimateExactInSwapResponse = app
        .wrap()
        .query_wasm_smart(
            contract_addr,
            &QueryMsg::EstimateExactInSwap {
                coin_in: coin(1000, coin_a.denom),
                denom_out: coin_b.denom,
            },
        )
        .unwrap();

    assert_eq!(res.amount, Uint128::new(250));
}

#[test]
fn test_estimate_swap_multi_step() {
    let mut app = mock_osmosis_app();

    let coin_a = coin(6_000_000, "uatom");
    let coin_b = coin(1_500_000, "uosmo");
    let pool_id_x = 1;
    let pool_x = Pool::new(coin_a.clone(), coin_b);

    let coin_c = coin(100_000, "uosmo");
    let coin_d = coin(1_000_000, "umars");
    let pool_id_y = 420;
    let pool_y = Pool::new(coin_c, coin_d);

    let coin_e = coin(100_000, "uosmo");
    let coin_f = coin(1_000_000, "uusdc");
    let pool_id_z = 69;
    let pool_z = Pool::new(coin_e, coin_f.clone());

    app.init_modules(|router, _, storage| {
        router.custom.set_pool(storage, pool_id_x, &pool_x).unwrap();
        router.custom.set_pool(storage, pool_id_y, &pool_y).unwrap();
        router.custom.set_pool(storage, pool_id_z, &pool_z).unwrap();
    });

    let owner = Addr::unchecked("owner");
    let contract_addr = instantiate_contract(&mut app);

    app.execute_contract(
        owner.clone(),
        contract_addr.clone(),
        &ExecuteMsg::SetRoute {
            denom_in: "uatom".to_string(),
            denom_out: "umars".to_string(),
            route: OsmosisRoute {
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
            route: OsmosisRoute {
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
            route: OsmosisRoute {
                steps: vec![Step {
                    pool_id: 420,
                    denom_out: "umars".to_string(),
                }],
            },
        },
        &[],
    )
    .unwrap();

    let res: EstimateExactInSwapResponse = app
        .wrap()
        .query_wasm_smart(
            contract_addr,
            &QueryMsg::EstimateExactInSwap {
                coin_in: coin(1000, coin_a.denom),
                denom_out: coin_f.denom,
            },
        )
        .unwrap();

    assert_eq!(res.amount, Uint128::new(2484));
}
