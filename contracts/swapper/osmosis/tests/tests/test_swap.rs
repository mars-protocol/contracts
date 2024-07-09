use cosmwasm_std::{coin, Addr, Coin, Decimal};
use cw_it::{
    osmosis_test_tube::{Account, Bank, Gamm, Module, OsmosisTestApp, Wasm},
    test_tube::FeeSetting,
};
use mars_swapper_base::ContractError;
use mars_swapper_osmosis::{
    config::OsmosisConfig,
    route::{OsmosisRoute, SwapAmountInRoute},
};
use mars_types::swapper::{
    EstimateExactInSwapResponse, ExecuteMsg, OsmoRoute, OsmoSwap, QueryMsg, SwapperRoute,
};

use super::helpers::{
    assert_err, instantiate_contract, query_balance, swap_to_create_twap_records,
};

#[test]
fn transfer_callback_only_internal() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let accs = app.init_accounts(&[coin(1_000_000_000_000, "uosmo")], 2).unwrap();
    let owner = &accs[0];
    let bad_guy = &accs[1];

    let contract_addr = instantiate_contract(&wasm, owner);

    let res_err = wasm
        .execute(
            &contract_addr,
            &ExecuteMsg::<OsmosisRoute, OsmosisConfig>::TransferResult {
                recipient: Addr::unchecked(bad_guy.address()),
                denom_in: "mars".to_string(),
                denom_out: "osmo".to_string(),
            },
            &[],
            bad_guy,
        )
        .unwrap_err();

    assert_err(
        res_err,
        ContractError::Unauthorized {
            user: bad_guy.address(),
            action: "transfer result".to_string(),
        },
    );
}

#[test]
fn swap_exact_in_slippage_too_high() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "uosmo"), coin(1_000_000_000_000, "umars")])
        .unwrap();
    let tx_fee = 1_000_000u128;
    let whale = app
        .init_account(&[coin(1_000_000, "umars"), coin(1_000_000, "uosmo")])
        .unwrap()
        .with_fee_setting(FeeSetting::Custom {
            amount: Coin::new(tx_fee, "uosmo"),
            gas_limit: tx_fee as u64,
        });

    let contract_addr = instantiate_contract(&wasm, &signer);

    let gamm = Gamm::new(&app);
    let pool_mars_osmo = gamm
        .create_basic_pool(&[coin(6_000_000, "umars"), coin(1_500_000, "uosmo")], &signer)
        .unwrap()
        .data
        .pool_id;

    swap_to_create_twap_records(&app, &signer, pool_mars_osmo, coin(10u128, "umars"), "uosmo");

    let route = Some(SwapperRoute::Osmo(OsmoRoute {
        swaps: vec![OsmoSwap {
            pool_id: pool_mars_osmo,
            to: "uosmo".to_string(),
        }],
    }));

    let res: EstimateExactInSwapResponse = wasm
        .query(
            &contract_addr,
            &QueryMsg::EstimateExactInSwap {
                coin_in: coin(1_000_000, "umars"),
                denom_out: "uosmo".to_string(),
                route: route.clone(),
            },
        )
        .unwrap();
    let min_receive = res.amount * (Decimal::one() - Decimal::percent(5));

    // whale does a huge trade
    let res_err = wasm
        .execute(
            &contract_addr,
            &ExecuteMsg::<OsmosisRoute, OsmosisConfig>::SwapExactIn {
                coin_in: coin(1_000_000, "umars"),
                denom_out: "uosmo".to_string(),
                min_receive,
                route,
            },
            &[coin(1_000_000, "umars")],
            &whale,
        )
        .unwrap_err();

    println!("{:?}", res_err);

    assert_err(
        res_err,
        "uosmo token is lesser than min amount: calculated amount is lesser than min amount",
    )
}

