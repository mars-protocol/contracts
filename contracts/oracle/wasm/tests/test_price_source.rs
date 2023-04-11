use astroport::{asset::AssetInfo, factory::PairType};
use cosmwasm_std::{Decimal, Uint128};
use cw_it::astroport::robot::AstroportTestRobot;
use mars_oracle_wasm::WasmPriceSourceUnchecked;

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

#[test]
fn test_query_astroport_xyk_spot_price_without_route_asset() {
    let runner = get_test_runner();
    let admin = &runner.init_accounts()[0];
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, Some("uosmo"));
    let initial_liq: [Uint128; 2] =
        [10000000000000000000000u128.into(), 1000000000000000000000u128.into()];
    let (pair_address, _lp_token_addr) = robot.create_astroport_pair(
        PairType::Xyk {},
        [
            AssetInfo::NativeToken {
                denom: "uatom".to_string(),
            },
            AssetInfo::NativeToken {
                denom: "uosmo".to_string(),
            },
        ],
        None,
        admin,
        Some(initial_liq),
    );
    let price_source = WasmPriceSourceUnchecked::AstroportSpot {
        pair_address,
        route_assets: vec![],
    };

    // Oracle uses a swap simulation rather than just dividing the reserves, because we need to support non XYK pools
    let fee = Decimal::permille(3);
    let expected_price =
        Decimal::from_ratio(initial_liq[1], initial_liq[0]) * (Decimal::one() - fee);

    // Execute SetPriceSource
    robot
        .add_denom_precision_to_coin_registry("uatom", 6, admin)
        .add_denom_precision_to_coin_registry("uosmo", 6, admin)
        .set_price_source("uatom", price_source.clone(), admin)
        .assert_price("uatom", expected_price);
}

#[test]
fn test_query_astroport_xyk_spot_price_with_route_asset() {
    let runner = get_test_runner();
    let admin = &runner.init_accounts()[0];
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, Some("usd"));
    let initial_liq: [Uint128; 2] =
        [10000000000000000000000u128.into(), 1000000000000000000000u128.into()];
    let osmo_price = Decimal::from_ratio(2u128, 1u128);
    let (pair_address, _lp_token_addr) = robot.create_astroport_pair(
        PairType::Xyk {},
        [
            AssetInfo::NativeToken {
                denom: "uatom".to_string(),
            },
            AssetInfo::NativeToken {
                denom: "uosmo".to_string(),
            },
        ],
        None,
        admin,
        Some(initial_liq),
    );
    let price_source = WasmPriceSourceUnchecked::AstroportSpot {
        pair_address,
        route_assets: vec!["uosmo".to_string(), "usd".to_string()],
    };
    let osmo_price_source = WasmPriceSourceUnchecked::Fixed {
        price: osmo_price,
    };
    let usd_price_source = WasmPriceSourceUnchecked::Fixed {
        price: Decimal::one(),
    };

    // Oracle uses a swap simulation rather than just dividing the reserves, because we need to support non XYK pools
    let fee = Decimal::permille(3);
    let expected_price =
        Decimal::from_ratio(initial_liq[1] * osmo_price, initial_liq[0]) * (Decimal::one() - fee);

    // Execute SetPriceSource
    robot
        .add_denom_precision_to_coin_registry("uatom", 6, admin)
        .add_denom_precision_to_coin_registry("uosmo", 6, admin)
        .set_price_source("usd", usd_price_source, admin)
        .set_price_source("uosmo", osmo_price_source, admin)
        .set_price_source("uatom", price_source.clone(), admin)
        .assert_price("uatom", expected_price);
}
