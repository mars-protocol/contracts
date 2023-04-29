#![allow(clippy::items_after_test_module)]

use astroport::factory::PairType;
use cosmwasm_std::{testing::mock_dependencies, Addr, Decimal, Uint128};
use cw_it::{
    astroport::{
        robot::AstroportTestRobot,
        utils::{native_asset, native_info},
    },
    test_tube::Account,
};
use cw_storage_plus::Map;
use mars_oracle_base::PriceSourceUnchecked;
use mars_oracle_wasm::{WasmPriceSource, WasmPriceSourceChecked, WasmPriceSourceUnchecked};
use test_case::test_case;

mod helpers;
pub use helpers::*;

const ONE: Decimal = Decimal::one();
const TWO: Decimal = Decimal::new(Uint128::new(2_000_000_000_000_000_000u128));
const DEFAULT_LIQ: [u128; 2] = [10000000000000000000000u128, 1000000000000000000000u128];

#[test]
fn test_contract_initialization() {
    let runner = get_test_runner();
    let admin = &runner.init_accounts()[0];
    let contract_map = get_contracts(&runner);
    let robot = setup_test(&runner, contract_map, admin, Some("USD"));

    let config = robot.query_config();
    assert_eq!(config.base_denom, "USD");
    assert_eq!(config.owner, Some(admin.address()));
    assert_eq!(config.proposed_new_owner, None);
}

#[test]
fn display_fixed_price_source() {
    let ps = WasmPriceSource::Fixed {
        price: Decimal::from_ratio(1u128, 2u128),
    };
    assert_eq!(ps.to_string(), "fixed:0.5")
}

#[test]
fn display_spot_price_source() {
    let ps = WasmPriceSourceChecked::AstroportSpot {
        pair_address: Addr::unchecked("fake_addr"),
        route_assets: vec![],
    };
    assert_eq!(ps.to_string(), "astroport_spot:fake_addr. Route: ")
}

#[test]
fn display_spot_price_source_with_route() {
    let ps = WasmPriceSourceChecked::AstroportSpot {
        pair_address: Addr::unchecked("fake_addr"),
        route_assets: vec!["fake_asset1".to_string(), "fake_asset2".to_string()],
    };
    assert_eq!(ps.to_string(), "astroport_spot:fake_addr. Route: fake_asset1,fake_asset2")
}

#[test]
fn display_twap_price_source() {
    let ps = WasmPriceSourceChecked::AstroportTwap {
        pair_address: Addr::unchecked("fake_addr"),
        window_size: 100,
        tolerance: 10,
        route_assets: vec![],
    };
    assert_eq!(ps.to_string(), "astroport_twap:fake_addr. Window Size: 100. Tolerance: 10. Route: ")
}

#[test]
fn display_twap_price_source_with_route() {
    let ps = WasmPriceSourceChecked::AstroportTwap {
        pair_address: Addr::unchecked("fake_addr"),
        window_size: 100,
        tolerance: 10,
        route_assets: vec!["fake_asset1".to_string(), "fake_asset2".to_string()],
    };
    assert_eq!(
        ps.to_string(),
        "astroport_twap:fake_addr. Window Size: 100. Tolerance: 10. Route: fake_asset1,fake_asset2"
    )
}

#[test]
fn validate_fixed_price_source() {
    let ps = WasmPriceSource::Fixed {
        price: Decimal::from_ratio(1u128, 2u128),
    };
    let deps = mock_dependencies();
    let price_sources = Map::new("price_sources");
    let denom = "uusd";
    let base_denom = "uusd";
    let res = ps.validate(&deps.as_ref(), denom, base_denom, &price_sources);
    assert!(res.is_ok());
}

#[test]
fn test_set_price_source_fixed() {
    let runner = get_test_runner();
    let admin = &runner.init_accounts()[0];
    let contract_map = get_contracts(&runner);
    let robot = setup_test(&runner, contract_map, admin, None);

    let price_source = WasmPriceSourceUnchecked::Fixed {
        price: ONE,
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
        price: ONE,
    };

    // Execute SetPriceSource
    robot
        .set_price_source(denom, price_source, admin)
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
        price: ONE,
    };

    // Set price and then query it
    robot.set_price_source(denom, price_source, admin).assert_price(denom, ONE);
}

