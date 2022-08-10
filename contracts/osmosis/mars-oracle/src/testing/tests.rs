use cosmwasm_std::testing::mock_env;
use cosmwasm_std::Decimal;

use mars_oracle_base::ContractError;
use mars_outpost::error::MarsError;
use mars_outpost::oracle::{Config, PriceResponse, QueryMsg};
use mars_testing::mock_info;

use osmo_bindings::{SpotPriceResponse, Swap};

use super::helpers;
use crate::contract::entry::execute;
use crate::msg::{ExecuteMsg, PriceSourceResponse};
use crate::OsmosisPriceSource;

#[test]
fn instantiating() {
    let deps = helpers::setup_test();

    let cfg: Config<String> = helpers::query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(cfg.owner, "owner".to_string());
    assert_eq!(cfg.base_denom, "uosmo".to_string());
}

#[test]
fn updating_config() {
    let mut deps = helpers::setup_test();

    let msg = ExecuteMsg::UpdateConfig {
        owner: Some("new_owner".to_string()),
    };

    // non-owner cannot update
    let err = execute(deps.as_mut(), mock_env(), mock_info("jake"), msg.clone()).unwrap_err();
    assert_eq!(err, MarsError::Unauthorized {}.into());

    // owner can update
    let res = execute(deps.as_mut(), mock_env(), mock_info("owner"), msg).unwrap();
    assert_eq!(res.messages.len(), 0);

    let cfg: Config<String> = helpers::query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(cfg.owner, "new_owner".to_string());
}

#[test]
fn setting_price_source_by_non_owner() {
    let mut deps = helpers::setup_test();

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake"),
        ExecuteMsg::SetPriceSource {
            denom: "uosmo".to_string(),
            price_source: OsmosisPriceSource::Fixed {
                price: Decimal::one(),
            },
        },
    )
    .unwrap_err();
    assert_eq!(err, MarsError::Unauthorized {}.into())
}

#[test]
fn setting_price_source_fixed() {
    let mut deps = helpers::setup_test();

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "uosmo".to_string(),
            price_source: OsmosisPriceSource::Fixed {
                price: Decimal::one(),
            },
        },
    )
    .unwrap();
    assert_eq!(res.messages.len(), 0);

    let res: PriceSourceResponse = helpers::query(
        deps.as_ref(),
        QueryMsg::PriceSource {
            denom: "uosmo".to_string(),
        },
    );
    assert_eq!(
        res.price_source,
        OsmosisPriceSource::Fixed {
            price: Decimal::one()
        }
    );
}

#[test]
fn setting_price_source_spot() {
    let mut deps = helpers::setup_test();

    let mut set_price_source_spot = |denom: &str, pool_id: u64| {
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("owner"),
            ExecuteMsg::SetPriceSource {
                denom: denom.to_string(),
                price_source: OsmosisPriceSource::Spot {
                    pool_id,
                },
            },
        )
    };

    // attempting to use a pool that does not contain the denom of interest; should fail
    let err = set_price_source_spot("umars", 1).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "pool 1 does not contain umars".to_string()
        }
    );

    // attempting to use a pool that does not contain the base denom, uosmo; should fail
    let err = set_price_source_spot("uatom", 64).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "pool 64 does not contain the base denom uosmo".to_string()
        }
    );

    // attempting to use a pool that contains more than two assets; should fail
    let err = set_price_source_spot("uusdc", 3333).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "expecting pool 3333 to contain exactly two coins; found 3".to_string()
        }
    );

    // properly set spot price source
    let res = set_price_source_spot("umars", 89).unwrap();
    assert_eq!(res.messages.len(), 0);

    let res: PriceSourceResponse = helpers::query(
        deps.as_ref(),
        QueryMsg::PriceSource {
            denom: "umars".to_string(),
        },
    );
    assert_eq!(
        res.price_source,
        OsmosisPriceSource::Spot {
            pool_id: 89,
        }
    );
}

#[test]
fn setting_price_source_liquidity_token() {
    let mut deps = helpers::setup_test();

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "gamm/pool/89".to_string(),
            price_source: OsmosisPriceSource::LiquidityToken {
                pool_id: 89,
            },
        },
    )
    .unwrap();
    assert_eq!(res.messages.len(), 0);

    let res: PriceSourceResponse = helpers::query(
        deps.as_ref(),
        QueryMsg::PriceSource {
            denom: "gamm/pool/89".to_string(),
        },
    );
    assert_eq!(
        res.price_source,
        OsmosisPriceSource::LiquidityToken {
            pool_id: 89,
        }
    );
}

#[test]
fn querying_price_source() {
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
    helpers::set_price_source(
        deps.as_mut(),
        "gamm/pool/89",
        OsmosisPriceSource::LiquidityToken {
            pool_id: 89,
        },
    );

    // try query a single price source
    let res: PriceSourceResponse = helpers::query(
        deps.as_ref(),
        QueryMsg::PriceSource {
            denom: "umars".to_string(),
        },
    );
    assert_eq!(
        res.price_source,
        OsmosisPriceSource::Spot {
            pool_id: 89,
        }
    );

    // try query all price sources
    //
    // NOTE: responses are ordered alphabetically by denoms
    let res: Vec<PriceSourceResponse> = helpers::query(
        deps.as_ref(),
        QueryMsg::PriceSources {
            start_after: None,
            limit: Some(2),
        },
    );
    assert_eq!(
        res,
        vec![
            PriceSourceResponse {
                denom: "gamm/pool/89".to_string(),
                price_source: OsmosisPriceSource::LiquidityToken {
                    pool_id: 89
                }
            },
            PriceSourceResponse {
                denom: "uatom".to_string(),
                price_source: OsmosisPriceSource::Spot {
                    pool_id: 1
                }
            }
        ]
    );

    let res: Vec<PriceSourceResponse> = helpers::query(
        deps.as_ref(),
        QueryMsg::PriceSources {
            start_after: Some("uatom".to_string()),
            limit: None,
        },
    );
    assert_eq!(
        res,
        vec![
            PriceSourceResponse {
                denom: "umars".to_string(),
                price_source: OsmosisPriceSource::Spot {
                    pool_id: 89
                }
            },
            PriceSourceResponse {
                denom: "uosmo".to_string(),
                price_source: OsmosisPriceSource::Fixed {
                    price: Decimal::one()
                }
            }
        ]
    );
}

#[test]
fn querying_price_fixed() {
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
fn querying_price_spot() {
    let mut deps = helpers::setup_test();

    helpers::set_price_source(
        deps.as_mut(),
        "umars",
        OsmosisPriceSource::Spot {
            pool_id: 89,
        },
    );

    deps.querier.set_spot_price(
        Swap {
            pool_id: 89,
            denom_in: "umars".to_string(),
            denom_out: "uosmo".to_string(),
        },
        SpotPriceResponse {
            price: Decimal::from_ratio(88888u128, 12345u128),
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
        Swap {
            pool_id: 1,
            denom_in: "uatom".to_string(),
            denom_out: "uosmo".to_string(),
        },
        SpotPriceResponse {
            price: Decimal::from_ratio(77777u128, 12345u128),
        },
    );
    deps.querier.set_spot_price(
        Swap {
            pool_id: 89,
            denom_in: "umars".to_string(),
            denom_out: "uosmo".to_string(),
        },
        SpotPriceResponse {
            price: Decimal::from_ratio(88888u128, 12345u128),
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