#[test]
fn swap_exact_in_success_with_saved_route() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "uosmo"), coin(1_000_000_000_000, "umars")])
        .unwrap();

    let tx_fee = 1_000_000u128;
    let user_osmo_starting_amount = 10_000_000u128;
    let user = app
        .init_account(&[coin(10_000, "umars"), coin(user_osmo_starting_amount, "uosmo")])
        .unwrap()
        .with_fee_setting(FeeSetting::Custom {
            amount: Coin::new(tx_fee, "uosmo"),
            gas_limit: tx_fee as u64,
        });

    let contract_addr = instantiate_contract(&wasm, &signer);

    let gamm = Gamm::new(&app);
    let pool_mars_osmo = gamm
        .create_basic_pool(&[coin(6_000_000, "umars"), coin(1_500_000, "uosmo")], &signer)
        .unwrap()
        .data
        .pool_id;

    swap_to_create_twap_records(&app, &signer, pool_mars_osmo, coin(10u128, "umars"), "uosmo");

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute, OsmosisConfig>::SetRoute {
            denom_in: "umars".to_string(),
            denom_out: "uosmo".to_string(),
            route: OsmosisRoute(vec![SwapAmountInRoute {
                pool_id: pool_mars_osmo,
                token_out_denom: "uosmo".to_string(),
            }]),
        },
        &[],
        &signer,
    )
    .unwrap();

    let bank = Bank::new(&app);
    let osmo_balance = query_balance(&bank, &user.address(), "uosmo");
    let mars_balance = query_balance(&bank, &user.address(), "umars");
    assert_eq!(osmo_balance, user_osmo_starting_amount);
    assert_eq!(mars_balance, 10_000);

    let res: EstimateExactInSwapResponse = wasm
        .query(
            &contract_addr,
            &QueryMsg::EstimateExactInSwap {
                coin_in: coin(10_000, "umars"),
                denom_out: "uosmo".to_string(),
                route: None,
            },
        )
        .unwrap();
    let min_receive = res.amount * (Decimal::one() - Decimal::percent(6));

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute, OsmosisConfig>::SwapExactIn {
            coin_in: coin(10_000, "umars"),
            denom_out: "uosmo".to_string(),
            min_receive,
            route: None,
        },
        &[coin(10_000, "umars")],
        &user,
    )
    .unwrap();

    // Assert user receives their new tokens
    let osmo_balance = query_balance(&bank, &user.address(), "uosmo");
    let mars_balance = query_balance(&bank, &user.address(), "umars");
    assert_eq!(osmo_balance, 2470 + user_osmo_starting_amount - tx_fee);
    assert_eq!(mars_balance, 0);

    // Assert no tokens in contract left over
    let osmo_balance = query_balance(&bank, &contract_addr, "uosmo");
    let mars_balance = query_balance(&bank, &contract_addr, "umars");
    assert_eq!(osmo_balance, 0);
    assert_eq!(mars_balance, 0);
}

#[test]
fn swap_exact_in_success_with_provided_route() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "uosmo"), coin(1_000_000_000_000, "umars")])
        .unwrap();

    let tx_fee = 1_000_000u128;
    let user_osmo_starting_amount = 10_000_000u128;
    let user = app
        .init_account(&[coin(10_000, "umars"), coin(user_osmo_starting_amount, "uosmo")])
        .unwrap()
        .with_fee_setting(FeeSetting::Custom {
            amount: Coin::new(tx_fee, "uosmo"),
            gas_limit: tx_fee as u64,
        });

    let contract_addr = instantiate_contract(&wasm, &signer);

    let gamm = Gamm::new(&app);
    let pool_mars_osmo = gamm
        .create_basic_pool(&[coin(6_000_000, "umars"), coin(1_500_000, "uosmo")], &signer)
        .unwrap()
        .data
        .pool_id;

    swap_to_create_twap_records(&app, &signer, pool_mars_osmo, coin(10u128, "umars"), "uosmo");

    let bank = Bank::new(&app);
    let osmo_balance = query_balance(&bank, &user.address(), "uosmo");
    let mars_balance = query_balance(&bank, &user.address(), "umars");
    assert_eq!(osmo_balance, user_osmo_starting_amount);
    assert_eq!(mars_balance, 10_000);

    let route = Some(SwapperRoute::Osmo(OsmoRoute {
        swaps: vec![OsmoSwap {
            pool_id: pool_mars_osmo,
            to: "uosmo".to_string(),
        }],
    }));

    let res: EstimateExactInSwapResponse = wasm
        .query(
            &contract_addr,
            &QueryMsg::EstimateExactInSwap {
                coin_in: coin(10_000, "umars"),
                denom_out: "uosmo".to_string(),
                route: route.clone(),
            },
        )
        .unwrap();
    let min_receive = res.amount * (Decimal::one() - Decimal::percent(6));

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute, OsmosisConfig>::SwapExactIn {
            coin_in: coin(10_000, "umars"),
            denom_out: "uosmo".to_string(),
            min_receive,
            route,
        },
        &[coin(10_000, "umars")],
        &user,
    )
    .unwrap();

    // Assert user receives their new tokens
    let osmo_balance = query_balance(&bank, &user.address(), "uosmo");
    let mars_balance = query_balance(&bank, &user.address(), "umars");
    assert_eq!(osmo_balance, 2470 + user_osmo_starting_amount - tx_fee);
    assert_eq!(mars_balance, 0);

    // Assert no tokens in contract left over
    let osmo_balance = query_balance(&bank, &contract_addr, "uosmo");
    let mars_balance = query_balance(&bank, &contract_addr, "umars");
    assert_eq!(osmo_balance, 0);
    assert_eq!(mars_balance, 0);
}

