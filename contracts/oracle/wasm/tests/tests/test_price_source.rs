use std::str::FromStr;

use astroport::factory::PairType;
use cosmwasm_std::{
    from_json,
    testing::{mock_dependencies, mock_env},
    Addr, Decimal, Empty, Uint128,
};
use cw_it::{
    astroport::{
        robot::AstroportTestRobot,
        utils::{native_asset, native_info},
    },
    robot::TestRobot,
    test_tube::{Account, Module, Wasm},
    traits::CwItRunner,
};
use cw_storage_plus::Map;
use mars_oracle_base::{ContractError, PriceSourceUnchecked};
use mars_oracle_wasm::{
    contract::entry::{self, execute},
    WasmPriceSource, WasmPriceSourceChecked, WasmPriceSourceUnchecked,
};
use mars_types::oracle::{ExecuteMsg, PriceResponse, QueryMsg};
use pyth_sdk_cw::PriceIdentifier;

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

use super::helpers;

#[test]
fn test_contract_initialization() {
    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let admin = &runner.init_default_account().unwrap();
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
    };
    assert_eq!(ps.to_string(), "astroport_spot:fake_addr.")
}

#[test]
fn display_spot_price_source_with_route() {
    let ps = WasmPriceSourceChecked::AstroportSpot {
        pair_address: Addr::unchecked("fake_addr"),
    };
    assert_eq!(ps.to_string(), "astroport_spot:fake_addr.")
}

#[test]
fn display_twap_price_source() {
    let ps = WasmPriceSourceChecked::AstroportTwap {
        pair_address: Addr::unchecked("fake_addr"),
        window_size: 100,
        tolerance: 10,
    };
    assert_eq!(ps.to_string(), "astroport_twap:fake_addr. Window Size: 100. Tolerance: 10.")
}

#[test]
fn display_twap_price_source_with_route() {
    let ps = WasmPriceSourceChecked::AstroportTwap {
        pair_address: Addr::unchecked("fake_addr"),
        window_size: 100,
        tolerance: 10,
    };
    assert_eq!(ps.to_string(), "astroport_twap:fake_addr. Window Size: 100. Tolerance: 10.")
}

#[test]
fn validate_fixed_price_source() {
    let ps = WasmPriceSource::Fixed {
        price: Decimal::from_ratio(1u128, 2u128),
    };
    let deps = mock_dependencies();
    let price_sources = Map::new("price_sources");
    let denom = "uosmo";
    let base_denom = "uusd";
    let res = ps.validate(&deps.as_ref(), denom, base_denom, &price_sources);
    assert!(res.is_ok());
}

#[test]
fn test_set_price_source_fixed() {
    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let admin = &runner.init_default_account().unwrap();
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
    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let admin = &runner.init_default_account().unwrap();
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
    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let admin = &runner.init_default_account().unwrap();
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, None);
    let denom = "uusd";
    let price_source = WasmPriceSourceUnchecked::Fixed {
        price: ONE,
    };

    // Set price and then query it
    robot.set_price_source(denom, price_source, admin).assert_price(denom, ONE);
}

#[test]
#[should_panic(expected = "cannot set price source for base denom")]
/// base_denom is set in instantiate of the contract. You should not be able to change it.
fn cannot_set_base_denom_price_source() {
    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let admin = &runner.init_default_account().unwrap();
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, Some("uusd"));
    let denom = "uusd";
    let price_source = WasmPriceSourceUnchecked::Fixed {
        price: ONE,
    };

    // Set price, should fail
    robot.set_price_source(denom, price_source, admin);
}

