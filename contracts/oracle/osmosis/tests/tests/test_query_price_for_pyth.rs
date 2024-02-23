use cosmwasm_std::{from_json, Decimal};
use mars_oracle_base::ContractError;
use mars_oracle_osmosis::{contract::entry, OsmosisPriceSourceUnchecked};
use mars_testing::mock_env_at_block_time;
use mars_types::oracle::{ActionKind, PriceResponse, QueryMsg};
use pyth_sdk_cw::{Price, PriceFeed, PriceFeedResponse, PriceIdentifier};

use super::helpers;

#[test]
fn querying_default_pyth_price_if_publish_price_too_old() {
    let mut deps = helpers::setup_test_for_pyth();

    let price_id = PriceIdentifier::from_hex(
        "61226d39beea19d334f17c2febce27e12646d84675924ebb02b9cdaea68727e3",
    )
    .unwrap();

    let max_staleness = 30u64;
    helpers::set_price_source(
        deps.as_mut(),
        "uatom",
        OsmosisPriceSourceUnchecked::Pyth {
            contract_addr: "pyth_contract_addr".to_string(),
            price_feed_id: price_id,
            max_staleness,
            max_confidence: Decimal::percent(10u64),
            max_deviation: Decimal::percent(15u64),
            denom_decimals: 6u8,
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
            kind: Some(ActionKind::Default),
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

    let ema_price_publish_time = 1677157333u64;
    let price_publish_time = ema_price_publish_time + max_staleness;
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
        mock_env_at_block_time(ema_price_publish_time + max_staleness + 1u64),
        QueryMsg::Price {
            denom: "uatom".to_string(),
            kind: Some(ActionKind::Default),
        },
    )
    .unwrap_err();
    assert_eq!(
        res_err,
        ContractError::InvalidPrice {
            reason:
                "EMA price publish time is too old/stale. published: 1677157333, now: 1677157364"
                    .to_string()
        }
    );
}

#[test]
fn querying_liquidation_pyth_price_if_publish_price_too_old() {
    let mut deps = helpers::setup_test_for_pyth();

    let price_id = PriceIdentifier::from_hex(
        "61226d39beea19d334f17c2febce27e12646d84675924ebb02b9cdaea68727e3",
    )
    .unwrap();

    let max_staleness = 30u64;
    helpers::set_price_source(
        deps.as_mut(),
        "uatom",
        OsmosisPriceSourceUnchecked::Pyth {
            contract_addr: "pyth_contract_addr".to_string(),
            price_feed_id: price_id,
            max_staleness,
            max_confidence: Decimal::percent(10u64),
            max_deviation: Decimal::percent(15u64),
            denom_decimals: 6u8,
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
            kind: Some(ActionKind::Liquidation),
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
fn querying_default_pyth_price_if_signed() {
    let mut deps = helpers::setup_test_for_pyth();

    let price_id = PriceIdentifier::from_hex(
        "61226d39beea19d334f17c2febce27e12646d84675924ebb02b9cdaea68727e3",
    )
    .unwrap();

    let max_staleness = 30u64;
    helpers::set_price_source(
        deps.as_mut(),
        "uatom",
        OsmosisPriceSourceUnchecked::Pyth {
            contract_addr: "pyth_contract_addr".to_string(),
            price_feed_id: price_id,
            max_staleness,
            max_confidence: Decimal::percent(10u64),
            max_deviation: Decimal::percent(15u64),
            denom_decimals: 6u8,
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
            kind: Some(ActionKind::Default),
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
fn querying_liquidation_pyth_price_if_signed() {
    let mut deps = helpers::setup_test_for_pyth();

    let price_id = PriceIdentifier::from_hex(
        "61226d39beea19d334f17c2febce27e12646d84675924ebb02b9cdaea68727e3",
    )
    .unwrap();

    let max_staleness = 30u64;
    helpers::set_price_source(
        deps.as_mut(),
        "uatom",
        OsmosisPriceSourceUnchecked::Pyth {
            contract_addr: "pyth_contract_addr".to_string(),
            price_feed_id: price_id,
            max_staleness,
            max_confidence: Decimal::percent(10u64),
            max_deviation: Decimal::percent(15u64),
            denom_decimals: 6u8,
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
            kind: Some(ActionKind::Liquidation),
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
fn querying_pyth_price_if_confidence_exceeded() {
    let mut deps = helpers::setup_test_for_pyth();

    let price_id = PriceIdentifier::from_hex(
        "61226d39beea19d334f17c2febce27e12646d84675924ebb02b9cdaea68727e3",
    )
    .unwrap();

    let max_staleness = 30u64;
    helpers::set_price_source(
        deps.as_mut(),
        "uatom",
        OsmosisPriceSourceUnchecked::Pyth {
            contract_addr: "pyth_contract_addr".to_string(),
            price_feed_id: price_id,
            max_staleness,
            max_confidence: Decimal::percent(5u64),
            max_deviation: Decimal::percent(6u64),
            denom_decimals: 6u8,
        },
    );

    let publish_time = 1677157333u64;
    deps.querier.set_pyth_price(
        price_id,
        PriceFeedResponse {
            price_feed: PriceFeed::new(
                price_id,
                Price {
                    price: 1010000,
                    conf: 51000,
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

    // should fail for Default pricing
    let res_err = entry::query(
        deps.as_ref(),
        mock_env_at_block_time(publish_time),
        QueryMsg::Price {
            denom: "uatom".to_string(),
            kind: Some(ActionKind::Default),
        },
    )
    .unwrap_err();
    assert_eq!(
        res_err,
        ContractError::InvalidPrice {
            reason: "price confidence deviation 0.051 exceeds max allowed 0.05".to_string()
        }
    );

    // should succeed for Liquidation pricing
    let res = entry::query(
        deps.as_ref(),
        mock_env_at_block_time(publish_time),
        QueryMsg::Price {
            denom: "uatom".to_string(),
            kind: Some(ActionKind::Liquidation),
        },
    )
    .unwrap();
    let res: PriceResponse = from_json(res).unwrap();
    assert_eq!(res.price, Decimal::from_ratio(101u128, 1u128));
}

#[test]
fn querying_pyth_price_if_deviation_exceeded() {
    let mut deps = helpers::setup_test_for_pyth();

    let price_id = PriceIdentifier::from_hex(
        "61226d39beea19d334f17c2febce27e12646d84675924ebb02b9cdaea68727e3",
    )
    .unwrap();

    let max_staleness = 30u64;
    helpers::set_price_source(
        deps.as_mut(),
        "uatom",
        OsmosisPriceSourceUnchecked::Pyth {
            contract_addr: "pyth_contract_addr".to_string(),
            price_feed_id: price_id,
            max_staleness,
            max_confidence: Decimal::percent(5u64),
            max_deviation: Decimal::percent(6u64),
            denom_decimals: 6u8,
        },
    );

    let publish_time = 1677157333u64;

    // price > ema_price
    deps.querier.set_pyth_price(
        price_id,
        PriceFeedResponse {
            price_feed: PriceFeed::new(
                price_id,
                Price {
                    price: 1061000,
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

    // should fail for Default pricing
    let res_err = entry::query(
        deps.as_ref(),
        mock_env_at_block_time(publish_time),
        QueryMsg::Price {
            denom: "uatom".to_string(),
            kind: Some(ActionKind::Default),
        },
    )
    .unwrap_err();
    assert_eq!(
        res_err,
        ContractError::InvalidPrice {
            reason: "price deviation 0.061 exceeds max allowed 0.06".to_string()
        }
    );

    // should succeed for Liquidation pricing
    let res = entry::query(
        deps.as_ref(),
        mock_env_at_block_time(publish_time),
        QueryMsg::Price {
            denom: "uatom".to_string(),
            kind: Some(ActionKind::Liquidation),
        },
    )
    .unwrap();
    let res: PriceResponse = from_json(res).unwrap();
    assert_eq!(res.price, Decimal::from_ratio(1061u128, 10u128));

    // ema_price > price
    deps.querier.set_pyth_price(
        price_id,
        PriceFeedResponse {
            price_feed: PriceFeed::new(
                price_id,
                Price {
                    price: 939999,
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

    // should fail for Default pricing
    let res_err = entry::query(
        deps.as_ref(),
        mock_env_at_block_time(publish_time),
        QueryMsg::Price {
            denom: "uatom".to_string(),
            kind: Some(ActionKind::Default),
        },
    )
    .unwrap_err();
    assert_eq!(
        res_err,
        ContractError::InvalidPrice {
            reason: "price deviation 0.060001 exceeds max allowed 0.06".to_string()
        }
    );

    // should succeed for Liquidation pricing
    let res = entry::query(
        deps.as_ref(),
        mock_env_at_block_time(publish_time),
        QueryMsg::Price {
            denom: "uatom".to_string(),
            kind: Some(ActionKind::Liquidation),
        },
    )
    .unwrap();
    let res: PriceResponse = from_json(res).unwrap();
    assert_eq!(res.price, Decimal::from_ratio(939999u128, 10000u128));
}

#[test]
fn querying_pyth_price_successfully() {
    let mut deps = helpers::setup_test_for_pyth();

    let price_id = PriceIdentifier::from_hex(
        "61226d39beea19d334f17c2febce27e12646d84675924ebb02b9cdaea68727e3",
    )
    .unwrap();

    let max_staleness = 30u64;
    helpers::set_price_source(
        deps.as_mut(),
        "uatom",
        OsmosisPriceSourceUnchecked::Pyth {
            contract_addr: "pyth_contract_addr".to_string(),
            price_feed_id: price_id,
            max_staleness,
            max_confidence: Decimal::percent(10u64),
            max_deviation: Decimal::percent(15u64),
            denom_decimals: 6u8,
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

    let default_res = entry::query(
        deps.as_ref(),
        mock_env_at_block_time(publish_time),
        QueryMsg::Price {
            denom: "uatom".to_string(),
            kind: Some(ActionKind::Default),
        },
    )
    .unwrap();
    let default_res: PriceResponse = from_json(default_res).unwrap();
    assert_eq!(default_res.price, Decimal::from_ratio(102000u128, 1u128));

    let liq_res = entry::query(
        deps.as_ref(),
        mock_env_at_block_time(publish_time),
        QueryMsg::Price {
            denom: "uatom".to_string(),
            kind: Some(ActionKind::Liquidation),
        },
    )
    .unwrap();
    let liq_res: PriceResponse = from_json(liq_res).unwrap();
    // Price for default and liquidation actions should be the same
    assert_eq!(liq_res.price, default_res.price);
}