#[test_case(PairType::Xyk {}, &["uatom","uosmo"], "uosmo", &[], true; "XYK, no route, base_denom in pair")]
#[test_case(PairType::Xyk {}, &["uatom","uosmo"], "USD", &[], true => panics; "XYK, no route, base_denom not in pair")]
#[test_case(PairType::Xyk {}, &["uatom","uosmo"], "uosmo", &[("uusd", TWO)], false => panics; "XYK, route asset does not exist")]
#[test_case(PairType::Xyk {}, &["uatom","uosmo"], "uosmo", &[("uosmo", TWO)], true; "XYK, route equal to base_denom")]
#[test_case(PairType::Xyk {}, &["uatom","uosmo"], "uosmo", &[("uion",TWO)], true => panics; "XYK, route with non-base existing asset, not in pair")]
#[test_case(PairType::Xyk {}, &["uatom","uion"], "uosmo", &[("uion",TWO)], true; "XYK, route with non-base existing asset, in pair")]
#[test_case(PairType::Stable {}, &["uatom","uosmo"], "uosmo", &[], true; "Stable, no route, base_denom in pair")]
#[test_case(PairType::Stable {}, &["uatom","uion"], "uosmo", &[("uion",TWO)], true; "Stable, route with non-base existing asset, in pair")]
#[test_case(PairType::Xyk {}, &["uosmo","stake"], "stake", &[("stake", TWO),("stake", TWO)], true => panics; "Duplicate asset in route")]
#[test_case(PairType::Xyk {}, &["stake", "uatom"], "uatom", &[("uatom", TWO),("stake", TWO)], true => panics; "pair asset in route")]
pub fn test_validate_and_query_astroport_spot_price_source(
    pair_type: PairType,
    pair_denoms: &[&str; 2],
    base_denom: &str,
    route_prices: &[(&str, Decimal)],
    register_routes: bool,
) {
    validate_and_query_astroport_spot_price_source(
        pair_type,
        pair_denoms,
        base_denom,
        route_prices,
        &DEFAULT_LIQ,
        register_routes,
    )
}

#[test_case(PairType::Xyk {}, &["uatom","uosmo"], "uosmo", &[], 5, 100; "XYK, no route, base_denom in pair")]
#[test_case(PairType::Xyk {}, &["uatom","uosmo"], "USD", &[], 5, 100 => panics; "XYK, no route, base_denom not in pair")]
#[test_case(PairType::Xyk {}, &["uatom","uosmo"], "uosmo", &[("uusd", TWO)], 5, 100 => panics; "XYK, route asset does not exist")]
#[test_case(PairType::Xyk {}, &["uatom","uosmo"], "uosmo", &[("uosmo", TWO)], 5, 100; "XYK, route equal to base_denom")]
#[test_case(PairType::Xyk {}, &["uatom","uosmo"], "uosmo", &[("uion",TWO)], 5, 100 => panics; "XYK, route with non-base existing asset, not in pair")]
#[test_case(PairType::Xyk {}, &["uatom","uion"], "uosmo", &[("uion",TWO)], 5, 100; "XYK, route with non-base existing asset, in pair")]
#[test_case(PairType::Stable {}, &["uatom","uosmo"], "uosmo", &[], 5, 100; "Stable, no route, base_denom in pair")]
#[test_case(PairType::Stable {}, &["uatom","uion"], "uosmo", &[("uion",TWO)], 5, 100; "Stable, route with non-base existing asset, in pair")]
fn test_validate_and_query_astroport_twap_price(
    pair_type: PairType,
    pair_denoms: &[&str; 2],
    base_denom: &str,
    route_prices: &[(&str, Decimal)],
    tolerance: u64,
    window_size: u64,
) {
    validate_and_query_astroport_twap_price_source(
        pair_type,
        pair_denoms,
        base_denom,
        route_prices,
        tolerance,
        window_size,
        &DEFAULT_LIQ,
    )
}

#[test]
#[should_panic]
fn record_twap_snapshots_errors_on_non_twap_price_source() {
    let runner = get_test_runner();
    let admin = &runner.init_accounts()[0];
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, None);

    robot
        .set_price_source("uosmo", fixed_source(ONE), admin)
        .record_twap_snapshots(&["uosmo"], admin);
}

#[test]
fn record_twap_snapshot_does_not_save_when_less_than_tolerance_ago() {
    let runner = get_test_runner();
    let admin = &runner.init_accounts()[0];
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, Some("uosmo"));

    let (pair_address, _) = robot.create_default_astro_pair(PairType::Xyk {}, admin);

    let price_source = WasmPriceSourceUnchecked::AstroportTwap {
        pair_address: pair_address.clone(),
        route_assets: vec![],
        tolerance: 20,
        window_size: 100,
    };

    robot
        .set_price_source("uatom", price_source, admin)
        .record_twap_snapshots(&["uatom"], admin)
        .increase_time(100)
        .record_twap_snapshots(&["uatom"], admin)
        .assert_price("uatom", Decimal::from_ratio(1u128, 10u128))
        .swap_on_astroport_pair(
            &pair_address,
            native_asset("uosmo", 1000000000000u128),
            None,
            None,
            Some(Decimal::percent(50)),
            admin,
        )
        .increase_time(10)
        .record_twap_snapshots(&["uatom"], admin)
        // Price should be the same as before
        .assert_price("uatom", Decimal::from_ratio(1u128, 10u128));
}
