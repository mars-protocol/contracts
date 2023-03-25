use cosmwasm_std::{coin, Decimal, StdError};
use mars_oracle::msg::{PriceResponse, QueryMsg};
use mars_oracle_base::ContractError;
use mars_oracle_osmosis::{Downtime, DowntimeDetector, OsmosisPriceSource};
use osmosis_std::types::osmosis::{
    gamm::v2::QuerySpotPriceResponse,
    twap::v1beta1::{ArithmeticTwapToNowResponse, GeometricTwapToNowResponse},
};

use crate::helpers::prepare_query_pool_response;

mod helpers;

#[test]
fn querying_fixed_price() {
    let mut deps = helpers::setup_test();

    helpers::set_price_source(
        deps.as_mut(),
        "uosmo",
        OsmosisPriceSource::Fixed {
            price: Decimal::one(),
        },
    );

    let res: PriceResponse = helpers::query(
        deps.as_ref(),
        QueryMsg::Price {
            denom: "uosmo".to_string(),
        },
    );
    assert_eq!(res.price, Decimal::one());
}

#[test]
fn querying_spot_price() {
    let mut deps = helpers::setup_test();

    helpers::set_price_source(
        deps.as_mut(),
        "umars",
        OsmosisPriceSource::Spot {
            pool_id: 89,
        },
    );

    deps.querier.set_spot_price(
        89,
        "umars",
        "uosmo",
        QuerySpotPriceResponse {
            spot_price: Decimal::from_ratio(88888u128, 12345u128).to_string(),
        },
    );

    let res: PriceResponse = helpers::query(
        deps.as_ref(),
        QueryMsg::Price {
            denom: "umars".to_string(),
        },
    );
    assert_eq!(res.price, Decimal::from_ratio(88888u128, 12345u128));
}

#[test]
fn querying_arithmetic_twap_price() {
    let mut deps = helpers::setup_test();

    helpers::set_price_source(
        deps.as_mut(),
        "umars",
        OsmosisPriceSource::ArithmeticTwap {
            pool_id: 89,
            window_size: 86400,
            downtime_detector: None,
        },
    );

    deps.querier.set_arithmetic_twap_price(
        89,
        "umars",
        "uosmo",
        ArithmeticTwapToNowResponse {
            arithmetic_twap: Decimal::from_ratio(77777u128, 12345u128).to_string(),
        },
    );

    let res: PriceResponse = helpers::query(
        deps.as_ref(),
        QueryMsg::Price {
            denom: "umars".to_string(),
        },
    );
    assert_eq!(res.price, Decimal::from_ratio(77777u128, 12345u128));
}

#[test]
fn querying_arithmetic_twap_price_with_downtime_detector() {
    let mut deps = helpers::setup_test();

    let dd = DowntimeDetector {
        downtime: Downtime::Duration10m,
        recovery: 360,
    };
    helpers::set_price_source(
        deps.as_mut(),
        "umars",
        OsmosisPriceSource::ArithmeticTwap {
            pool_id: 89,
            window_size: 86400,
            downtime_detector: Some(dd.clone()),
        },
    );

    deps.querier.set_downtime_detector(dd.clone(), false);
    let res_err = helpers::query_err(
        deps.as_ref(),
        QueryMsg::Price {
            denom: "umars".to_string(),
        },
    );
    assert_eq!(
        res_err,
        ContractError::InvalidPrice {
            reason: "chain is recovering from downtime".to_string()
        }
    );

    deps.querier.set_downtime_detector(dd, true);
    deps.querier.set_arithmetic_twap_price(
        89,
        "umars",
        "uosmo",
        ArithmeticTwapToNowResponse {
            arithmetic_twap: Decimal::from_ratio(77777u128, 12345u128).to_string(),
        },
    );
    let res: PriceResponse = helpers::query(
        deps.as_ref(),
        QueryMsg::Price {
            denom: "umars".to_string(),
        },
    );
    assert_eq!(res.price, Decimal::from_ratio(77777u128, 12345u128));
}

