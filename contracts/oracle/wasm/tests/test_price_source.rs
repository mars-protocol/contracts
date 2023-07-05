use std::str::FromStr;

use astroport::factory::PairType;
use cosmwasm_std::{
    from_binary,
    testing::{mock_dependencies, mock_env},
    Addr, Decimal, Uint128,
};
use cw_it::{
    astroport::{
        robot::AstroportTestRobot,
        utils::{native_asset, native_info},
    },
    robot::TestRobot,
    test_tube::Account,
};
use cw_storage_plus::Map;
use mars_oracle_base::{pyth::PriceIdentifier, ContractError, PriceSourceUnchecked};
use mars_oracle_wasm::{
    contract::entry::{self, execute},
    WasmPriceSource, WasmPriceSourceChecked, WasmPriceSourceUnchecked,
};
use mars_red_bank_types::oracle::{ExecuteMsg, PriceResponse, QueryMsg};

const ONE: Decimal = Decimal::one();
const TWO: Decimal = Decimal::new(Uint128::new(2_000_000_000_000_000_000u128));
const DEFAULT_LIQ: [u128; 2] = [10000000000000000000000u128, 1000000000000000000000u128];

use mars_testing::{
    mock_env_at_block_time, mock_info,
    test_runner::get_test_runner,
    wasm_oracle::{
        astro_init_params, fixed_source, get_contracts, setup_test,
        validate_and_query_astroport_spot_price_source,
        validate_and_query_astroport_twap_price_source, WasmOracleTestRobot,
    },
};
use pyth_sdk_cw::{Price, PriceFeed, PriceFeedResponse};
use test_case::test_case;

mod helpers;

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
#[test_case(PairType::Xyk {}, &["uatom","uosmo"], "uosmo", &[], 5,0 => panics; "Zero window size")]
#[test_case(PairType::Xyk {}, &["uatom","uosmo"], "uosmo", &[], 0,5; "Zero tolerance")]
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
fn test_query_astroport_twap_price_with_only_one_snapshot() {
    let base_denom = "uosmo";
    let runner = get_test_runner();
    let admin = &runner.init_accounts()[0];
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, Some(base_denom));

    let pair_type = PairType::Xyk {};
    let pair_denoms = ["uatom", "uosmo"];

    let initial_liq: [Uint128; 2] =
        DEFAULT_LIQ.iter().map(|x| Uint128::from(*x)).collect::<Vec<_>>().try_into().unwrap();
    let (pair_address, _lp_token_addr) = robot.create_astroport_pair(
        pair_type.clone(),
        [native_info(pair_denoms[0]), native_info(pair_denoms[1])],
        astro_init_params(&pair_type),
        admin,
        Some(initial_liq),
    );

    let price_source = WasmPriceSourceUnchecked::AstroportTwap {
        pair_address,
        route_assets: vec![],
        tolerance: 3,
        window_size: 2,
    };

    robot
        .add_denom_precision_to_coin_registry(pair_denoms[0], 6, admin)
        .add_denom_precision_to_coin_registry(pair_denoms[1], 6, admin)
        .add_denom_precision_to_coin_registry(base_denom, 6, admin)
        .set_price_source(pair_denoms[0], price_source.clone(), admin)
        .assert_price_source(pair_denoms[0], price_source)
        .record_twap_snapshots(&[pair_denoms[0]], admin);

    let err = robot
        .wasm()
        .query::<_, mars_red_bank_types::oracle::PriceResponse>(
            &robot.mars_oracle_contract_addr,
            &QueryMsg::Price {
                denom: "uatom".to_string(),
            },
        )
        .unwrap_err();

    assert!(err.to_string().contains("There needs to be at least two TWAP snapshots"));
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

#[test]
fn querying_pyth_price_if_publish_price_too_old() {
    let runner = get_test_runner();
    let robot = WasmOracleTestRobot::new(
        &runner,
        get_contracts(&get_test_runner()),
        &get_test_runner().init_accounts()[0],
        None,
    );

    let mut deps = helpers::setup_test(&robot.astroport_contracts.factory.address);

    // price source used to convert USD to base_denom
    helpers::set_price_source(
        deps.as_mut(),
        "usd",
        WasmPriceSourceUnchecked::Fixed {
            price: Decimal::from_str("1000000").unwrap(),
        },
    );

    let price_id = PriceIdentifier::from_hex(
        "61226d39beea19d334f17c2febce27e12646d84675924ebb02b9cdaea68727e3",
    )
    .unwrap();

    let max_staleness = 30u64;
    helpers::set_price_source(
        deps.as_mut(),
        "uatom",
        WasmPriceSourceUnchecked::Pyth {
            contract_addr: "pyth_contract_addr".to_string(),
            price_feed_id: price_id,
            max_staleness,
            denom_decimals: 6,
        },
    );

    let price_publish_time = 1677157333u64;
    let ema_price_publish_time = price_publish_time + max_staleness;
    deps.querier.set_pyth_price(
        price_id,
        PriceFeedResponse {
            price_feed: PriceFeed::new(
                price_id,
                Price {
                    price: 1371155677,
                    conf: 646723,
                    expo: -8,
                    publish_time: price_publish_time as i64,
                },
                Price {
                    price: 1365133270,
                    conf: 574566,
                    expo: -8,
                    publish_time: ema_price_publish_time as i64,
                },
            ),
        },
    );

    let res_err = entry::query(
        deps.as_ref(),
        mock_env_at_block_time(price_publish_time + max_staleness + 1u64),
        QueryMsg::Price {
            denom: "uatom".to_string(),
        },
    )
    .unwrap_err();
    assert_eq!(
        res_err,
        ContractError::InvalidPrice {
            reason:
                "current price publish time is too old/stale. published: 1677157333, now: 1677157364"
                    .to_string()
        }
    );
}

