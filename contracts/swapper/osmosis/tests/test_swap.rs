use cosmwasm_std::{coin, Addr, Decimal};
use osmosis_std::types::osmosis::gamm::v1beta1::SwapAmountInRoute;
use osmosis_testing::{Account, Bank, Gamm, Module, OsmosisTestApp, Wasm};

use mars_rover::adapters::swap::ExecuteMsg;
use mars_rover::error::ContractError as RoverError;
use mars_swapper_base::ContractError;
use mars_swapper_osmosis::route::OsmosisRoute;

use crate::helpers::{
    assert_err, instantiate_contract, query_balance, swap_to_create_twap_records,
};

pub mod helpers;

#[test]
fn test_transfer_callback_only_internal() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let accs = app
        .init_accounts(&[coin(1_000_000_000_000, "uosmo")], 2)
        .unwrap();
    let admin = &accs[0];
    let bad_guy = &accs[1];

    let contract_addr = instantiate_contract(&wasm, admin);

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
        ContractError::Rover(RoverError::Unauthorized {
            user: bad_guy.address(),
            action: "transfer result".to_string(),
        }),
    );
}

#[test]
fn test_swap_exact_in_slippage_too_high() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[
            coin(1_000_000_000_000, "uosmo"),
            coin(1_000_000_000_000, "umars"),
        ])
        .unwrap();
    let whale = app.init_account(&[coin(1_000_000, "umars")]).unwrap();

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

    swap_to_create_twap_records(
        &app,
        &signer,
        pool_mars_osmo,
        coin(10u128, "umars"),
        "uosmo",
    );

    let route = OsmosisRoute(vec![SwapAmountInRoute {
        pool_id: pool_mars_osmo,
        token_out_denom: "uosmo".to_string(),
    }]);

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::SetRoute {
            denom_in: "umars".to_string(),
            denom_out: "uosmo".to_string(),
            route,
        },
        &[],
        &signer,
    )
    .unwrap();

    // whale does a huge trade
    let res_err = wasm
        .execute(
            &contract_addr,
            &ExecuteMsg::<OsmosisRoute>::SwapExactIn {
                coin_in: coin(1_000_000, "umars"),
                denom_out: "uosmo".to_string(),
                slippage: Decimal::percent(5),
            },
            &[coin(1_000_000, "umars")],
            &whale,
        )
        .unwrap_err();

    assert_err(
        res_err,
        "uosmo token is lesser than min amount: calculated amount is lesser than min amount",
    )
}

#[test]
fn test_swap_exact_in_success() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[
            coin(1_000_000_000_000, "uosmo"),
            coin(1_000_000_000_000, "umars"),
        ])
        .unwrap();
    let user = app.init_account(&[coin(10_000, "umars")]).unwrap();

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

    swap_to_create_twap_records(
        &app,
        &signer,
        pool_mars_osmo,
        coin(10u128, "umars"),
        "uosmo",
    );

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::SetRoute {
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
    assert_eq!(osmo_balance, 0);
    assert_eq!(mars_balance, 10_000);

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute>::SwapExactIn {
            coin_in: coin(10_000, "umars"),
            denom_out: "uosmo".to_string(),
            slippage: Decimal::percent(6),
        },
        &[coin(10_000, "umars")],
        &user,
    )
    .unwrap();

    // Assert user receives their new tokens
    let osmo_balance = query_balance(&bank, &user.address(), "uosmo");
    let mars_balance = query_balance(&bank, &user.address(), "umars");
    assert_eq!(osmo_balance, 2470);
    assert_eq!(mars_balance, 0);

    // Assert no tokens in contract left over
    let osmo_balance = query_balance(&bank, &contract_addr, "uosmo");
    let mars_balance = query_balance(&bank, &contract_addr, "umars");
    assert_eq!(osmo_balance, 0);
    assert_eq!(mars_balance, 0);
}
