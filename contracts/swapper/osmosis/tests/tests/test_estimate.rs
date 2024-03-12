use std::{marker::PhantomData, str::FromStr};

use cosmwasm_std::{
    coin, from_json,
    testing::{mock_env, MockApi, MockQuerier, MockStorage},
    to_json_vec, Decimal, OwnedDeps, Uint128,
};
use cw_it::osmosis_test_tube::{Gamm, Module, OsmosisTestApp, RunnerResult, Wasm};
use mars_osmosis::ConcentratedLiquidityPool;
use mars_swapper_osmosis::{
    config::OsmosisConfig,
    contract::{instantiate, query},
    route::{OsmosisRoute, SwapAmountInRoute},
};
use mars_testing::{mock_info, MarsMockQuerier};
use mars_types::swapper::{
    EstimateExactInSwapResponse, ExecuteMsg, InstantiateMsg, OsmoRoute, OsmoSwap, QueryMsg,
    SwapperRoute,
};
use osmosis_std::types::osmosis::{
    cosmwasmpool::v1beta1::{CosmWasmPool, InstantiateMsg as CosmwasmPoolInstantiateMsg},
    poolmanager::v1beta1::PoolResponse,
    twap::v1beta1::ArithmeticTwapToNowResponse,
};

use super::helpers::{
    assert_err, instantiate_contract, query_price_from_pool, swap_to_create_twap_records,
};

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
            route: None,
        },
    );
    let err = res.unwrap_err();

    assert_err(err, "No route found from jake to mars");
}

#[test]
fn estimate_swap_with_saved_route() {
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
        &ExecuteMsg::<OsmosisRoute, OsmosisConfig>::SetRoute {
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
                route: None,
            },
        )
        .unwrap();
    assert_eq!(res.amount, expected_output);
}

#[test]
fn estimate_swap_with_provided_route() {
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

    let coin_in_amount = Uint128::from(1000u128);
    let uosmo_price = query_price_from_pool(&gamm, pool_atom_osmo, "uosmo");
    let expected_output = coin_in_amount * uosmo_price;

    let res: EstimateExactInSwapResponse = wasm
        .query(
            &contract_addr,
            &QueryMsg::EstimateExactInSwap {
                coin_in: coin(coin_in_amount.u128(), "uosmo"),
                denom_out: "uatom".to_string(),
                route: Some(SwapperRoute::Osmo(OsmoRoute {
                    swaps: vec![OsmoSwap {
                        pool_id: pool_atom_osmo,
                        to: "uatom".to_string(),
                    }],
                })),
            },
        )
        .unwrap();
    assert_eq!(res.amount, expected_output);
}

#[test]
fn estimate_swap_with_provided_route_when_saved_route_exists() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "uatom"), coin(1_000_000_000_000, "uosmo")])
        .unwrap();

    let contract_addr = instantiate_contract(&wasm, &signer);

    let gamm = Gamm::new(&app);
    let pool_atom_osmo_saved = gamm
        .create_basic_pool(&[coin(1_500_000, "uatom"), coin(6_000_000, "uosmo")], &signer)
        .unwrap()
        .data
        .pool_id;
    let pool_atom_osmo_provided = gamm
        .create_basic_pool(&[coin(3_000_000, "uatom"), coin(6_000_000, "uosmo")], &signer)
        .unwrap()
        .data
        .pool_id;

    // check that we are not using the same pool id
    assert_ne!(pool_atom_osmo_saved, pool_atom_osmo_provided);

    swap_to_create_twap_records(
        &app,
        &signer,
        pool_atom_osmo_saved,
        coin(10u128, "uatom"),
        "uosmo",
    );
    swap_to_create_twap_records(
        &app,
        &signer,
        pool_atom_osmo_provided,
        coin(10u128, "uatom"),
        "uosmo",
    );

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute, OsmosisConfig>::SetRoute {
            denom_in: "uosmo".to_string(),
            denom_out: "uatom".to_string(),
            route: OsmosisRoute(vec![SwapAmountInRoute {
                pool_id: pool_atom_osmo_saved,
                token_out_denom: "uatom".to_string(),
            }]),
        },
        &[],
        &signer,
    )
    .unwrap();

    let coin_in_amount = Uint128::from(1000u128);

    let uosmo_price = query_price_from_pool(&gamm, pool_atom_osmo_saved, "uosmo");
    let expected_output_saved = coin_in_amount * uosmo_price;
    let res: EstimateExactInSwapResponse = wasm
        .query(
            &contract_addr,
            &QueryMsg::EstimateExactInSwap {
                coin_in: coin(coin_in_amount.u128(), "uosmo"),
                denom_out: "uatom".to_string(),
                route: None,
            },
        )
        .unwrap();
    assert_eq!(res.amount, expected_output_saved);

    let uosmo_price = query_price_from_pool(&gamm, pool_atom_osmo_provided, "uosmo");
    let expected_output_provided = coin_in_amount * uosmo_price;
    let res: EstimateExactInSwapResponse = wasm
        .query(
            &contract_addr,
            &QueryMsg::EstimateExactInSwap {
                coin_in: coin(coin_in_amount.u128(), "uosmo"),
                denom_out: "uatom".to_string(),
                route: Some(SwapperRoute::Osmo(OsmoRoute {
                    swaps: vec![OsmoSwap {
                        pool_id: pool_atom_osmo_provided,
                        to: "uatom".to_string(),
                    }],
                })),
            },
        )
        .unwrap();
    assert_eq!(res.amount, expected_output_provided);

    // check that we are not using the same routes
    assert_ne!(expected_output_saved, expected_output_provided);
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
    let _pool_osmo_mars = gamm
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
                route: Some(SwapperRoute::Osmo(OsmoRoute {
                    swaps: vec![
                        OsmoSwap {
                            pool_id: pool_atom_osmo,
                            to: "uosmo".to_string(),
                        },
                        OsmoSwap {
                            pool_id: pool_osmo_usdc,
                            to: "uusdc".to_string(),
                        },
                    ],
                })),
            },
        )
        .unwrap();
    assert_eq!(res.amount, expected_output);
}

