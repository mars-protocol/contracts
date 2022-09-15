use cosmwasm_std::StdError::GenericErr;
use cosmwasm_std::{coin, Addr};
use cw_multi_test::Executor;
use osmo_bindings::Step;
use osmo_bindings_test::Pool;

use rover::adapters::swap::{ExecuteMsg, QueryMsg, RouteResponse};
use rover::error::ContractError as RoverError;
use swapper_base::ContractError;
use swapper_osmosis::route::OsmosisRoute;

use crate::helpers::mock_osmosis_app;
use crate::helpers::{assert_err, instantiate_contract};

pub mod helpers;

#[test]
fn test_only_owner_can_set_routes() {
    let mut app = mock_osmosis_app();
    let contract_addr = instantiate_contract(&mut app);

    let bad_guy = Addr::unchecked("bad_guy");
    let res = app.execute_contract(
        bad_guy.clone(),
        contract_addr,
        &ExecuteMsg::SetRoute {
            denom_in: "mars".to_string(),
            denom_out: "weth".to_string(),
            route: OsmosisRoute {
                steps: vec![
                    Step {
                        pool_id: 1,
                        denom_out: "osmo".to_string(),
                    },
                    Step {
                        pool_id: 2,
                        denom_out: "weth".to_string(),
                    },
                ],
            },
        },
        &[],
    );

    assert_err(
        res,
        ContractError::Rover(RoverError::Unauthorized {
            user: bad_guy.to_string(),
            action: "set route".to_string(),
        }),
    );
}

#[test]
fn test_must_pass_at_least_one_step() {
    let owner = Addr::unchecked("owner");
    let mut app = mock_osmosis_app();
    let contract_addr = instantiate_contract(&mut app);

    let res = app.execute_contract(
        owner,
        contract_addr,
        &ExecuteMsg::SetRoute {
            denom_in: "mars".to_string(),
            denom_out: "weth".to_string(),
            route: OsmosisRoute { steps: vec![] },
        },
        &[],
    );

    assert_err(
        res,
        ContractError::InvalidRoute {
            reason: "the route must contain at least one step".to_string(),
        },
    );
}

#[test]
fn test_must_be_available_in_osmosis() {
    let owner = Addr::unchecked("owner");
    let mut app = mock_osmosis_app();
    let contract_addr = instantiate_contract(&mut app);

    let res = app.execute_contract(
        owner,
        contract_addr,
        &ExecuteMsg::SetRoute {
            denom_in: "mars".to_string(),
            denom_out: "weth".to_string(),
            route: OsmosisRoute {
                steps: vec![Step {
                    pool_id: 1,
                    denom_out: "osmo".to_string(),
                }],
            },
        },
        &[],
    );

    assert_err(
        res,
        ContractError::Std(GenericErr {
            msg: "Querier contract error: osmo_bindings_test::multitest::Pool not found"
                .to_string(),
        }),
    );
}

#[test]
fn test_step_does_not_contain_input_denom() {
    let owner = Addr::unchecked("owner");
    let mut app = mock_osmosis_app();

    let coin_a = coin(6_000_000, "atom");
    let coin_b = coin(1_500_000, "osmo");
    let pool_id_x = 43;
    let pool_x = Pool::new(coin_a, coin_b);

    app.init_modules(|router, _, storage| {
        router.custom.set_pool(storage, pool_id_x, &pool_x).unwrap();
    });

    let contract_addr = instantiate_contract(&mut app);

    let res = app.execute_contract(
        owner,
        contract_addr,
        &ExecuteMsg::SetRoute {
            denom_in: "mars".to_string(),
            denom_out: "weth".to_string(),
            route: OsmosisRoute {
                steps: vec![Step {
                    pool_id: pool_id_x,
                    denom_out: "osmo".to_string(),
                }],
            },
        },
        &[],
    );

    assert_err(
        res,
        ContractError::InvalidRoute {
            reason: "step 1: pool 43 does not contain input denom mars".to_string(),
        },
    );
}

#[test]
fn test_step_does_not_contain_output_denom() {
    let owner = Addr::unchecked("owner");
    let mut app = mock_osmosis_app();

    let coin_a = coin(6_000_000, "mars");
    let coin_b = coin(1_500_000, "osmo");
    let pool_id_x = 43;
    let pool_x = Pool::new(coin_a, coin_b);

    app.init_modules(|router, _, storage| {
        router.custom.set_pool(storage, pool_id_x, &pool_x).unwrap();
    });

    let contract_addr = instantiate_contract(&mut app);

    let res = app.execute_contract(
        owner,
        contract_addr,
        &ExecuteMsg::SetRoute {
            denom_in: "mars".to_string(),
            denom_out: "weth".to_string(),
            route: OsmosisRoute {
                steps: vec![Step {
                    pool_id: pool_id_x,
                    denom_out: "weth".to_string(),
                }],
            },
        },
        &[],
    );

    assert_err(
        res,
        ContractError::InvalidRoute {
            reason: "step 1: pool 43 does not contain output denom weth".to_string(),
        },
    );
}

