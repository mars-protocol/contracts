use cosmwasm_std::testing::mock_env;
use cosmwasm_std::Decimal;

use mars_oracle_base::ContractError;
use mars_outpost::error::MarsError;
use mars_outpost::oracle::QueryMsg;
use mars_testing::mock_info;

use mars_oracle_osmosis::contract::entry::execute;
use mars_oracle_osmosis::msg::{ExecuteMsg, PriceSourceResponse};
use mars_oracle_osmosis::OsmosisPriceSource;

mod helpers;

#[test]
fn test_setting_price_source_by_non_owner() {
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
fn test_setting_price_source_fixed() {
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
fn test_setting_price_source_spot() {
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

    // attempting to use not XYK pool
    let err = set_price_source_spot("uion", 4444).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "assets in pool 4444 do not have equal weights".to_string()
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
fn test_setting_price_source_twap() {
    let mut deps = helpers::setup_test();

    let mut set_price_source_twap = |denom: &str, pool_id: u64, window_size| {
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("owner"),
            ExecuteMsg::SetPriceSource {
                denom: denom.to_string(),
                price_source: OsmosisPriceSource::Twap {
                    pool_id,
                    window_size,
                },
            },
        )
    };

    // attempting to use a pool that does not contain the denom of interest; should fail
    let err = set_price_source_twap("umars", 1, 86400).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "pool 1 does not contain umars".to_string()
        }
    );

    // attempting to use a pool that does not contain the base denom, uosmo; should fail
    let err = set_price_source_twap("uatom", 64, 86400).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "pool 64 does not contain the base denom uosmo".to_string()
        }
    );

    // attempting to use a pool that contains more than two assets; should fail
    let err = set_price_source_twap("uusdc", 3333, 86400).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "expecting pool 3333 to contain exactly two coins; found 3".to_string()
        }
    );

    // attempting to use not XYK pool
    let err = set_price_source_twap("uion", 4444, 86400).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "assets in pool 4444 do not have equal weights".to_string()
        }
    );

    // attempting to set window_size bigger than 172800 sec (48h)
    let err = set_price_source_twap("umars", 89, 172801).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "expecting window size to be within 172800 sec".to_string()
        }
    );

    // properly set spot price source
    let res = set_price_source_twap("umars", 89, 86400).unwrap();
    assert_eq!(res.messages.len(), 0);

    let res: PriceSourceResponse = helpers::query(
        deps.as_ref(),
        QueryMsg::PriceSource {
            denom: "umars".to_string(),
        },
    );
    assert_eq!(
        res.price_source,
        OsmosisPriceSource::Twap {
            pool_id: 89,
            window_size: 86400
        }
    );
}

#[test]
fn test_setting_price_source_liquidity_token() {
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
fn test_querying_price_source() {
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
