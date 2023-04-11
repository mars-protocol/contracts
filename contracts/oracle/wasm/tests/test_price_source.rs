use astroport::{asset::AssetInfo, factory::PairType};
use cosmwasm_std::{Decimal, Uint128};
use cw_it::astroport::{
    robot::AstroportTestRobot,
    utils::{native_asset, native_info},
};
use mars_oracle_wasm::WasmPriceSourceUnchecked;
use test_case::test_case;

mod helpers;
pub use helpers::*;

#[test]
fn test_contract_initialization() {
    let runner = get_test_runner();
    let admin = &runner.init_accounts()[0];
    let contract_map = get_contracts(&runner);
    setup_test(&runner, contract_map, admin, None);
}

#[test]
fn test_set_price_source_fixed() {
    let runner = get_test_runner();
    let admin = &runner.init_accounts()[0];
    let contract_map = get_contracts(&runner);
    let robot = setup_test(&runner, contract_map, admin, None);

    let price_source = WasmPriceSourceUnchecked::Fixed {
        price: cosmwasm_std::Decimal::one(),
    };
    let denom = "uatom";

    // Execute SetPriceSource
    robot
        .set_price_source(denom, price_source.clone(), admin)
        .assert_price_source(denom, price_source);
}

#[test]
fn remove_price_source() {
    let runner = get_test_runner();
    let admin = &runner.init_accounts()[0];
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, None);
    let denom = "uusd";
    let price_source = WasmPriceSourceUnchecked::Fixed {
        price: cosmwasm_std::Decimal::one(),
    };

    // Execute SetPriceSource
    robot
        .set_price_source(denom, price_source.clone(), admin)
        .remove_price_source(admin, denom)
        .assert_price_source_not_exists(denom);
}

#[test]
fn test_query_fixed_price() {
    let runner = get_test_runner();
    let admin = &runner.init_accounts()[0];
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, None);
    let denom = "uusd";
    let price_source = WasmPriceSourceUnchecked::Fixed {
        price: Decimal::one(),
    };

    // Set price and then query it
    robot.set_price_source(denom, price_source.clone(), admin).assert_price(denom, Decimal::one());
}

#[test_case(PairType::Xyk {}; "xyk")]
#[test_case(PairType::Stable {}; "stable")]

fn test_query_astroport_spot_price_without_route_asset(pair_type: PairType) {
    let runner = get_test_runner();
    let admin = &runner.init_accounts()[0];
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, Some("uosmo"));
    let initial_liq: [Uint128; 2] =
        [10000000000000000000000u128.into(), 1000000000000000000000u128.into()];
    let init_params = astro_init_params(&pair_type);
    let (pair_address, _lp_token_addr) = robot.create_astroport_pair(
        pair_type,
        [
            AssetInfo::NativeToken {
                denom: "uatom".to_string(),
            },
            AssetInfo::NativeToken {
                denom: "uosmo".to_string(),
            },
        ],
        init_params,
        admin,
        Some(initial_liq),
    );
    let price_source = WasmPriceSourceUnchecked::AstroportSpot {
        pair_address: pair_address.clone(),
        route_assets: vec![],
    };

    // Oracle uses a swap simulation rather than just dividing the reserves, because we need to support non XYK pools
    let sim_res = robot.query_simulate_swap(
        &pair_address,
        native_asset("uatom", 1000000u128),
        Some(native_info("uosmo")),
    );
    let expected_price = Decimal::from_ratio(sim_res.return_amount, 1000000u128);

    // Execute SetPriceSource
    robot
        .add_denom_precision_to_coin_registry("uatom", 6, admin)
        .add_denom_precision_to_coin_registry("uosmo", 6, admin)
        .set_price_source("uatom", price_source.clone(), admin)
        .assert_price("uatom", expected_price);
}

#[test_case(PairType::Xyk {}; "xyk")]
#[test_case(PairType::Stable {}; "stable")]
fn test_query_astroport_xyk_spot_price_with_route_asset(pair_type: PairType) {
    let runner = get_test_runner();
    let admin = &runner.init_accounts()[0];
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, Some("usd"));
    let initial_liq: [Uint128; 2] =
        [10000000000000000000000u128.into(), 1000000000000000000000u128.into()];
    let osmo_price = Decimal::from_ratio(2u128, 1u128);
    let init_params = astro_init_params(&pair_type);
    let (pair_address, _lp_token_addr) = robot.create_astroport_pair(
        pair_type,
        [
            AssetInfo::NativeToken {
                denom: "uatom".to_string(),
            },
            AssetInfo::NativeToken {
                denom: "uosmo".to_string(),
            },
        ],
        init_params,
        admin,
        Some(initial_liq),
    );
    let price_source = WasmPriceSourceUnchecked::AstroportSpot {
        pair_address: pair_address.clone(),
        route_assets: vec!["uosmo".to_string(), "usd".to_string()],
    };
    let osmo_price_source = WasmPriceSourceUnchecked::Fixed {
        price: osmo_price,
    };
    let usd_price_source = WasmPriceSourceUnchecked::Fixed {
        price: Decimal::one(),
    };

    // Oracle uses a swap simulation rather than just dividing the reserves, because we need to support non XYK pools
    let sim_res = robot.query_simulate_swap(
        &pair_address,
        native_asset("uatom", 1000000u128),
        Some(native_info("uosmo")),
    );
    let expected_price = Decimal::from_ratio(sim_res.return_amount, 1000000u128) * osmo_price;

    // Execute SetPriceSource
    robot
        .add_denom_precision_to_coin_registry("uatom", 6, admin)
        .add_denom_precision_to_coin_registry("uosmo", 6, admin)
        .set_price_source("usd", usd_price_source, admin)
        .set_price_source("uosmo", osmo_price_source, admin)
        .set_price_source("uatom", price_source.clone(), admin)
        .assert_price("uatom", expected_price);
}