#[test_case(PairType::Xyk {}, &["uatom","uosmo"], "uosmo", None, false, &[6,6]; "XYK, base_denom in pair")]
#[test_case(PairType::Xyk {}, &["uatom","uion"], "uosmo", Some(TWO), true, &[6,6]; "XYK, non-base asset in pair")]
#[test_case(PairType::Xyk {}, &["uatom","uosmo"], "USD", None, false, &[6,6] => panics "pair does not contain base denom and no price source is configured for the other denom"; "XYK, base_denom not in pair, no source for other asset")]
#[test_case(PairType::Xyk {}, &["uatom","uosmo"], "uosmo", None, false, &[6,8]; "XYK, base_denom in pair, 6:8 decimals")]
#[test_case(PairType::Xyk {}, &["uatom","uion"], "uosmo", Some(TWO), true, &[8,6]; "XYK, non-base asset in pair, 8:6 decimals")]
#[test_case(PairType::Stable {}, &["uatom","uosmo"], "uosmo", None, false, &[6,6]; "Stable, base_denom in pair")]
#[test_case(PairType::Stable {}, &["uatom","uion"], "uosmo", Some(TWO), true, &[6,6]; "Stable, non-base asset in pair")]
#[test_case(PairType::Stable {}, &["uatom","uosmo"], "USD", None, false, &[6,6] => panics; "Stable, base_denom not in pair, no source for other asset")]
#[test_case(PairType::Stable {}, &["uatom","uosmo"], "uosmo", None, false, &[6,8]; "Stable, base_denom in pair, 6:8 decimals")]
#[test_case(PairType::Stable {}, &["uatom","uion"], "uosmo", Some(TWO), true, &[6,8]; "Stable, non-base asset in pair, 6:8 decimals")]
pub fn test_validate_and_query_astroport_spot_price_source(
    pair_type: PairType,
    pair_denoms: &[&str; 2],
    base_denom: &str,
    other_asset_price: Option<Decimal>,
    register_second_price: bool,
    decimals: &[u8; 2],
) {
    validate_and_query_astroport_spot_price_source(
        pair_type,
        pair_denoms,
        base_denom,
        other_asset_price,
        &DEFAULT_LIQ,
        register_second_price,
        decimals,
    )
}

