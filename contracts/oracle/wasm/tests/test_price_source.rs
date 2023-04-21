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

#[test_case(PairType::Xyk {}, &["uatom","uosmo"], "uosmo", &[]; "XYK, no route, base_denom in pair")]
#[test_case(PairType::Xyk {}, &["uatom","uosmo"], "USD", &[] => panics; "XYK, no route, base_denom not in pair")]
#[test_case(PairType::Xyk {}, &["uatom","uosmo"], "uosmo", &[("uusd", TWO)] => panics; "XYK, route asset does not exist")]
#[test_case(PairType::Xyk {}, &["uatom","uosmo"], "uosmo", &[("uosmo", TWO)]; "XYK, route equal to base_denom")]
#[test_case(PairType::Xyk {}, &["uatom","uosmo"], "uosmo", &[("uion",TWO)] => panics; "XYK, route with non-base existing asset, not in pair")]
#[test_case(PairType::Xyk {}, &["uatom","uion"], "uosmo", &[("uion",TWO)]; "XYK, route with non-base existing asset, in pair")]
#[test_case(PairType::Stable {}, &["uatom","uosmo"], "uosmo", &[]; "Stable, no route, base_denom in pair")]
#[test_case(PairType::Stable {}, &["uatom","uion"], "uosmo", &[("uion",TWO)]; "Stable, route with non-base existing asset, in pair")]
fn test_validate_and_query_astroport_spot_price_source(
    pair_type: PairType,
    pair_denoms: &[&str; 2],
    base_denom: &str,
    route_prices: &[(&str, Decimal)],
) {
    let runner = get_test_runner();
    let admin = &runner.init_accounts()[0];
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, Some(base_denom));

    let initial_liq: [Uint128; 2] =
        [10000000000000000000000u128.into(), 1000000000000000000000u128.into()];
    let (pair_address, _lp_token_addr) = robot.create_astroport_pair(
        pair_type.clone(),
        [native_info(pair_denoms[0]), native_info(pair_denoms[1])],
        astro_init_params(&pair_type),
        admin,
        Some(initial_liq),
    );

    let price_source = WasmPriceSourceUnchecked::AstroportSpot {
        pair_address: pair_address.clone(),
        route_assets: route_prices.iter().map(|&(s, _)| s.to_string()).collect(),
    };
    let route_price_sources: Vec<_> =
        route_prices.iter().map(|&(s, p)| (s, fixed_source(p))).collect();

    // Oracle uses a swap simulation rather than just dividing the reserves, because we need to support non-XYK pools
    let sim_res =
        robot.query_simulate_swap(&pair_address, native_asset(pair_denoms[0], 1000000u128), None);
    let expected_price = route_prices
        .iter()
        .fold(Decimal::from_ratio(sim_res.return_amount, 1000000u128), |acc, &(_, p)| acc * p);

    // Execute SetPriceSource
    robot
        .add_denom_precision_to_coin_registry(pair_denoms[0], 6, admin)
        .add_denom_precision_to_coin_registry(pair_denoms[1], 6, admin)
        .add_denom_precision_to_coin_registry(base_denom, 6, admin)
        .set_price_sources(route_price_sources, admin)
        .set_price_source(pair_denoms[0], price_source.clone(), admin)
        .assert_price_source(pair_denoms[0], price_source)
        .assert_price(pair_denoms[0], expected_price);
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
    let runner = get_test_runner();
    let admin = &runner.init_accounts()[0];
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, Some(base_denom));

    let initial_liq: [Uint128; 2] =
        [10000000000000000000000u128.into(), 1000000000000000000000u128.into()];
    let (pair_address, _lp_token_addr) = robot.create_astroport_pair(
        pair_type.clone(),
        [native_info(pair_denoms[0]), native_info(pair_denoms[1])],
        astro_init_params(&pair_type),
        admin,
        Some(initial_liq),
    );
    let initial_price = robot
        .add_denom_precision_to_coin_registry(pair_denoms[0], 6, admin)
        .add_denom_precision_to_coin_registry(pair_denoms[1], 6, admin)
        .add_denom_precision_to_coin_registry(base_denom, 6, admin)
        .query_price_via_simulation(&pair_address, pair_denoms[0]);

    let price_source = WasmPriceSourceUnchecked::AstroportTwap {
        pair_address: pair_address.clone(),
        route_assets: route_prices.iter().map(|&(s, _)| s.to_string()).collect(),
        tolerance,
        window_size,
    };
    let route_price_sources: Vec<_> =
        route_prices.iter().map(|&(s, p)| (s, fixed_source(p))).collect();

    let price_after_swap = robot
        .set_price_sources(route_price_sources, admin)
        .set_price_source(pair_denoms[0], price_source.clone(), admin)
        .assert_price_source(pair_denoms[0], price_source)
        .record_twap_snapshots(&[pair_denoms[0]], admin)
        .increase_time(window_size / 2)
        .swap_on_astroport_pair(
            &pair_address,
            native_asset(pair_denoms[1], 10000000000000000000u128),
            None,
            None,
            Some(Decimal::from_ratio(1u128, 2u128)),
            admin,
        )
        .query_price_via_simulation(&pair_address, pair_denoms[0]);

    let price_precision: Uint128 = Uint128::from(10_u128.pow(8));
    let expected_price = Decimal::from_ratio(
        (initial_price + price_after_swap) * Decimal::from_ratio(1u128, 2u128) * price_precision,
        price_precision,
    );
    let expected_price = route_prices.iter().fold(expected_price, |acc, &(_, p)| acc * p);

    robot
        .record_twap_snapshots(&[pair_denoms[0]], admin)
        .increase_time(window_size / 2)
        .assert_price_almost_equal(pair_denoms[0], expected_price, Decimal::percent(1));
}