#[test]
fn querying_pyth_price_if_signed() {
    let runner = get_test_runner();
    let robot = WasmOracleTestRobot::new(
        &runner,
        get_contracts(&get_test_runner()),
        &get_test_runner().init_accounts()[0],
        None,
    );

    let mut deps = helpers::setup_test(&robot.astroport_contracts.factory.address);

    // price source used to convert USD to base_denom
    helpers::set_price_source(
        deps.as_mut(),
        "usd",
        WasmPriceSourceUnchecked::Fixed {
            price: Decimal::from_str("1000000").unwrap(),
        },
    );

    let price_id = PriceIdentifier::from_hex(
        "61226d39beea19d334f17c2febce27e12646d84675924ebb02b9cdaea68727e3",
    )
    .unwrap();

    let max_staleness = 30u64;
    helpers::set_price_source(
        deps.as_mut(),
        "uatom",
        WasmPriceSourceUnchecked::Pyth {
            contract_addr: "pyth_contract_addr".to_string(),
            price_feed_id: price_id,
            max_staleness,
            denom_decimals: 6,
        },
    );

    let publish_time = 1677157333u64;
    deps.querier.set_pyth_price(
        price_id,
        PriceFeedResponse {
            price_feed: PriceFeed::new(
                price_id,
                Price {
                    price: -1371155677,
                    conf: 646723,
                    expo: -8,
                    publish_time: publish_time as i64,
                },
                Price {
                    price: -1365133270,
                    conf: 574566,
                    expo: -8,
                    publish_time: publish_time as i64,
                },
            ),
        },
    );

    let res_err = entry::query(
        deps.as_ref(),
        mock_env_at_block_time(publish_time),
        QueryMsg::Price {
            denom: "uatom".to_string(),
        },
    )
    .unwrap_err();
    assert_eq!(
        res_err,
        ContractError::InvalidPrice {
            reason: "price can't be <= 0".to_string()
        }
    );
}

#[test]
fn querying_pyth_price_successfully() {
    let runner = get_test_runner();
    let robot = WasmOracleTestRobot::new(
        &runner,
        get_contracts(&get_test_runner()),
        &get_test_runner().init_accounts()[0],
        None,
    );

    let mut deps = helpers::setup_test(&robot.astroport_contracts.factory.address);

    // price source used to convert USD to base_denom
    helpers::set_price_source(
        deps.as_mut(),
        "usd",
        WasmPriceSourceUnchecked::Fixed {
            price: Decimal::from_str("1000000").unwrap(),
        },
    );

    let price_id = PriceIdentifier::from_hex(
        "61226d39beea19d334f17c2febce27e12646d84675924ebb02b9cdaea68727e3",
    )
    .unwrap();

    let max_staleness = 30u64;
    helpers::set_price_source(
        deps.as_mut(),
        "uatom",
        WasmPriceSourceUnchecked::Pyth {
            contract_addr: "pyth_contract_addr".to_string(),
            price_feed_id: price_id,
            max_staleness,
            denom_decimals: 6,
        },
    );

    let publish_time = 1677157333u64;

    // exp < 0
    deps.querier.set_pyth_price(
        price_id,
        PriceFeedResponse {
            price_feed: PriceFeed::new(
                price_id,
                Price {
                    price: 1021000,
                    conf: 50000,
                    expo: -4,
                    publish_time: publish_time as i64,
                },
                Price {
                    price: 1000000,
                    conf: 40000,
                    expo: -4,
                    publish_time: publish_time as i64,
                },
            ),
        },
    );

    let res = entry::query(
        deps.as_ref(),
        mock_env_at_block_time(publish_time),
        QueryMsg::Price {
            denom: "uatom".to_string(),
        },
    )
    .unwrap();
    let res: PriceResponse = from_binary(&res).unwrap();
    assert_eq!(res.price, Decimal::from_ratio(1021000u128, 10000u128));

    // exp > 0
    deps.querier.set_pyth_price(
        price_id,
        PriceFeedResponse {
            price_feed: PriceFeed::new(
                price_id,
                Price {
                    price: 102,
                    conf: 5,
                    expo: 3,
                    publish_time: publish_time as i64,
                },
                Price {
                    price: 100,
                    conf: 4,
                    expo: 3,
                    publish_time: publish_time as i64,
                },
            ),
        },
    );

    let res = entry::query(
        deps.as_ref(),
        mock_env_at_block_time(publish_time),
        QueryMsg::Price {
            denom: "uatom".to_string(),
        },
    )
    .unwrap();
    let res: PriceResponse = from_binary(&res).unwrap();
    assert_eq!(res.price, Decimal::from_ratio(102000u128, 1u128));
}

#[test]
fn setting_price_source_pyth_if_missing_usd() {
    let runner = get_test_runner();
    let robot = WasmOracleTestRobot::new(
        &runner,
        get_contracts(&get_test_runner()),
        &get_test_runner().init_accounts()[0],
        None,
    );

    let mut deps = helpers::setup_test(&robot.astroport_contracts.factory.address);

    let price_id = PriceIdentifier::from_hex(
        "61226d39beea19d334f17c2febce27e12646d84675924ebb02b9cdaea68727e3",
    )
    .unwrap();

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "uatom".to_string(),
            price_source: WasmPriceSourceUnchecked::Pyth {
                contract_addr: "new_pyth_contract_addr".to_string(),
                price_feed_id: price_id,
                max_staleness: 30,
                denom_decimals: 8,
            },
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "missing price source for usd".to_string()
        }
    );
}
