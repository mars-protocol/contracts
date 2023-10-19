use cosmwasm_std::{Addr, Decimal};
use mars_oracle_osmosis::{
    DowntimeDetector, OsmosisPriceSourceChecked, RedemptionRate, Twap, TwapKind,
};
use osmosis_std::types::osmosis::downtimedetector::v1beta1::Downtime;
use pyth_sdk_cw::PriceIdentifier;

#[test]
fn display_downtime_detector() {
    let dd = DowntimeDetector {
        downtime: Downtime::Duration10m,
        recovery: 550,
    };
    assert_eq!(dd.to_string(), "Duration10m:550")
}

#[test]
fn display_fixed_price_source() {
    let ps = OsmosisPriceSourceChecked::Fixed {
        price: Decimal::from_ratio(1u128, 2u128),
    };
    assert_eq!(ps.to_string(), "fixed:0.5")
}

#[test]
fn display_spot_price_source() {
    let ps = OsmosisPriceSourceChecked::Spot {
        pool_id: 123,
    };
    assert_eq!(ps.to_string(), "spot:123")
}

#[test]
fn display_arithmetic_twap_price_source() {
    let ps = OsmosisPriceSourceChecked::ArithmeticTwap {
        pool_id: 123,
        window_size: 300,
        downtime_detector: None,
    };
    assert_eq!(ps.to_string(), "arithmetic_twap:123:300:None");

    let ps = OsmosisPriceSourceChecked::ArithmeticTwap {
        pool_id: 123,
        window_size: 300,
        downtime_detector: Some(DowntimeDetector {
            downtime: Downtime::Duration30m,
            recovery: 568,
        }),
    };
    assert_eq!(ps.to_string(), "arithmetic_twap:123:300:Some(Duration30m:568)");
}

#[test]
fn display_geometric_twap_price_source() {
    let ps = OsmosisPriceSourceChecked::GeometricTwap {
        pool_id: 123,
        window_size: 300,
        downtime_detector: None,
    };
    assert_eq!(ps.to_string(), "geometric_twap:123:300:None");

    let ps = OsmosisPriceSourceChecked::GeometricTwap {
        pool_id: 123,
        window_size: 300,
        downtime_detector: Some(DowntimeDetector {
            downtime: Downtime::Duration30m,
            recovery: 568,
        }),
    };
    assert_eq!(ps.to_string(), "geometric_twap:123:300:Some(Duration30m:568)");
}

#[test]
fn display_staked_geometric_twap_price_source() {
    let ps = OsmosisPriceSourceChecked::StakedGeometricTwap {
        transitive_denom: "transitive".to_string(),
        pool_id: 123,
        window_size: 300,
        downtime_detector: None,
    };
    assert_eq!(ps.to_string(), "staked_geometric_twap:transitive:123:300:None");

    let ps = OsmosisPriceSourceChecked::StakedGeometricTwap {
        transitive_denom: "transitive".to_string(),
        pool_id: 123,
        window_size: 300,
        downtime_detector: Some(DowntimeDetector {
            downtime: Downtime::Duration30m,
            recovery: 568,
        }),
    };
    assert_eq!(ps.to_string(), "staked_geometric_twap:transitive:123:300:Some(Duration30m:568)");
}

#[test]
fn display_xyk_lp_price_source() {
    let ps = OsmosisPriceSourceChecked::XykLiquidityToken {
        pool_id: 224,
    };
    assert_eq!(ps.to_string(), "xyk_liquidity_token:224")
}

#[test]
fn display_pyth_price_source() {
    let ps = OsmosisPriceSourceChecked::Pyth {
        contract_addr: Addr::unchecked("osmo12j43nf2f0qumnt2zrrmpvnsqgzndxefujlvr08"),
        price_feed_id: PriceIdentifier::from_hex(
            "61226d39beea19d334f17c2febce27e12646d84675924ebb02b9cdaea68727e3",
        )
        .unwrap(),
        max_staleness: 60,
        max_confidence: Decimal::percent(10u64),
        max_deviation: Decimal::percent(15u64),
        denom_decimals: 18,
    };
    assert_eq!(
            ps.to_string(),
            "pyth:osmo12j43nf2f0qumnt2zrrmpvnsqgzndxefujlvr08:0x61226d39beea19d334f17c2febce27e12646d84675924ebb02b9cdaea68727e3:60:0.1:0.15:18"
        )
}

#[test]
fn display_lsd_price_source() {
    let ps = OsmosisPriceSourceChecked::Lsd {
        transitive_denom: "transitive".to_string(),
        twap: Twap {
            pool_id: 456,
            window_size: 380,
            downtime_detector: None,
            kind: TwapKind::ArithmeticTwap {},
        },
        redemption_rate: RedemptionRate {
            contract_addr: Addr::unchecked(
                "osmo1zw4fxj4pt0pu0jdd7cs6gecdj3pvfxhhtgkm4w2y44jp60hywzvssud6uc",
            ),
            max_staleness: 1234,
        },
    };
    assert_eq!(ps.to_string(), "lsd:transitive:456:380:None:arithmetic_twap:osmo1zw4fxj4pt0pu0jdd7cs6gecdj3pvfxhhtgkm4w2y44jp60hywzvssud6uc:1234");

    let ps = OsmosisPriceSourceChecked::Lsd {
        transitive_denom: "transitive".to_string(),
        twap: Twap {
            pool_id: 456,
            window_size: 380,
            downtime_detector: None,
            kind: TwapKind::GeometricTwap {},
        },
        redemption_rate: RedemptionRate {
            contract_addr: Addr::unchecked(
                "osmo1zw4fxj4pt0pu0jdd7cs6gecdj3pvfxhhtgkm4w2y44jp60hywzvssud6uc",
            ),
            max_staleness: 1234,
        },
    };
    assert_eq!(ps.to_string(), "lsd:transitive:456:380:None:geometric_twap:osmo1zw4fxj4pt0pu0jdd7cs6gecdj3pvfxhhtgkm4w2y44jp60hywzvssud6uc:1234");

    let ps = OsmosisPriceSourceChecked::Lsd {
        transitive_denom: "transitive".to_string(),
        twap: Twap {
            pool_id: 456,
            window_size: 380,
            downtime_detector: Some(DowntimeDetector {
                downtime: Downtime::Duration30m,
                recovery: 552,
            }),
            kind: TwapKind::GeometricTwap {},
        },
        redemption_rate: RedemptionRate {
            contract_addr: Addr::unchecked(
                "osmo1zw4fxj4pt0pu0jdd7cs6gecdj3pvfxhhtgkm4w2y44jp60hywzvssud6uc",
            ),
            max_staleness: 1234,
        },
    };
    assert_eq!(ps.to_string(), "lsd:transitive:456:380:Some(Duration30m:552):geometric_twap:osmo1zw4fxj4pt0pu0jdd7cs6gecdj3pvfxhhtgkm4w2y44jp60hywzvssud6uc:1234");
}
