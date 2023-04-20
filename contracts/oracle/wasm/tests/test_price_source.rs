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

// TODO: Display test for twap

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
        price: Decimal::one(),
    };

    // Set price and then query it
    robot.set_price_source(denom, price_source, admin).assert_price(denom, Decimal::one());
}

#[test_case(&["uatom","uosmo"], "uosmo", &[] ; "no route, base_denom in pair")]
#[test_case(&["uatom","uosmo"], "USD", &[] => panics; "no route, base_denom not in pair")]
#[test_case(&["uatom","uosmo"], "uosmo", &["uusd"] => panics; "route asset does not exist")]
#[test_case(&["uatom","uosmo"], "uosmo", &["uosmo"]; "route equal to base_denom")]
#[test_case(&["uatom","uosmo"], "uosmo", &["uion"] => panics; "route with non-base existing asset, not in pair")]
#[test_case(&["uatom","uion"], "uosmo", &["uion"]; "route with non-base existing asset, in pair")]
fn test_validate_astroport_spot_price_source(
    pair_denoms: &[&str; 2],
    base_denom: &str,
    route_assets: &[&str],
) {
    let runner = get_test_runner();
    let admin = &runner.init_accounts()[0];
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, Some(base_denom));

    let initial_liq: [Uint128; 2] =
        [10000000000000000000000u128.into(), 1000000000000000000000u128.into()];
    let (pair_address, _lp_token_addr) = robot.create_astroport_pair(
        PairType::Xyk {},
        [native_info(pair_denoms[0]), native_info(pair_denoms[1])],
        None,
        admin,
        Some(initial_liq),
    );

    let price_source = WasmPriceSourceUnchecked::AstroportSpot {
        pair_address,
        route_assets: route_assets.iter().map(|&s| s.to_string()).collect(),
    };

    // Execute SetPriceSource
    robot
        .add_denom_precision_to_coin_registry("uatom", 6, admin)
        .add_denom_precision_to_coin_registry("uosmo", 6, admin)
        .set_price_source("uion", fixed_source(), admin)
        .set_price_source("uatom", price_source.clone(), admin)
        .assert_price_source("uatom", price_source);
}

#[test_case(&["uatom","uosmo"], "uosmo", &[] ; "no route, base_denom in pair")]
#[test_case(&["uatom","uosmo"], "USD", &[] => panics; "no route, base_denom not in pair")]
#[test_case(&["uatom","uosmo"], "uosmo", &["uusd"] => panics; "route asset does not exist")]
#[test_case(&["uatom","uosmo"], "uosmo", &["uosmo"]; "route equal to base_denom")]
#[test_case(&["uatom","uosmo"], "uosmo", &["uion"] => panics; "route with non-base existing asset, not in pair")]
#[test_case(&["uatom","uion"], "uosmo", &["uion"]; "route with non-base existing asset, in pair")]
fn test_validate_astroport_twap_price_source(
    pair_denoms: &[&str; 2],
    base_denom: &str,
    route_assets: &[&str],
) {
    let runner = get_test_runner();
    let admin = &runner.init_accounts()[0];
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, Some(base_denom));

    let initial_liq: [Uint128; 2] =
        [10000000000000000000000u128.into(), 1000000000000000000000u128.into()];
    let (pair_address, _lp_token_addr) = robot.create_astroport_pair(
        PairType::Xyk {},
        [native_info(pair_denoms[0]), native_info(pair_denoms[1])],
        None,
        admin,
        Some(initial_liq),
    );

    let price_source = WasmPriceSourceUnchecked::AstroportTwap {
        pair_address,
        route_assets: route_assets.iter().map(|&s| s.to_string()).collect(),
        tolerance: 5,
        window_size: 100,
    };

    // Execute SetPriceSource
    robot
        .add_denom_precision_to_coin_registry("uatom", 6, admin)
        .add_denom_precision_to_coin_registry("uosmo", 6, admin)
        .set_price_source("uion", fixed_source(), admin)
        .set_price_source("uatom", price_source.clone(), admin)
        .assert_price_source("uatom", price_source);
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
        [native_info("uatom"), native_info("uosmo")],
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
        .set_price_source("uatom", price_source, admin)
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
        [native_info("uatom"), native_info("uosmo")],
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
        .set_price_source("uatom", price_source, admin)
        .assert_price("uatom", expected_price);
}

