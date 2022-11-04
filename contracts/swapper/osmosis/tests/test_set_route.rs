use cosmwasm_std::coin;
use cosmwasm_std::StdError::GenericErr;
use osmosis_std::types::osmosis::gamm::v1beta1::SwapAmountInRoute;
use osmosis_testing::{Account, Gamm, Module, OsmosisTestApp, Wasm};

use mars_rover::adapters::swap::{ExecuteMsg, QueryMsg, RouteResponse};
use mars_rover::error::ContractError as RoverError;
use mars_swapper_base::ContractError;
use mars_swapper_osmosis::route::OsmosisRoute;

use crate::helpers::{assert_err, instantiate_contract};

pub mod helpers;

#[test]
fn test_only_owner_can_set_routes() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let accs = app
        .init_accounts(&[coin(1_000_000_000_000, "uosmo")], 2)
        .unwrap();
    let owner = &accs[0];
    let bad_guy = &accs[1];

    let contract_addr = instantiate_contract(&wasm, owner);

    let res_err = wasm
        .execute(
            &contract_addr,
            &ExecuteMsg::SetRoute {
                denom_in: "mars".to_string(),
                denom_out: "weth".to_string(),
                route: OsmosisRoute(vec![
                    SwapAmountInRoute {
                        pool_id: 1,
                        token_out_denom: "osmo".to_string(),
                    },
                    SwapAmountInRoute {
                        pool_id: 2,
                        token_out_denom: "weth".to_string(),
                    },
                ]),
            },
            &[],
            bad_guy,
        )
        .unwrap_err();

    assert_err(
        res_err,
        ContractError::Rover(RoverError::Unauthorized {
            user: bad_guy.address(),
            action: "set route".to_string(),
        }),
    );
}

#[test]
fn test_must_pass_at_least_one_step() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "uosmo")])
        .unwrap();

    let contract_addr = instantiate_contract(&wasm, &signer);

    let res_err = wasm
        .execute(
            &contract_addr,
            &ExecuteMsg::SetRoute {
                denom_in: "mars".to_string(),
                denom_out: "weth".to_string(),
                route: OsmosisRoute(vec![]),
            },
            &[],
            &signer,
        )
        .unwrap_err();

    assert_err(
        res_err,
        ContractError::InvalidRoute {
            reason: "the route must contain at least one step".to_string(),
        },
    );
}

#[test]
fn test_must_be_available_in_osmosis() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "uosmo")])
        .unwrap();

    let contract_addr = instantiate_contract(&wasm, &signer);

    let res_err = wasm
        .execute(
            &contract_addr,
            &ExecuteMsg::SetRoute {
                denom_in: "mars".to_string(),
                denom_out: "weth".to_string(),
                route: OsmosisRoute(vec![SwapAmountInRoute {
                    pool_id: 1,
                    token_out_denom: "osmo".to_string(),
                }]),
            },
            &[],
            &signer,
        )
        .unwrap_err();

    assert_err(
        res_err,
        ContractError::Std(GenericErr {
            msg: "Querier contract error".to_string(),
        }),
    );
}

#[test]
fn test_step_does_not_contain_input_denom() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[
            coin(1_000_000_000_000, "uatom"),
            coin(1_000_000_000_000, "uosmo"),
        ])
        .unwrap();

    let contract_addr = instantiate_contract(&wasm, &signer);

    let gamm = Gamm::new(&app);
    let pool_atom_osmo = gamm
        .create_basic_pool(
            &[coin(6_000_000, "uatom"), coin(1_500_000, "uosmo")],
            &signer,
        )
        .unwrap()
        .data
        .pool_id;

    let res_err = wasm
        .execute(
            &contract_addr,
            &ExecuteMsg::SetRoute {
                denom_in: "umars".to_string(),
                denom_out: "uweth".to_string(),
                route: OsmosisRoute(vec![SwapAmountInRoute {
                    pool_id: pool_atom_osmo,
                    token_out_denom: "uosmo".to_string(),
                }]),
            },
            &[],
            &signer,
        )
        .unwrap_err();

    assert_err(
        res_err,
        ContractError::InvalidRoute {
            reason: format!(
                "step 1: pool {} does not contain input denom umars",
                pool_atom_osmo
            ),
        },
    );
}