#[test_case(PairType::Xyk {}, &["uatom","uosmo"], "uosmo", None, false, 5, 100, DEFAULT_LIQ, &[6,6]; "XYK, base_denom in pair")]
#[test_case(PairType::Xyk {}, &["uatom","uion"], "uosmo", Some(TWO), true, 5, 100, DEFAULT_LIQ, &[6,6]; "XYK, non-base asset in pair")]
#[test_case(PairType::Xyk {}, &["uatom","uosmo"], "USD", None, false, 5, 100, DEFAULT_LIQ, &[6,6] => panics "pair does not contain base denom and no price source is configured for the other denom"; "XYK, base_denom not in pair, no source for other asset")]
#[test_case(PairType::Xyk {}, &["uatom","uosmo"], "uosmo", None, false, 5, 100, DEFAULT_LIQ, &[6,8]; "XYK, base_denom in pair, 6:8 decimals")]
#[test_case(PairType::Xyk {}, &["uatom","uion"], "uosmo", Some(TWO), true, 5, 100, DEFAULT_LIQ, &[8,6]; "XYK, non-base asset in pair, 8:6 decimals")]
#[test_case(PairType::Stable {}, &["uatom","uosmo"], "uosmo", None, false, 5, 100, DEFAULT_LIQ, &[6,6]; "Stable, base_denom in pair")]
#[test_case(PairType::Stable {}, &["uatom","uion"], "uosmo", Some(TWO), true, 5, 100, DEFAULT_LIQ, &[6,6]; "Stable, non-base asset in pair")]
#[test_case(PairType::Stable {}, &["uatom","uosmo"], "USD", None, false, 5, 100, DEFAULT_LIQ, &[6,6] => panics "pair does not contain base denom and no price source is configured for the other denom"; "Stable, base_denom not in pair, no source for other asset")]
#[test_case(PairType::Xyk {}, &["uatom","uosmo"], "uosmo", None, false, 0,0, DEFAULT_LIQ, &[6,6] => panics; "Zero window size")]
#[test_case(PairType::Xyk {}, &["uatom","uosmo"], "uosmo", None, false, 0,5, DEFAULT_LIQ, &[6,6]; "Zero tolerance")]
#[test_case(PairType::Xyk {}, &["uatom","uosmo"], "uosmo", None, false, 5,2, DEFAULT_LIQ, &[6,6] => panics "tolerance must be less than window size"; "tolerance larger than window size")]
#[test_case(PairType::Stable {}, &["uatom","uosmo"], "uosmo", None, false, 5, 100, DEFAULT_LIQ, &[6,8]; "Stable, base_denom in pair, 6:8 decimals")]
#[test_case(PairType::Stable {}, &["uatom","uosmo"], "uosmo", None, false, 5, 100, DEFAULT_LIQ, &[7,6]; "Stable, base_denom in pair, 7:6 decimals")]
#[test_case(PairType::Stable {}, &["uatom","uosmo"], "uosmo", None, false, 5, 100, DEFAULT_LIQ, &[8,7]; "Stable, base_denom in pair, 8:7 decimals")]
#[test_case(PairType::Stable {}, &["uatom","uosmo"], "uosmo", None, false, 5, 213378, [100000000000000000000, 1000000000000000000], &[7,5]; "Stable, base_denom in pair, 6:4 decimals, adjusted 1:1 price")]
#[test_case(PairType::Stable {}, &["uatom","uosmo"], "uosmo", None, false, 5, 1000, [1000000000000000000u128, 1000000000000000000000u128], &[5,9]; "Stable, base_denom in pair, 5:9 decimals, adjusted 1:1 price")]
#[test_case(PairType::Stable {}, &["uatom","uosmo"], "uosmo", None, false, 5, 1000, [10000000000000000000000u128, 100000000000000000u128], &[10,5]; "Stable, base_denom in pair, 10:5 decimals, adjusted 1:1 price")]
#[test_case(PairType::Custom("concentrated".to_string()), &["uatom","uosmo"], "uosmo", None, false, 5, 100, [145692686804, 175998046105], &[6,6]; "Concentrated, base_denom in pair")]
#[test_case(PairType::Custom("concentrated".to_string()), &["uatom","uion"], "uosmo", Some(TWO), true, 5, 100, [145692686804, 175998046105], &[6,6]; "Concentrated, non-base asset in pair")]
#[test_case(PairType::Custom("concentrated".to_string()), &["uatom","uosmo"], "USD", None, false, 5, 100, [145692686804, 175998046105], &[6,6] => panics "pair does not contain base denom and no price source is configured for the other denom"; "Concentrated, base_denom not in pair, no source for other asset")]
#[test_case(PairType::Custom("concentrated".to_string()), &["uatom","uosmo"], "uosmo", None, false, 5, 100, [145692686804, 175998046105], &[6,8]; "Concentrated, base_denom in pair, 6:8 decimals")]
fn test_validate_and_query_astroport_twap_price(
    pair_type: PairType,
    pair_denoms: &[&str; 2],
    base_denom: &str,
    other_asset_price: Option<Decimal>,
    register_second_price: bool,
    tolerance: u64,
    window_size: u64,
    initial_liq: [u128; 2],
    decimals: &[u8; 2],
) {
    validate_and_query_astroport_twap_price_source(
        pair_type,
        pair_denoms,
        base_denom,
        other_asset_price,
        register_second_price,
        tolerance,
        window_size,
        &initial_liq,
        decimals,
    )
}

#[test]
fn test_query_astroport_twap_price_with_only_one_snapshot() {
    let base_denom = "uosmo";
    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let admin = &runner.init_default_account().unwrap();
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, Some(base_denom));

    let pair_type = PairType::Xyk {};
    let pair_denoms = ["uatom", "uosmo"];

    let (pair_address, _lp_token_addr) = robot.create_astroport_pair(
        pair_type.clone(),
        &[native_info(pair_denoms[0]), native_info(pair_denoms[1])],
        astro_init_params(&pair_type),
        admin,
        Some(&DEFAULT_LIQ),
        None,
    );

    let price_source = WasmPriceSourceUnchecked::AstroportTwap {
        pair_address,
        tolerance: 3,
        window_size: 4,
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
        .query::<_, mars_types::oracle::PriceResponse>(
            &robot.mars_oracle_contract_addr,
            &QueryMsg::Price {
                denom: "uatom".to_string(),
                kind: None,
            },
        )
        .unwrap_err();

    assert!(err.to_string().contains("There needs to be at least two TWAP snapshots"));
}

#[test]
#[should_panic]
fn record_twap_snapshots_errors_on_non_twap_price_source() {
    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let admin = &runner.init_default_account().unwrap();
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, None);

    robot
        .set_price_source("uosmo", fixed_source(ONE), admin)
        .record_twap_snapshots(&["uosmo"], admin);
}