#[test]
fn querying_geometric_twap_price() {
    let mut deps = helpers::setup_test();

    helpers::set_price_source(
        deps.as_mut(),
        "umars",
        OsmosisPriceSource::GeometricTwap {
            pool_id: 89,
            window_size: 86400,
            downtime_detector: None,
        },
    );

    deps.querier.set_geometric_twap_price(
        89,
        "umars",
        "uosmo",
        GeometricTwapToNowResponse {
            geometric_twap: Decimal::from_ratio(66666u128, 12345u128).to_string(),
        },
    );

    let res: PriceResponse = helpers::query(
        deps.as_ref(),
        QueryMsg::Price {
            denom: "umars".to_string(),
        },
    );
    assert_eq!(res.price, Decimal::from_ratio(66666u128, 12345u128));
}

#[test]
fn querying_geometric_twap_price_with_downtime_detector() {
    let mut deps = helpers::setup_test();

    let dd = DowntimeDetector {
        downtime: Downtime::Duration10m,
        recovery: 360,
    };
    helpers::set_price_source(
        deps.as_mut(),
        "umars",
        OsmosisPriceSource::GeometricTwap {
            pool_id: 89,
            window_size: 86400,
            downtime_detector: Some(dd.clone()),
        },
    );

    deps.querier.set_downtime_detector(dd.clone(), false);
    let res_err = helpers::query_err(
        deps.as_ref(),
        QueryMsg::Price {
            denom: "umars".to_string(),
        },
    );
    assert_eq!(
        res_err,
        ContractError::InvalidPrice {
            reason: "chain is recovering from downtime".to_string()
        }
    );

    deps.querier.set_downtime_detector(dd, true);
    deps.querier.set_geometric_twap_price(
        89,
        "umars",
        "uosmo",
        GeometricTwapToNowResponse {
            geometric_twap: Decimal::from_ratio(77777u128, 12345u128).to_string(),
        },
    );
    let res: PriceResponse = helpers::query(
        deps.as_ref(),
        QueryMsg::Price {
            denom: "umars".to_string(),
        },
    );
    assert_eq!(res.price, Decimal::from_ratio(77777u128, 12345u128));
}

#[test]
fn querying_staked_geometric_twap_price() {
    let mut deps = helpers::setup_test();

    helpers::set_price_source(
        deps.as_mut(),
        "uatom",
        OsmosisPriceSource::GeometricTwap {
            pool_id: 1,
            window_size: 86400,
            downtime_detector: None,
        },
    );
    helpers::set_price_source(
        deps.as_mut(),
        "ustatom",
        OsmosisPriceSource::StakedGeometricTwap {
            transitive_denom: "uatom".to_string(),
            pool_id: 803,
            window_size: 86400,
            downtime_detector: None,
        },
    );

    let uatom_uosmo_price = Decimal::from_ratio(135u128, 10u128);
    deps.querier.set_geometric_twap_price(
        1,
        "uatom",
        "uosmo",
        GeometricTwapToNowResponse {
            geometric_twap: uatom_uosmo_price.to_string(),
        },
    );
    let ustatom_uatom_price = Decimal::from_ratio(105u128, 100u128);
    deps.querier.set_geometric_twap_price(
        803,
        "ustatom",
        "uatom",
        GeometricTwapToNowResponse {
            geometric_twap: ustatom_uatom_price.to_string(),
        },
    );

    let res: PriceResponse = helpers::query(
        deps.as_ref(),
        QueryMsg::Price {
            denom: "ustatom".to_string(),
        },
    );
    let expected_price = ustatom_uatom_price * uatom_uosmo_price;
    assert_eq!(res.price, expected_price);
}