#[test]
fn test_step_does_not_contain_output_denom() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[
            coin(1_000_000_000_000, "umars"),
            coin(1_000_000_000_000, "uosmo"),
        ])
        .unwrap();

    let contract_addr = instantiate_contract(&wasm, &signer);

    let gamm = Gamm::new(&app);
    let pool_mars_osmo = gamm
        .create_basic_pool(
            &[coin(6_000_000, "umars"), coin(1_500_000, "uosmo")],
            &signer,
        )
        .unwrap()
        .data
        .pool_id;

    let res_err = wasm
        .execute(
            &contract_addr,
            &ExecuteMsg::SetRoute {
                denom_in: "umars".to_string(),
                denom_out: "uweth".to_string(),
                route: OsmosisRoute(vec![SwapAmountInRoute {
                    pool_id: pool_mars_osmo,
                    token_out_denom: "uweth".to_string(),
                }]),
            },
            &[],
            &signer,
        )
        .unwrap_err();

    assert_err(
        res_err,
        ContractError::InvalidRoute {
            reason: format!(
                "step 1: pool {} does not contain output denom uweth",
                pool_mars_osmo
            ),
        },
    );
}

#[test]
fn test_steps_do_not_loop() {
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
        .create_basic_pool(
            &[coin(6_000_000, "uatom"), coin(1_500_000, "uosmo")],
            &signer,
        )
        .unwrap()
        .data
        .pool_id;
    let pool_osmo_usdc = gamm
        .create_basic_pool(
            &[coin(6_000_000, "uosmo"), coin(1_500_000, "uusdc")],
            &signer,
        )
        .unwrap()
        .data
        .pool_id;
    let pool_osmo_mars = gamm
        .create_basic_pool(
            &[coin(6_000_000, "uosmo"), coin(1_500_000, "umars")],
            &signer,
        )
        .unwrap()
        .data
        .pool_id;

    let res_err = wasm
        .execute(
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
                        pool_id: pool_osmo_usdc,
                        token_out_denom: "uusdc".to_string(),
                    },
                    SwapAmountInRoute {
                        pool_id: pool_osmo_usdc,
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
        .unwrap_err();

    // invalid - route contains a loop
    // this example: ATOM -> OSMO -> USDC -> OSMO -> MARS
    assert_err(
        res_err,
        ContractError::InvalidRoute {
            reason: "route contains a loop: denom uosmo seen twice".to_string(),
        },
    );
}

#[test]
fn test_step_output_does_not_match() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[
            coin(1_000_000_000_000, "uatom"),
            coin(1_000_000_000_000, "uosmo"),
        ])
        .unwrap();

    let contract_addr = instantiate_contract(&wasm, &signer);

    let gamm = Gamm::new(&app);
    let pool_atom_osmo = gamm
        .create_basic_pool(
            &[coin(6_000_000, "uatom"), coin(1_500_000, "uosmo")],
            &signer,
        )
        .unwrap()
        .data
        .pool_id;

    let res_err = wasm
        .execute(
            &contract_addr,
            &ExecuteMsg::SetRoute {
                denom_in: "uatom".to_string(),
                denom_out: "umars".to_string(),
                route: OsmosisRoute(vec![SwapAmountInRoute {
                    pool_id: pool_atom_osmo,
                    token_out_denom: "uosmo".to_string(),
                }]),
            },
            &[],
            &signer,
        )
        .unwrap_err();

    assert_err(
        res_err,
        ContractError::InvalidRoute {
            reason: "the route's output denom uosmo does not match the desired output umars"
                .to_string(),
        },
    );
}

#[test]
fn test_set_route_success() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[
            coin(1_000_000_000_000, "uosmo"),
            coin(1_000_000_000_000, "umars"),
            coin(1_000_000_000_000, "uweth"),
        ])
        .unwrap();

    let contract_addr = instantiate_contract(&wasm, &signer);

    let gamm = Gamm::new(&app);
    let pool_mars_osmo = gamm
        .create_basic_pool(
            &[coin(6_000_000, "umars"), coin(1_500_000, "uosmo")],
            &signer,
        )
        .unwrap()
        .data
        .pool_id;
    let pool_weth_osmo = gamm
        .create_basic_pool(&[coin(100_000, "uweth"), coin(1_000_000, "uosmo")], &signer)
        .unwrap()
        .data
        .pool_id;

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::SetRoute {
            denom_in: "umars".to_string(),
            denom_out: "uweth".to_string(),
            route: OsmosisRoute(vec![
                SwapAmountInRoute {
                    pool_id: pool_mars_osmo,
                    token_out_denom: "uosmo".to_string(),
                },
                SwapAmountInRoute {
                    pool_id: pool_weth_osmo,
                    token_out_denom: "uweth".to_string(),
                },
            ]),
        },
        &[],
        &signer,
    )
    .unwrap();

    let res: RouteResponse<OsmosisRoute> = wasm
        .query(
            &contract_addr,
            &QueryMsg::Route {
                denom_in: "umars".to_string(),
                denom_out: "uweth".to_string(),
            },
        )
        .unwrap();

    assert_eq!(res.denom_in, "umars".to_string());
    assert_eq!(res.denom_out, "uweth".to_string());
    assert_eq!(
        res.route.to_string(),
        format!("{}:uosmo|{}:uweth", pool_mars_osmo, pool_weth_osmo)
    );
}
