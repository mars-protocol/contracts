use std::str::FromStr;

use astroport::{factory::PairType, pair_concentrated::ConcentratedPoolParams};
use cosmwasm_std::{
    coin, from_json,
    testing::{mock_dependencies, mock_env},
    Addr, Decimal, Decimal256, Empty, Isqrt, Uint128, Uint256,
};
use cw_it::{
    astroport::{
        robot::AstroportTestRobot,
        utils::{native_asset, native_info},
    },
    robot::TestRobot,
    test_tube::{Account, Module, Wasm},
    traits::{CwItRunner, DEFAULT_COIN_AMOUNT},
};
use cw_storage_plus::Map;
use mars_oracle_base::{redemption_rate::RedemptionRate, ContractError, PriceSourceUnchecked};
use mars_oracle_wasm::{
    contract::entry::{self, execute},
    AstroportTwap, WasmPriceSource, WasmPriceSourceChecked, WasmPriceSourceUnchecked,
};
use mars_types::oracle::{ExecuteMsg, PriceResponse, QueryMsg};
use pyth_sdk_cw::PriceIdentifier;

const ONE: Decimal = Decimal::one();
const TWO: Decimal = Decimal::new(Uint128::new(2_000_000_000_000_000_000u128));
const DEFAULT_LIQ: [u128; 2] = [10000000000000000000000u128, 1000000000000000000000u128];