#[test_case(5, 100; "Query TWAP price without route asset, XYK")]
fn test_query_astroport_twap_price_without_route_asset_xyk(tolerance: u64, window_size: u64) {
    let runner = get_test_runner();
    let admin = &runner.init_accounts()[0];
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, Some("uosmo"));

    let initial_liq: [Uint128; 2] =
        [10000000000000000000000u128.into(), 1000000000000000000000u128.into()]; // price 0.1 or 10
    let (pair_address, _lp_token_addr) = robot.create_astroport_pair(
        PairType::Xyk {},
        [native_info("uatom"), native_info("uosmo")],
        None,
        admin,
        Some(initial_liq),
    );
    let reserves = robot
        .add_denom_precision_to_coin_registry("uatom", 6, admin)
        .add_denom_precision_to_coin_registry("uosmo", 6, admin)
        .query_pool(&pair_address)
        .assets;
    let initial_price = Decimal::from_ratio(reserves[1].amount, reserves[0].amount);

    let price_source = WasmPriceSourceUnchecked::AstroportTwap {
        pair_address: pair_address.clone(),
        route_assets: vec![],
        tolerance,
        window_size,
    };

    let reserves = robot
        .set_price_source("uatom", price_source, admin)
        .record_twap_snapshots(&["uatom"], admin)
        .increase_time(window_size / 2)
        .swap_on_astroport_pair(
            &pair_address,
            native_asset("uosmo", 10000000000000000000u128),
            None,
            None,
            Some(Decimal::from_ratio(1u128, 2u128)),
            admin,
        )
        .query_pool(&pair_address)
        .assets;
    let price_after_swap = Decimal::from_ratio(reserves[1].amount, reserves[0].amount);

    let price_precision: Uint128 = Uint128::from(10_u128.pow(8));
    let expected_price = Decimal::from_ratio(
        (initial_price + price_after_swap) * Decimal::from_ratio(1u128, 2u128) * price_precision,
        price_precision,
    );

    robot
        .record_twap_snapshots(&["uatom"], admin)
        .increase_time(window_size / 2)
        .assert_price("uatom", expected_price);
}

#[test_case(5, 100; "Query TWAP price without route asset, StableSwap")]
fn test_query_astroport_twap_price_without_route_asset_stableswap(
    tolerance: u64,
    window_size: u64,
) {
    let runner = get_test_runner();
    let admin = &runner.init_accounts()[0];
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, Some("uosmo"));

    let pair_type = PairType::Stable {};
    let initial_liq: [Uint128; 2] =
        [10000000000000000000000u128.into(), 1000000000000000000000u128.into()]; // price 0.1 or 10
    let init_params = astro_init_params(&pair_type);
    let (pair_address, _lp_token_addr) = robot.create_astroport_pair(
        pair_type,
        [native_info("uatom"), native_info("uosmo")],
        init_params,
        admin,
        Some(initial_liq),
    );
    let initial_price = robot
        .add_denom_precision_to_coin_registry("uatom", 6, admin)
        .add_denom_precision_to_coin_registry("uosmo", 6, admin)
        .query_price_via_simulation(&pair_address, "uatom");

    let price_source = WasmPriceSourceUnchecked::AstroportTwap {
        pair_address: pair_address.clone(),
        route_assets: vec![],
        tolerance,
        window_size,
    };

    let price_after_swap = robot
        .set_price_source("uatom", price_source, admin)
        .record_twap_snapshots(&["uatom"], admin)
        .increase_time(window_size / 2)
        .swap_on_astroport_pair(
            &pair_address,
            native_asset("uosmo", 10000000000000000000u128),
            None,
            None,
            Some(Decimal::from_ratio(1u128, 2u128)),
            admin,
        )
        .query_price_via_simulation(&pair_address, "uatom");

    let price_precision: Uint128 = Uint128::from(10_u128.pow(8));
    let expected_price = Decimal::from_ratio(
        (initial_price + price_after_swap) * Decimal::from_ratio(1u128, 2u128) * price_precision,
        price_precision,
    );

    robot
        .record_twap_snapshots(&["uatom"], admin)
        .increase_time(window_size / 2)
        .assert_price_almost_equal("uatom", expected_price, Decimal::percent(1));
}