#[test]
fn estimate_swap_multi_step_with_cosmwasm_pool() {
    let mut deps = OwnedDeps::<_, _, _> {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: MarsMockQuerier::new(MockQuerier::new(&[])),
        custom_query_type: PhantomData,
    };

    // instantiate the swapper contract
    instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        InstantiateMsg {
            owner: "owner".to_string(),
        },
    )
    .unwrap();

    let atom = "uatom".to_string();
    let noble_usdc = "unusdc".to_string();
    let axl_usdc = "uausdc".to_string();

    // prepare ConcentratedLiquidity pool
    let cl_pool_id = 1251;
    let pool = ConcentratedLiquidityPool {
        address: "osmo126pr9qp44aft4juw7x4ev4s2qdtnwe38jzwunec9pxt5cpzaaphqyagqpu".to_string(),
        incentives_address: "osmo1h2mhtj3wmsdt3uacev9pgpg38hkcxhsmyyn9ums0ya6eddrsafjsxs9j03"
            .to_string(),
        spread_rewards_address: "osmo16j5sssw32xuk8a0kjj8n54g25ye6kr339nz5axf8lzyeajk0k22stsm36c"
            .to_string(),
        id: cl_pool_id,
        current_tick_liquidity: "3820025893854099618.699762490947860933".to_string(),
        token0: atom.clone(),
        token1: noble_usdc.clone(),
        current_sqrt_price: "656651.537483144215151633465586753226461989".to_string(),
        current_tick: 102311912,
        tick_spacing: 100,
        exponent_at_price_one: -6,
        spread_factor: "0.002000000000000000".to_string(),
        last_liquidity_update: None,
    };
    let cl_pool = PoolResponse {
        pool: Some(pool.to_any()),
    };

    // prepare CosmWasm (transmuter) pool
    let cw_pool_id = 1212;
    let msg = CosmwasmPoolInstantiateMsg {
        pool_asset_denoms: vec![axl_usdc.clone(), noble_usdc.clone()],
    };
    let pool = CosmWasmPool {
        contract_address: "osmo10c8y69yylnlwrhu32ralf08ekladhfknfqrjsy9yqc9ml8mlxpqq2sttzk"
            .to_string(),
        pool_id: cw_pool_id,
        code_id: 148,
        instantiate_msg: to_json_vec(&msg).unwrap(),
    };
    let cw_pool = PoolResponse {
        pool: Some(pool.to_any()),
    };

    deps.querier.set_query_pool_response(cl_pool_id, cl_pool);
    deps.querier.set_query_pool_response(cw_pool_id, cw_pool);

    // set arithmetic twap price for the ConcentratedLiquidity pool
    deps.querier.set_arithmetic_twap_price(
        cl_pool_id,
        &atom,
        &noble_usdc,
        ArithmeticTwapToNowResponse {
            arithmetic_twap: Decimal::from_str("11.5").unwrap().to_string(),
        },
    );

    // check the estimate swap output
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::EstimateExactInSwap {
            coin_in: coin(1250, &atom),
            denom_out: axl_usdc.clone(),
            route: Some(SwapperRoute::Osmo(OsmoRoute {
                swaps: vec![
                    OsmoSwap {
                        pool_id: cl_pool_id,
                        to: noble_usdc,
                    },
                    OsmoSwap {
                        pool_id: cw_pool_id,
                        to: axl_usdc,
                    },
                ],
            })),
        },
    )
    .unwrap();
    let res: EstimateExactInSwapResponse = from_json(res).unwrap();
    // 1250 * 11.5 * 1 = 14375
    assert_eq!(res.amount, Uint128::from(14375u128));
}