#[test]
fn test_steps_do_not_loop() {
    let owner = Addr::unchecked("owner");
    let mut app = mock_osmosis_app();

    let coin_a = coin(6_000_000, "atom");
    let coin_b = coin(1_500_000, "osmo");
    let pool_id_x = 43;
    let pool_x = Pool::new(coin_a, coin_b);

    let coin_c = coin(6_000_000, "osmo");
    let coin_d = coin(1_500_000, "usdc");
    let pool_id_y = 101;
    let pool_y = Pool::new(coin_c, coin_d);

    let coin_e = coin(6_000_000, "osmo");
    let coin_f = coin(1_500_000, "mars");
    let pool_id_z = 2;
    let pool_z = Pool::new(coin_e, coin_f);

    app.init_modules(|router, _, storage| {
        router.custom.set_pool(storage, pool_id_x, &pool_x).unwrap();
        router.custom.set_pool(storage, pool_id_y, &pool_y).unwrap();
        router.custom.set_pool(storage, pool_id_z, &pool_z).unwrap();
    });

    let contract_addr = instantiate_contract(&mut app);

    let res = app.execute_contract(
        owner,
        contract_addr,
        &ExecuteMsg::SetRoute {
            denom_in: "atom".to_string(),
            denom_out: "mars".to_string(),
            route: OsmosisRoute {
                steps: vec![
                    Step {
                        pool_id: pool_id_x,
                        denom_out: "osmo".to_string(),
                    },
                    Step {
                        pool_id: pool_id_y,
                        denom_out: "usdc".to_string(),
                    },
                    Step {
                        pool_id: pool_id_y,
                        denom_out: "osmo".to_string(),
                    },
                    Step {
                        pool_id: pool_id_z,
                        denom_out: "mars".to_string(),
                    },
                ],
            },
        },
        &[],
    );

    // invalid - route contains a loop
    // this example: ATOM -> OSMO -> USDC -> OSMO -> MARS
    assert_err(
        res,
        ContractError::InvalidRoute {
            reason: "route contains a loop: denom osmo seen twice".to_string(),
        },
    );
}

#[test]
fn test_step_output_does_not_match() {
    let owner = Addr::unchecked("owner");
    let mut app = mock_osmosis_app();

    let coin_a = coin(6_000_000, "atom");
    let coin_b = coin(1_500_000, "osmo");
    let pool_id_x = 43;
    let pool_x = Pool::new(coin_a, coin_b);

    app.init_modules(|router, _, storage| {
        router.custom.set_pool(storage, pool_id_x, &pool_x).unwrap();
    });

    let contract_addr = instantiate_contract(&mut app);

    let res = app.execute_contract(
        owner,
        contract_addr,
        &ExecuteMsg::SetRoute {
            denom_in: "atom".to_string(),
            denom_out: "mars".to_string(),
            route: OsmosisRoute {
                steps: vec![Step {
                    pool_id: pool_id_x,
                    denom_out: "osmo".to_string(),
                }],
            },
        },
        &[],
    );

    assert_err(
        res,
        ContractError::InvalidRoute {
            reason: "the route's output denom osmo does not match the desired output mars"
                .to_string(),
        },
    );
}

#[test]
fn test_set_route_success() {
    let owner = Addr::unchecked("owner");
    let mut app = mock_osmosis_app();
    let contract_addr = instantiate_contract(&mut app);

    let coin_a = coin(6_000_000, "mars");
    let coin_b = coin(1_500_000, "osmo");
    let pool_id_x = 43;
    let pool_x = Pool::new(coin_a, coin_b);

    let coin_c = coin(100_000, "weth");
    let coin_d = coin(1_000_000, "osmo");
    let pool_id_y = 15;
    let pool_y = Pool::new(coin_c, coin_d);

    app.init_modules(|router, _, storage| {
        router.custom.set_pool(storage, pool_id_x, &pool_x).unwrap();
        router.custom.set_pool(storage, pool_id_y, &pool_y).unwrap();
    });

    app.execute_contract(
        owner,
        contract_addr.clone(),
        &ExecuteMsg::SetRoute {
            denom_in: "mars".to_string(),
            denom_out: "weth".to_string(),
            route: OsmosisRoute {
                steps: vec![
                    Step {
                        pool_id: pool_id_x,
                        denom_out: "osmo".to_string(),
                    },
                    Step {
                        pool_id: pool_id_y,
                        denom_out: "weth".to_string(),
                    },
                ],
            },
        },
        &[],
    )
    .unwrap();

    let res: RouteResponse<OsmosisRoute> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.to_string(),
            &QueryMsg::Route {
                denom_in: "mars".to_string(),
                denom_out: "weth".to_string(),
            },
        )
        .unwrap();

    assert_eq!(res.denom_in, "mars".to_string());
    assert_eq!(res.denom_out, "weth".to_string());
    assert_eq!(res.route.to_string(), "43:osmo|15:weth".to_string());
}