#[test]
fn querying_staked_geometric_twap_price_if_no_transitive_denom_price_source() {
    let mut deps = helpers::setup_test();

    helpers::set_price_source(
        deps.as_mut(),
        "ustatom",
        OsmosisPriceSource::StakedGeometricTwap {
            transitive_denom: "uatom".to_string(),
            pool_id: 803,
            window_size: 86400,
            downtime_detector: None,
        },
    );

    let ustatom_uatom_price = Decimal::from_ratio(105u128, 100u128);
    deps.querier.set_geometric_twap_price(
        803,
        "ustatom",
        "uatom",
        GeometricTwapToNowResponse {
            geometric_twap: ustatom_uatom_price.to_string(),
        },
    );

    let res_err = helpers::query_err(
        deps.as_ref(),
        QueryMsg::Price {
            denom: "ustatom".to_string(),
        },
    );
    assert_eq!(
        res_err,
        ContractError::Std(StdError::not_found(
            "mars_oracle_osmosis::price_source::OsmosisPriceSource"
        ))
    );
}

#[test]
fn querying_staked_geometric_twap_price_with_downtime_detector() {
    let mut deps = helpers::setup_test();

    let dd = DowntimeDetector {
        downtime: Downtime::Duration10m,
        recovery: 360,
    };
    helpers::set_price_source(
        deps.as_mut(),
        "uatom",
        OsmosisPriceSource::GeometricTwap {
            pool_id: 1,
            window_size: 86400,
            downtime_detector: Some(dd.clone()),
        },
    );
    helpers::set_price_source(
        deps.as_mut(),
        "ustatom",
        OsmosisPriceSource::StakedGeometricTwap {
            transitive_denom: "uatom".to_string(),
            pool_id: 803,
            window_size: 86400,
            downtime_detector: Some(dd.clone()),
        },
    );

    deps.querier.set_downtime_detector(dd.clone(), false);
    let res_err = helpers::query_err(
        deps.as_ref(),
        QueryMsg::Price {
            denom: "ustatom".to_string(),
        },
    );
    assert_eq!(
        res_err,
        ContractError::InvalidPrice {
            reason: "chain is recovering from downtime".to_string()
        }
    );

    deps.querier.set_downtime_detector(dd, true);

    let uatom_uosmo_price = Decimal::from_ratio(135u128, 10u128);
    deps.querier.set_geometric_twap_price(
        1,
        "uatom",
        "uosmo",
        GeometricTwapToNowResponse {
            geometric_twap: uatom_uosmo_price.to_string(),
        },
    );
    let ustatom_uatom_price = Decimal::from_ratio(105u128, 100u128);
    deps.querier.set_geometric_twap_price(
        803,
        "ustatom",
        "uatom",
        GeometricTwapToNowResponse {
            geometric_twap: ustatom_uatom_price.to_string(),
        },
    );

    let res: PriceResponse = helpers::query(
        deps.as_ref(),
        QueryMsg::Price {
            denom: "ustatom".to_string(),
        },
    );
    let expected_price = ustatom_uatom_price * uatom_uosmo_price;
    assert_eq!(res.price, expected_price);
}