use mars_oracle_wasm::lp_pricing::{
    compute_pcl_lp_price, compute_pcl_lp_price_model, compute_pcl_lp_price_real,
    compute_ss_lp_price,
};
use mars_testing::{
    mock_env_at_block_time, mock_info,
    test_runner::get_test_runner,
    wasm_oracle::{
        astro_init_params, fixed_source, get_contracts, setup_test,
        validate_and_query_astroport_spot_price_source,
        validate_and_query_astroport_twap_price_source, WasmOracleTestRobot,
        STRIDE_TRANSFER_CHANNEL_ID,
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
fn display_lsd_price_source() {
    let ps = WasmPriceSourceChecked::Lsd {
        transitive_denom: "other_denom".to_string(),
        twap: AstroportTwap {
            pair_address: Addr::unchecked("astro_addr"),
            window_size: 101,
            tolerance: 16,
        },
        redemption_rate: RedemptionRate {
            contract_addr: Addr::unchecked("redemption_addr"),
            max_staleness: 1234,
        },
    };
    assert_eq!(ps.to_string(), "lsd:other_denom:astro_addr:101:16:redemption_addr:1234")
}

#[test]
fn display_lp_token_price_source() {
    let ps = WasmPriceSourceChecked::XykLiquidityToken {
        pair_address: Addr::unchecked(
            "neutron1e22zh5p8meddxjclevuhjmfj69jxfsa8uu3jvht72rv9d8lkhves6t8veq",
        ),
    };
    assert_eq!(
        ps.to_string(),
        "xyk_liquidity_token:neutron1e22zh5p8meddxjclevuhjmfj69jxfsa8uu3jvht72rv9d8lkhves6t8veq"
    )
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

#[test_case(PairType::Xyk {}, &["usttia","utia"], None, DEFAULT_LIQ, 5, 100, true, false => panics "missing price source for"; "missing transitive denom price source")]
#[test_case(PairType::Xyk {}, &["usttia","utia"], Some(Decimal::from_ratio(1242u128, 10u128)), DEFAULT_LIQ, 100, 100, true, false => panics "tolerance must be less than window size"; "tolerance equal to window size")]
#[test_case(PairType::Xyk {}, &["usttia","utia"], Some(Decimal::from_ratio(1242u128, 10u128)), DEFAULT_LIQ, 101, 100, true, false => panics "tolerance must be less than window size"; "tolerance larger than window size")]
#[test_case(PairType::Xyk {}, &["usttia","utia"], Some(Decimal::from_ratio(1242u128, 10u128)), DEFAULT_LIQ, 0, 1, true, false => panics "window_size must be greater than 1"; "window size equal to 1")]
#[test_case(PairType::Xyk {}, &["usttia","utia"], Some(Decimal::from_ratio(1242u128, 10u128)), DEFAULT_LIQ, 5, 100, false, false => panics "redemption rate update time is too old/stale"; "XYK, redemption rate too old")]
#[test_case(PairType::Xyk {}, &["usttia","utia"], Some(Decimal::from_ratio(1242u128, 10u128)), DEFAULT_LIQ, 5, 100, true, true; "XYK, staleness valid, rr gt twap")]
#[test_case(PairType::Xyk {}, &["usttia","utia"], Some(Decimal::from_ratio(1242u128, 10u128)), DEFAULT_LIQ, 5, 100, true, false; "XYK, staleness valid, rr lt twap")]
#[test_case(PairType::Stable { }, &["usttia","utia"], Some(Decimal::from_ratio(2242u128, 10u128)), DEFAULT_LIQ, 10, 200, true, true; "Stable, staleness valid, rr gt twap")]
#[test_case(PairType::Stable {}, &["usttia","utia"], Some(Decimal::from_ratio(2242u128, 10u128)), DEFAULT_LIQ, 10, 200, true, false; "Stable, staleness valid, rr lt twap")]
#[test_case(PairType::Custom("concentrated".to_string()), &["usttia","utia"], Some(Decimal::from_ratio(5242u128, 10u128)), [145692686804, 175998046105], 10, 200, true, true; "Concentrated, staleness valid, rr gt twap")]
#[test_case(PairType::Custom("concentrated".to_string()), &["usttia","utia"], Some(Decimal::from_ratio(5242u128, 10u128)), [145692686804, 175998046105], 10, 200, true, false; "Concentrated, staleness valid, rr lt twap")]
pub fn validate_and_query_lsd_price_source(
    pair_type: PairType,
    pair_denoms: &[&str; 2],
    other_asset_price: Option<Decimal>,
    initial_liq: [u128; 2],
    tolerance: u64,
    window_size: u64,
    rr_staleness_valid: bool,
    rr_value_gt_twap: bool,
) {
    let rr_max_staleness = 43200u64; // 12 hours

    let primary_denom = pair_denoms[0];
    let other_denom = pair_denoms[1];

    // Convert denoms to IBC denoms (redemption rate metric is pushed as native denom but queried as IBC denom)
    let primary_ibc_denom =
        ica_oracle::helpers::denom_trace_to_hash(primary_denom, STRIDE_TRANSFER_CHANNEL_ID)
            .unwrap();
    let other_ibc_denom =
        ica_oracle::helpers::denom_trace_to_hash(other_denom, STRIDE_TRANSFER_CHANNEL_ID).unwrap();
    let primary_ibc_denom = primary_ibc_denom.as_str();
    let other_ibc_denom = other_ibc_denom.as_str();

    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let admin = &runner
        .init_account(&[
            coin(DEFAULT_COIN_AMOUNT, primary_ibc_denom),
            coin(DEFAULT_COIN_AMOUNT, other_ibc_denom),
        ])
        .unwrap();
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, Some("uusd"));

    let (pair_address, _lp_token_addr) = robot.create_astroport_pair(
        pair_type.clone(),
        &[native_info(primary_ibc_denom), native_info(other_ibc_denom)],
        astro_init_params(&pair_type),
        admin,
        Some(&initial_liq),
        Some(&[6, 6]),
    );
    let initial_price = robot.query_price_via_simulation(&pair_address, primary_ibc_denom);

    let stride_contract_addr =
        robot.stride_contract_addr.clone().expect("Stride ica oracle contract not found");
    let price_source = WasmPriceSourceUnchecked::Lsd {
        transitive_denom: other_ibc_denom.to_string(),
        twap: AstroportTwap {
            pair_address: pair_address.clone(),
            tolerance,
            window_size,
        },
        redemption_rate: RedemptionRate {
            contract_addr: stride_contract_addr,
            max_staleness: rr_max_staleness,
        },
    };

    let other_asset_price_source = if let Some(other_asset_price) = other_asset_price {
        vec![(other_ibc_denom, fixed_source(other_asset_price))]
    } else {
        vec![]
    };

    println!("Swap amount: {}", initial_liq[1] / 1000000);

    let price_after_swap = robot
        .set_price_sources(other_asset_price_source, admin)
        .set_price_source(primary_ibc_denom, price_source.clone(), admin)
        .assert_price_source(primary_ibc_denom, price_source)
        .record_twap_snapshots(&[primary_ibc_denom], admin)
        .increase_time(window_size + tolerance)
        .swap_on_astroport_pair(
            &pair_address,
            native_asset(other_ibc_denom, initial_liq[1] / 1000000),
            None,
            None,
            Some(Decimal::from_ratio(1u128, 2u128)),
            admin,
        )
        .query_price_via_simulation(&pair_address, primary_ibc_denom);

    let price_precision: Uint128 = Uint128::from(10_u128.pow(8));
    let mut expected_price = Decimal::from_ratio(
        (initial_price + price_after_swap) * Decimal::from_ratio(1u128, 2u128) * price_precision,
        price_precision,
    );

    // Configure redemption rate value to be greater or less than TWAP
    let rr_value = if rr_value_gt_twap {
        expected_price.checked_mul(Decimal::percent(105)).unwrap()
    } else {
        expected_price = expected_price.checked_mul(Decimal::percent(95)).unwrap();
        expected_price
    };

    if let Some(other_asset_price) = other_asset_price {
        expected_price *= other_asset_price
    }

    let robot = robot
        .record_twap_snapshots(&[primary_ibc_denom], admin)
        .increase_time(window_size + tolerance);

    let block_time_nanos = robot.runner().query_block_time_nanos();
    let block_time_sec = block_time_nanos / 1_000_000_000;

    // Configure staleness to be valid or invalid
    let block_time_sec = if rr_staleness_valid {
        block_time_sec - rr_max_staleness + 100
    } else {
        block_time_sec - rr_max_staleness - 100
    };

    robot
        .set_redemption_rate_metric(primary_denom, rr_value, block_time_sec, block_time_sec, admin) // block_height doesn't matter here
        .assert_redemption_rate(primary_ibc_denom, rr_value)
        .assert_price_almost_equal(primary_ibc_denom, expected_price, Decimal::percent(1));
}

#[test_case(PairType::Xyk {}, &["uatom","untrn"], Some(Decimal::from_str("8.86506356").unwrap()), Some(Decimal::from_str("0.97696221").unwrap()), [1171210862745u128, 12117922358503u128], &[6,6]; "XYK, 6:6 decimals")]
#[test_case(PairType::Xyk {}, &["untrn","ueth"], Some(Decimal::from_str("0.85676231").unwrap()), Some(Decimal::from_str("0.000000003192778061").unwrap()), [291397962796u128, 65345494060528260316u128], &[6,18]; "XYK, 6:18 decimals")]
#[test_case(PairType::Xyk {}, &["ueth","udydx"], Some(Decimal::from_str("0.000000003195385").unwrap()), Some(Decimal::from_str("0.00000000000238175").unwrap()), DEFAULT_LIQ, &[18,18]; "XYK, 18:18 decimals")]
#[test_case(PairType::Stable {  }, &["utia","ustia"], Some(Decimal::one()), Some(Decimal::one()), DEFAULT_LIQ, &[6,6] => panics "Invalid price source: expecting pair contract14 to be xyk pool; found stable"; "XYK required, found StableSwap")]
#[test_case(PairType::Custom("concentrated".to_string()), &["utia","ustia"], Some(Decimal::one()), Some(Decimal::one()), [145692686804, 175998046105], &[6,6] => panics "Invalid price source: expecting pair contract14 to be xyk pool; found custom-concentrated"; "XYK required, found PCL")]
#[test_case(PairType::Xyk {}, &["uatom","untrn"], None, None, DEFAULT_LIQ, &[6,6] => panics "Invalid price source: missing price source for uatom"; "XYK, missing price source for both assets")]
#[test_case(PairType::Xyk {}, &["uatom","untrn"], None, Some(Decimal::one()), DEFAULT_LIQ, &[6,6] => panics "Invalid price source: missing price source for uatom"; "XYK, missing price source for first asset")]
#[test_case(PairType::Xyk {}, &["uatom","untrn"], Some(Decimal::one()), None, DEFAULT_LIQ, &[6,6] => panics "Invalid price source: missing price source for untrn"; "XYK, missing price source for second asset")]
pub fn test_validate_and_query_astroport_xyk_lp_price_source(
    pair_type: PairType,
    pair_denoms: &[&str; 2],
    primary_asset_price: Option<Decimal>,
    other_asset_price: Option<Decimal>,
    initial_liq: [u128; 2],
    decimals: &[u8; 2],
) {
    let primary_denom = pair_denoms[0];
    let other_denom = pair_denoms[1];
    let lp_denom = format!("pair:{}-{}", pair_denoms[0], pair_denoms[1]);

    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let admin = &runner
        .init_account(&[
            coin(DEFAULT_COIN_AMOUNT, primary_denom),
            coin(DEFAULT_COIN_AMOUNT, other_denom),
        ])
        .unwrap();
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, Some("uusd"));

    let (pair_address, _lp_token_addr) = robot.create_astroport_pair(
        pair_type.clone(),
        &[native_info(primary_denom), native_info(other_denom)],
        astro_init_params(&pair_type),
        admin,
        Some(&initial_liq),
        Some(decimals),
    );

    let pool = robot.query_pool(&pair_address);
    let coin0 = pool.assets[0].to_coin().unwrap();
    let coin1 = pool.assets[1].to_coin().unwrap();
    let mut coin0_value = Uint256::zero();
    let mut coin1_value = Uint256::zero();

    let mut other_assets_price_sources = vec![];
    if let Some(price) = primary_asset_price {
        other_assets_price_sources.push((primary_denom, fixed_source(price)));

        coin0_value = Uint256::from_uint128(coin0.amount) * Decimal256::from(price);
    }
    if let Some(price) = other_asset_price {
        other_assets_price_sources.push((other_denom, fixed_source(price)));

        coin1_value = Uint256::from_uint128(coin1.amount) * Decimal256::from(price);
    }

    // Calculate expected price
    let pool_value_u256 = Uint256::from(2u8) * (coin0_value * coin1_value).isqrt();
    let pool_value_u128 = Uint128::try_from(pool_value_u256).unwrap();
    let expected_price = Decimal::from_ratio(pool_value_u128, pool.total_share);

    let price_source = WasmPriceSourceUnchecked::XykLiquidityToken {
        pair_address: pair_address.clone(),
    };

    // Set price sources and assert that the price is as expected
    robot
        .set_price_sources(other_assets_price_sources, admin)
        .set_price_source(&lp_denom, price_source.clone(), admin)
        .assert_price_source(&lp_denom, price_source)
        .assert_price_almost_equal(&lp_denom, expected_price, Decimal::percent(1));
}

#[test_case(PairType::Custom("concentrated".to_string()), &["uatom","untrn"], Some(Decimal::from_str("8.86506356").unwrap()), Some(Decimal::from_str("0.97696221").unwrap()), [1171210862745u128, 12117922358503u128], &[6,6], Some(Decimal::from_str("5.88585833583172000").unwrap()), Some(Decimal::from_str("5.89494461787180000").unwrap()); "PCL, 6:6 decimals")]
#[test_case(PairType::Custom("concentrated".to_string()), &["uatom","untrn"], Some(Decimal::from_str("821123123435412349.73564").unwrap()), Some(Decimal::from_str("0.97696221").unwrap()), [923752936745723845u128, 12117922358503u128], &[6,6], Some(Decimal::from_str("1791319358").unwrap()), Some(Decimal::from_str("225997634181761000000").unwrap()); "PCL, [6, 6] decimals Uint128 overflow)")]
#[test_case(PairType::Custom("concentrated".to_string()), &["uatom","untrn"], Some(Decimal::from_str("0.000000000585").unwrap()), Some(Decimal::from_str("0.0000000097696221").unwrap()), [34567u128, 67891u128], &[6,6], Some(Decimal::from_str("0.00000000478137514").unwrap()), Some(Decimal::from_str("0.00000001410918211").unwrap()); "PCL, [6, 6] decimals, rounding small numbers)")]
#[test_case(PairType::Custom("concentrated".to_string()), &["udydx","untrn"], Some(Decimal::from_str("3000").unwrap()), Some(Decimal::from_str("0.97696221").unwrap()), [92347562936745723845u128, 12117922358503u128], &[18,6], Some(Decimal::from_str("108275327").unwrap()), Some(Decimal::from_str("8251396252898.20").unwrap()); "PCL, [18, 6] decimals")]
#[test_case(PairType::Custom("concentrated".to_string()), &["udydx","ueth"], Some(Decimal::from_str("0.000000000002095907").unwrap()), Some(Decimal::from_str("0.000000003705405005").unwrap()), [230049283723446123784938u128,  134273643746123784938u128], &[18,18], Some(Decimal::from_str("176.25191391713600000").unwrap()), Some(Decimal::from_str("175.98").unwrap()); "PCL, [18, 18] decimals")]
#[test_case(PairType::Xyk{}, &["uatom","untrn"], Some(Decimal::from_str("8.86506356").unwrap()), Some(Decimal::from_str("0.97696221").unwrap()), [1171210862745u128, 12117922358503u128], &[6,6], None, None => panics "Invalid price source: expecting pair contract14 to be custom-concentrated pool; found xyk"; "PCL required, found XYK")]
#[test_case(PairType::Stable{}, &["uatom","untrn"], Some(Decimal::from_str("8.86506356").unwrap()), Some(Decimal::from_str("0.97696221").unwrap()), [1171210862745u128, 12117922358503u128], &[6,6], None, None => panics "Invalid price source: expecting pair contract14 to be custom-concentrated pool; found stable"; "PCL required, found Stable")]
#[test_case(PairType::Custom("concentrated".to_string()), &["uatom","untrn"], None, None, [1171210862745u128, 1171210862745u128], &[6,6], None, None => panics "Invalid price source: missing price source for uatom"; "PCL, missing price source for both assets")]
#[test_case(PairType::Custom("concentrated".to_string()), &["uatom","untrn"], None, Some(Decimal::one()), [1171210862745u128, 1171210862745u128], &[6,6], None, None => panics "Invalid price source: missing price source for uatom"; "PCL, missing price source for first asset")]
#[test_case(PairType::Custom("concentrated".to_string()), &["uatom","untrn"], Some(Decimal::one()), None, [1171210862745u128, 1171210862745u128], &[6,6], None, None => panics "Invalid price source: missing price source for untrn"; "PCL, missing price source for second asset")]
pub fn test_validate_and_query_astroport_pcl_lp_price_source(
    pair_type: PairType,
    pair_denoms: &[&str; 2],
    coin0_price: Option<Decimal>,
    coin1_price: Option<Decimal>,
    initial_liq: [u128; 2],
    decimals: &[u8; 2],
    expected_model_price: Option<Decimal>,
    expected_real_price: Option<Decimal>,
) {
    let primary_denom = pair_denoms[0];
    let secondary_denom = pair_denoms[1];
    let lp_denom = format!("pair:{}-{}", pair_denoms[0], pair_denoms[1]);

    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let admin = &runner
        .init_account(&[
            coin(DEFAULT_COIN_AMOUNT, primary_denom),
            coin(DEFAULT_COIN_AMOUNT, secondary_denom),
        ])
        .unwrap();

    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, Some("uusd"));

    let (pair_address, _lp_token_addr) = robot.create_astroport_pair(
        pair_type.clone(),
        &[native_info(primary_denom), native_info(secondary_denom)],
        astro_init_params(&pair_type),
        admin,
        Some(&initial_liq),
        Some(decimals),
    );

    let mut other_assets_price_sources = vec![];
    if let Some(price) = coin0_price {
        other_assets_price_sources.push((primary_denom, fixed_source(price)));
    }
    if let Some(price) = coin1_price {
        other_assets_price_sources.push((secondary_denom, fixed_source(price)));
    }

    let price_source = WasmPriceSourceUnchecked::PclLiquidityToken {
        pair_address: pair_address.clone(),
    };

    // Validate the price sources
    robot
        .set_price_sources(other_assets_price_sources.clone(), admin)
        .set_price_source(&lp_denom, price_source.clone(), admin)
        .assert_price_source(&lp_denom, price_source.clone());

    let pool = robot.query_pool(&pair_address);
    let curve_invariant = robot.query_pcl_curve_invariant(&pair_address);
    let pool_config = robot.query_astroport_config(&pair_address);

    let pool_params = from_json::<ConcentratedPoolParams>(pool_config.params.unwrap()).unwrap();

    let coin0_amount = pool.assets[0].to_coin().unwrap().amount;
    let coin1_amount = pool.assets[1].to_coin().unwrap().amount;

    let mut lp_token_price = Decimal::zero();
    let mut lp_price_model = Decimal::zero();
    let mut lp_price_real = Decimal::zero();

    // Prices have been validated before, so both are defined
    if let (Some(price0), Some(price1)) = (coin0_price, coin1_price) {
        lp_token_price = compute_pcl_lp_price(
            decimals[0],
            decimals[1],
            price0,
            price1,
            coin0_amount,
            coin1_amount,
            pool.total_share,
            pool_params.price_scale,
            curve_invariant,
        )
        .unwrap();

        lp_price_model = compute_pcl_lp_price_model(
            price0,
            price1,
            decimals[0],
            decimals[1],
            pool.total_share,
            pool_params.price_scale,
            curve_invariant,
        )
        .unwrap();

        lp_price_real =
            compute_pcl_lp_price_real(coin0_amount, coin1_amount, price0, price1, pool.total_share)
                .unwrap();
    };

    // Validate the queried price with the expected price
    robot.assert_price(&lp_denom, lp_token_price);

    if let Some(expected_model_price) = expected_model_price {
        robot.assert_prices_almost_equal(lp_price_model, expected_model_price, Decimal::percent(1));
    }

    if let Some(expected_real_price) = expected_real_price {
        robot.assert_prices_almost_equal(lp_price_real, expected_real_price, Decimal::percent(1));
    }
}