#[test]
fn record_twap_snapshot_does_not_save_when_less_than_tolerance_ago() {
    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let admin = &runner.init_default_account().unwrap();
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, Some("uosmo"));

    let (pair_address, _) = robot.create_default_astro_pair(admin);

    let price_source = WasmPriceSourceUnchecked::AstroportTwap {
        pair_address: pair_address.clone(),
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
    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let robot = WasmOracleTestRobot::new(
        &runner,
        get_contracts(&runner),
        &get_test_runner().init_default_account().unwrap(),
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
            max_confidence: Decimal::percent(12),
            max_deviation: Decimal::percent(14),
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
            kind: None,
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
    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let robot = WasmOracleTestRobot::new(
        &runner,
        get_contracts(&runner),
        &get_test_runner().init_default_account().unwrap(),
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
            max_confidence: Decimal::percent(12),
            max_deviation: Decimal::percent(14),
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
            kind: None,
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
    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let robot = WasmOracleTestRobot::new(
        &runner,
        get_contracts(&runner),
        &get_test_runner().init_default_account().unwrap(),
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
            max_confidence: Decimal::percent(12),
            max_deviation: Decimal::percent(14),
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
            kind: None,
        },
    )
    .unwrap();
    let res: PriceResponse = from_json(res).unwrap();
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
            kind: None,
        },
    )
    .unwrap();
    let res: PriceResponse = from_json(res).unwrap();
    assert_eq!(res.price, Decimal::from_ratio(102000u128, 1u128));
}

#[test]
fn setting_price_source_pyth_if_missing_usd() {
    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let robot = WasmOracleTestRobot::new(
        &runner,
        get_contracts(&runner),
        &get_test_runner().init_default_accounts().unwrap()[0],
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
                max_confidence: Decimal::percent(10),
                max_deviation: Decimal::percent(10),
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

#[test]
fn setting_price_source_pyth_with_invalid_params() {
    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let robot = WasmOracleTestRobot::new(
        &runner,
        get_contracts(&runner),
        &get_test_runner().init_default_accounts().unwrap()[0],
        None,
    );

    let mut deps = helpers::setup_test(&robot.astroport_contracts.factory.address);

    let mut set_price_source_pyth =
        |max_confidence: Decimal, max_deviation: Decimal, denom_decimals: u8| {
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info("owner"),
                ExecuteMsg::SetPriceSource {
                    denom: "uatom".to_string(),
                    price_source: WasmPriceSourceUnchecked::Pyth {
                        contract_addr: "pyth_contract_addr".to_string(),
                        price_feed_id: PriceIdentifier::from_hex(
                            "61226d39beea19d334f17c2febce27e12646d84675924ebb02b9cdaea68727e3",
                        )
                        .unwrap(),
                        max_staleness: 30,
                        max_confidence,
                        max_deviation,
                        denom_decimals,
                    },
                },
            )
        };

    // attempting to set max_confidence > 20%; should fail
    let err = set_price_source_pyth(Decimal::percent(21), Decimal::percent(6), 6).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "max_confidence must be in the range of <0;0.2>".to_string()
        }
    );

    // attempting to set max_deviation > 20%; should fail
    let err = set_price_source_pyth(Decimal::percent(5), Decimal::percent(21), 18).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "max_deviation must be in the range of <0;0.2>".to_string()
        }
    );

    // attempting to set denom_decimals > 18; should fail
    let err = set_price_source_pyth(Decimal::percent(5), Decimal::percent(20), 19).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "denom_decimals must be <= 18".to_string()
        }
    );
}

#[test]
fn twap_window_size_not_gt_tolerance() {
    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let admin = &runner.init_default_accounts().unwrap()[0];
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, None);

    let (pair_address, _) = robot.create_default_astro_pair(admin);

    let price_source = WasmPriceSourceUnchecked::AstroportTwap {
        pair_address,
        tolerance: 100,
        window_size: 100,
    };

    let wasm = Wasm::new(&runner);
    let msg = mars_types::oracle::ExecuteMsg::<_, Empty>::SetPriceSource {
        denom: "uatom".to_string(),
        price_source,
    };
    let err = wasm.execute(&robot.mars_oracle_contract_addr, &msg, &[], admin).unwrap_err();

    println!("{:?}", err);
    assert!(err.to_string().contains("tolerance must be less than window size"));
}