#[test]
fn querying_xyk_lp_price() {
    let mut deps = helpers::setup_test();

    let assets = vec![coin(1, "uatom"), coin(1, "uosmo")];
    deps.querier.set_query_pool_response(
        10001,
        prepare_query_pool_response(
            10001,
            &assets,
            &[5000u64, 5000u64],
            &coin(1, "gamm/pool/10001"),
        ),
    );

    let assets = vec![coin(1, "umars"), coin(1, "uosmo")];
    deps.querier.set_query_pool_response(
        10002,
        prepare_query_pool_response(
            10002,
            &assets,
            &[5000u64, 5000u64],
            &coin(1, "gamm/pool/10002"),
        ),
    );

    let assets = vec![coin(10000, "uatom"), coin(885000, "umars")];
    deps.querier.set_query_pool_response(
        10003,
        prepare_query_pool_response(
            10003,
            &assets,
            &[5000u64, 5000u64],
            &coin(10000, "gamm/pool/10003"),
        ),
    );

    // set price source for uatom
    let uatom_price = Decimal::from_ratio(885_u128, 10_u128);
    helpers::set_price_source(
        deps.as_mut(),
        "uatom",
        OsmosisPriceSource::Fixed {
            price: uatom_price,
        },
    );
    deps.querier.set_oracle_price("uatom", uatom_price);

    // set price source for umars
    let umars_price = Decimal::one();
    helpers::set_price_source(
        deps.as_mut(),
        "umars",
        OsmosisPriceSource::Fixed {
            price: umars_price,
        },
    );
    deps.querier.set_oracle_price("umars", umars_price);

    // set price source for xyk lp token
    helpers::set_price_source(
        deps.as_mut(),
        "uatom_umars_lp",
        OsmosisPriceSource::XykLiquidityToken {
            pool_id: 10003,
        },
    );

    // Atom price: 88.5
    // Atom depth: 10000
    // Mars price: 1
    // Mars depth: 885000
    // pool value: 2 * sqrt((88.5 * 10000) * (1 * 885000)) = 1770000
    // LP token price: 1770000 / 10000 = 177
    let res: PriceResponse = helpers::query(
        deps.as_ref(),
        QueryMsg::Price {
            denom: "uatom_umars_lp".to_string(),
        },
    );
    assert_eq!(res.price, Decimal::from_ratio(1770000_u128, 10000_u128));

    // Now assume someone buys a large amount of Atom, skewing the pool depths. Let's see if
    // the oracle price of the LP token is affected.
    //
    // Assume the attacker sells 500000 mars for atom
    // Mars depth = 885000 + 500000 = 1385000
    // Atom depth = 10000 * 885000 / 1385000 = 6389
    let assets = vec![coin(6389, "uatom"), coin(1385000, "umars")];
    deps.querier.set_query_pool_response(
        10003,
        prepare_query_pool_response(
            10003,
            &assets,
            &[5000u64, 5000u64],
            &coin(10000, "gamm/pool/10003"),
        ),
    );

    let res: PriceResponse = helpers::query(
        deps.as_ref(),
        QueryMsg::Price {
            denom: "uatom_umars_lp".to_string(),
        },
    );
    // Atom price: 88.5
    // Mars price: 1
    // pool value = 2 * sqrt((88.5 * 6389) * (1 * 1385000)) = 1769874
    //
    // Is slightly (<0.01%) off from the pre-manipulation value.
    assert_eq!(res.price, Decimal::from_ratio(1769874_u128, 10000_u128));
}

#[test]
fn querying_all_prices() {
    let mut deps = helpers::setup_test();

    helpers::set_price_source(
        deps.as_mut(),
        "uosmo",
        OsmosisPriceSource::Fixed {
            price: Decimal::one(),
        },
    );
    helpers::set_price_source(
        deps.as_mut(),
        "uatom",
        OsmosisPriceSource::Spot {
            pool_id: 1,
        },
    );
    helpers::set_price_source(
        deps.as_mut(),
        "umars",
        OsmosisPriceSource::Spot {
            pool_id: 89,
        },
    );

    deps.querier.set_spot_price(
        1,
        "uatom",
        "uosmo",
        QuerySpotPriceResponse {
            spot_price: Decimal::from_ratio(77777u128, 12345u128).to_string(),
        },
    );
    deps.querier.set_spot_price(
        89,
        "umars",
        "uosmo",
        QuerySpotPriceResponse {
            spot_price: Decimal::from_ratio(88888u128, 12345u128).to_string(),
        },
    );

    // NOTE: responses are ordered alphabetically by denom
    let res: Vec<PriceResponse> = helpers::query(
        deps.as_ref(),
        QueryMsg::Prices {
            start_after: None,
            limit: None,
        },
    );
    assert_eq!(
        res,
        vec![
            PriceResponse {
                denom: "uatom".to_string(),
                price: Decimal::from_ratio(77777u128, 12345u128),
            },
            PriceResponse {
                denom: "umars".to_string(),
                price: Decimal::from_ratio(88888u128, 12345u128),
            },
            PriceResponse {
                denom: "uosmo".to_string(),
                price: Decimal::one(),
            },
        ]
    );
}
