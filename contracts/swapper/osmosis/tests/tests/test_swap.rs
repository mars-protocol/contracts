use cosmwasm_std::{coin, Addr, Coin, Decimal};
use cw_it::{
    osmosis_test_tube::{Account, Bank, Gamm, Module, OsmosisTestApp, Wasm},
    test_tube::FeeSetting,
};
use mars_swapper_base::ContractError;
use mars_swapper_osmosis::route::OsmosisRoute;
use mars_types::swapper::{ExecuteMsg, SwapperRoute};

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
            &ExecuteMsg::<OsmosisRoute>::TransferResult {
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
fn max_slippage_exeeded() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let accs = app
        .init_accounts(&[coin(1_000_000_000_000, "uosmo"), coin(1_000_000_000_000, "umars")], 2)
        .unwrap();
    let owner = &accs[0];
    let other_guy = &accs[1];

    let contract_addr = instantiate_contract(&wasm, owner);

    let res_err = wasm
        .execute(
            &contract_addr,
            &ExecuteMsg::<OsmosisRoute>::SwapExactIn {
                coin_in: coin(1_000_000, "umars"),
                denom_out: "uosmo".to_string(),
                slippage: Decimal::percent(11),
                route: Some(SwapperRoute::Osmo(mars_types::swapper::OsmosisRoute(vec![
                    mars_types::swapper::SwapAmountInRoute {
                        pool_id: 1,
                        token_out_denom: "uosmo".to_string(),
                    },
                ]))),
            },
            &[coin(1_000_000, "umars")],
            other_guy,
        )
        .unwrap_err();

    assert_err(
        res_err,
        ContractError::MaxSlippageExceeded {
            max_slippage: Decimal::percent(10),
            slippage: Decimal::percent(11),
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

    // whale does a huge trade
    let res_err = wasm
        .execute(
            &contract_addr,
            &ExecuteMsg::<OsmosisRoute>::SwapExactIn {
                coin_in: coin(1_000_000, "umars"),
                denom_out: "uosmo".to_string(),
                slippage: Decimal::percent(5),
                route: Some(SwapperRoute::Osmo(mars_types::swapper::OsmosisRoute(vec![
                    mars_types::swapper::SwapAmountInRoute {
                        pool_id: pool_mars_osmo,
                        token_out_denom: "uosmo".to_string(),
                    },
                ]))),
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
fn swap_exact_in_success() {
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

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute>::SwapExactIn {
            coin_in: coin(10_000, "umars"),
            denom_out: "uosmo".to_string(),
            slippage: Decimal::percent(6),
            route: Some(SwapperRoute::Osmo(mars_types::swapper::OsmosisRoute(vec![
                mars_types::swapper::SwapAmountInRoute {
                    pool_id: pool_mars_osmo,
                    token_out_denom: "uosmo".to_string(),
                },
            ]))),
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