#[test]
fn swap_exact_in_success_with_provided_route_when_saved_route_exists() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "uosmo"), coin(1_000_000_000_000, "umars")])
        .unwrap();

    let tx_fee = 1_000_000u128;
    let user_osmo_starting_amount = 10_000_000u128;
    let user = app
        .init_account(&[coin(20_000, "umars"), coin(user_osmo_starting_amount, "uosmo")])
        .unwrap()
        .with_fee_setting(FeeSetting::Custom {
            amount: Coin::new(tx_fee, "uosmo"),
            gas_limit: tx_fee as u64,
        });

    let contract_addr = instantiate_contract(&wasm, &signer);

    let gamm = Gamm::new(&app);
    let pool_mars_osmo_saved = gamm
        .create_basic_pool(&[coin(6_000_000, "umars"), coin(3_000_000, "uosmo")], &signer)
        .unwrap()
        .data
        .pool_id;
    let pool_mars_osmo_provided = gamm
        .create_basic_pool(&[coin(6_000_000, "umars"), coin(1_500_000, "uosmo")], &signer)
        .unwrap()
        .data
        .pool_id;

    swap_to_create_twap_records(
        &app,
        &signer,
        pool_mars_osmo_provided,
        coin(10u128, "umars"),
        "uosmo",
    );

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute, OsmosisConfig>::SetRoute {
            denom_in: "umars".to_string(),
            denom_out: "uosmo".to_string(),
            route: OsmosisRoute(vec![SwapAmountInRoute {
                pool_id: pool_mars_osmo_saved,
                token_out_denom: "uosmo".to_string(),
            }]),
        },
        &[],
        &signer,
    )
    .unwrap();

    let bank = Bank::new(&app);
    let osmo_balance = query_balance(&bank, &user.address(), "uosmo");
    let mars_balance = query_balance(&bank, &user.address(), "umars");
    assert_eq!(osmo_balance, user_osmo_starting_amount);
    assert_eq!(mars_balance, 20_000);

    let route = Some(SwapperRoute::Osmo(OsmoRoute {
        swaps: vec![OsmoSwap {
            pool_id: pool_mars_osmo_provided,
            to: "uosmo".to_string(),
        }],
    }));

    let res: EstimateExactInSwapResponse = wasm
        .query(
            &contract_addr,
            &QueryMsg::EstimateExactInSwap {
                coin_in: coin(10_000, "umars"),
                denom_out: "uosmo".to_string(),
                route: route.clone(),
            },
        )
        .unwrap();
    let min_receive = res.amount * (Decimal::one() - Decimal::percent(6));

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute, OsmosisConfig>::SwapExactIn {
            coin_in: coin(10_000, "umars"),
            denom_out: "uosmo".to_string(),
            min_receive,
            route,
        },
        &[coin(10_000, "umars")],
        &user,
    )
    .unwrap();

    // Assert user receives their new tokens
    let osmo_balance_with_provided_route = query_balance(&bank, &user.address(), "uosmo");
    let mars_balance = query_balance(&bank, &user.address(), "umars");
    assert_eq!(osmo_balance_with_provided_route, 2470 + user_osmo_starting_amount - tx_fee);
    assert_eq!(mars_balance, 10_000);

    // Assert no tokens in contract left over
    let osmo_balance = query_balance(&bank, &contract_addr, "uosmo");
    let mars_balance = query_balance(&bank, &contract_addr, "umars");
    assert_eq!(osmo_balance, 0);
    assert_eq!(mars_balance, 0);

    let res: EstimateExactInSwapResponse = wasm
        .query(
            &contract_addr,
            &QueryMsg::EstimateExactInSwap {
                coin_in: coin(10_000, "umars"),
                denom_out: "uosmo".to_string(),
                route: None,
            },
        )
        .unwrap();
    let min_receive = res.amount * (Decimal::one() - Decimal::percent(6));

    // Confirm that previous swap with provided routes uses correct route (pools).
    // Check if we swap with saved route we get different amount of tokens.
    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute, OsmosisConfig>::SwapExactIn {
            coin_in: coin(10_000, "umars"),
            denom_out: "uosmo".to_string(),
            min_receive,
            route: None,
        },
        &[coin(10_000, "umars")],
        &user,
    )
    .unwrap();
    let osmo_balance = query_balance(&bank, &user.address(), "uosmo");
    let mars_balance = query_balance(&bank, &user.address(), "umars");
    // 2470 is the amount of tokens we get from swap with provided route, 4941 is the amount of tokens we get from swap with saved route.
    // It is expected that we get more tokens from swap with saved route because we have more uosmo liquidity in the pool.
    // pool_mars_osmo_saved: 6_000_000 umars, 3_000_000 uosmo
    // pool_mars_osmo_provided: 6_000_000 umars, 1_500_000 uosmo
    assert_eq!(osmo_balance, 4941 + osmo_balance_with_provided_route - tx_fee);
    assert_eq!(mars_balance, 0);
}