#[test_case(PairType::Stable{}, &["uusdc","uusdt"], Some(Decimal::from_str("0.9999").unwrap()), Some(Decimal::from_str("1.00001").unwrap()), [10912049231u128, 11242686517u128], &[6,6], Some(Decimal::from_str("1.00155249644").unwrap()); "SS, 6:6 decimals")]
#[test_case(PairType::Stable{}, &["uatom","untrn"], Some(Decimal::from_str("821123123432349.73564").unwrap()), Some(Decimal::from_str("721123123432349.73564").unwrap()), [923752936745723845u128, 12117922358503u128], &[6,6], Some(Decimal::from_str("721123123432349.0000000000000000").unwrap()); "SS, [6, 6] decimals Uint128 overflow)")]
#[test_case(PairType::Stable{}, &["uatom","untrn"], Some(Decimal::from_str("0.000000000585").unwrap()), Some(Decimal::from_str("0.0000000097696221").unwrap()), [34567u128, 67891u128], &[6,6], Some(Decimal::from_str("0.0000000005850000").unwrap()); "PCL, [6, 6] decimals, rounding small numbers)")]
#[test_case(PairType::Stable{}, &["uneth","ueth"], Some(Decimal::from_str("3605.405005").unwrap()), Some(Decimal::from_str("0.00000000370540501").unwrap()), [1909955u128, 1715278424796108660u128], &[6,18], Some(Decimal::from_str("0.0000000036054050").unwrap()); "PCL, [6, 18] decimals")]
#[test_case(PairType::Stable{}, &["usteth","ueth"], Some(Decimal::from_str("0.00000000370240501").unwrap()), Some(Decimal::from_str("0.00000000370540501").unwrap()), [1909955195744952147u128, 1715278424796108660u128], &[18,18], Some(Decimal::from_str("0.0000000037024050").unwrap()); "SS, [18, 18] decimals")]
#[test_case(PairType::Xyk{}, &["uatom","untrn"], Some(Decimal::from_str("8.86506356").unwrap()), Some(Decimal::from_str("0.97696221").unwrap()), [1171210862745u128, 12117922358503u128], &[6,6], None => panics "Invalid price source: expecting pair contract14 to be stable pool; found xyk"; "SS required, found XYK")]
#[test_case(PairType::Custom("concentrated".to_string()), &["uatom","untrn"], Some(Decimal::from_str("8.86506356").unwrap()), Some(Decimal::from_str("0.97696221").unwrap()), [1171210862745u128, 12117922358503u128], &[6,6], None => panics "Invalid price source: expecting pair contract14 to be stable pool; found custom-concentrated"; "SS required, found PCL")]
#[test_case(PairType::Stable{}, &["uatom","untrn"], None, None, [1171210862745u128, 1171210862745u128], &[6,6], None => panics "Invalid price source: missing price source for uatom"; "SS, missing price source for both assets")]
#[test_case(PairType::Stable{}, &["uatom","untrn"], None, Some(Decimal::one()), [1171210862745u128, 1171210862745u128], &[6,6], None => panics "Invalid price source: missing price source for uatom"; "SS, missing price source for first asset")]
#[test_case(PairType::Stable{}, &["uatom","untrn"], Some(Decimal::one()), None, [1171210862745u128, 1171210862745u128], &[6,6], None => panics "Invalid price source: missing price source for untrn"; "SS, missing price source for second asset")]
pub fn test_validate_and_query_astroport_ss_lp_price_source(
    pair_type: PairType,
    pair_denoms: &[&str; 2],
    coin0_price: Option<Decimal>,
    coin1_price: Option<Decimal>,
    initial_liq: [u128; 2],
    decimals: &[u8; 2],
    expected_price: Option<Decimal>,
) {
    let primary_denom = pair_denoms[0];
    let secondary_denom = pair_denoms[1];
    let lp_denom = format!("pair:{}-{}", pair_denoms[0], pair_denoms[1]);

    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let admin = &runner
        .init_account(&[
            coin(DEFAULT_COIN_AMOUNT, primary_denom),
            coin(DEFAULT_COIN_AMOUNT, secondary_denom),
        ])
        .unwrap();

    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, Some("uusd"));

    let (pair_address, _lp_token_addr) = robot.create_astroport_pair(
        pair_type.clone(),
        &[native_info(primary_denom), native_info(secondary_denom)],
        astro_init_params(&pair_type),
        admin,
        Some(&initial_liq),
        Some(decimals),
    );

    let mut other_assets_price_sources = vec![];
    if let Some(price) = coin0_price {
        other_assets_price_sources.push((primary_denom, fixed_source(price)));
    }
    if let Some(price) = coin1_price {
        other_assets_price_sources.push((secondary_denom, fixed_source(price)));
    }

    let price_source = WasmPriceSourceUnchecked::SsLiquidityToken {
        pair_address: pair_address.clone(),
    };

    // Validate the price sources
    robot
        .set_price_sources(other_assets_price_sources.clone(), admin)
        .set_price_source(&lp_denom, price_source.clone(), admin)
        .assert_price_source(&lp_denom, price_source.clone());

    let pool = robot.query_pool(&pair_address);
    let curve_invariant = robot.query_ss_curve_invariant(&pair_address);

    let mut lp_token_price = Decimal::zero();

    // Prices have been validated before, so both are defined
    if let (Some(price0), Some(price1)) = (coin0_price, coin1_price) {
        lp_token_price = compute_ss_lp_price(
            price0,
            price1,
            decimals[0],
            decimals[1],
            pool.total_share,
            curve_invariant,
        )
        .unwrap();
    };

    // Validate the queried price with the expected price
    robot.assert_price(&lp_denom, lp_token_price);

    if let Some(expected_price) = expected_price {
        robot.assert_prices_almost_equal(lp_token_price, expected_price, Decimal::percent(1));
    }
}
