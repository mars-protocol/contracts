use cosmwasm_std::{testing::mock_env, Decimal};
use mars_oracle_base::ContractError;
use mars_oracle_osmosis::{
    contract::entry::execute,
    msg::{ExecuteMsg, PriceSourceResponse},
    Downtime, DowntimeDetector, OsmosisPriceSource,
};
use mars_owner::OwnerError::NotOwner;
use mars_red_bank_types::oracle::QueryMsg;
use mars_testing::mock_info;

mod helpers;

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
    assert_eq!(err, ContractError::Owner(NotOwner {}))
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
fn setting_price_source_incorrect_denom() {
    let mut deps = helpers::setup_test();

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "!*jadfaefc".to_string(),
            price_source: OsmosisPriceSource::Fixed {
                price: Decimal::one(),
            },
        },
    );
    assert_eq!(
        res,
        Err(ContractError::InvalidDenom {
            reason: "First character is not ASCII alphabetic".to_string()
        })
    );

    let res_two = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "ahdbufenf&*!-".to_string(),
            price_source: OsmosisPriceSource::Fixed {
                price: Decimal::one(),
            },
        },
    );
    assert_eq!(
        res_two,
        Err(ContractError::InvalidDenom {
            reason: "Not all characters are ASCII alphanumeric or one of:  /  :  .  _  -"
                .to_string()
        })
    );

    let res_three = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "ab".to_string(),
            price_source: OsmosisPriceSource::Fixed {
                price: Decimal::one(),
            },
        },
    );
    assert_eq!(
        res_three,
        Err(ContractError::InvalidDenom {
            reason: "Invalid denom length".to_string()
        })
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
fn setting_price_source_arithmetic_twap_with_invalid_params() {
    let mut deps = helpers::setup_test();

    let mut set_price_source_twap =
        |denom: &str,
         pool_id: u64,
         window_size: u64,
         downtime_detector: Option<DowntimeDetector>| {
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info("owner"),
                ExecuteMsg::SetPriceSource {
                    denom: denom.to_string(),
                    price_source: OsmosisPriceSource::ArithmeticTwap {
                        pool_id,
                        window_size,
                        downtime_detector,
                    },
                },
            )
        };

    // attempting to use a pool that does not contain the denom of interest; should fail
    let err = set_price_source_twap("umars", 1, 86400, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "pool 1 does not contain umars".to_string()
        }
    );

    // attempting to use a pool that does not contain the base denom, uosmo; should fail
    let err = set_price_source_twap("uatom", 64, 86400, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "pool 64 does not contain the base denom uosmo".to_string()
        }
    );

    // attempting to use a pool that contains more than two assets; should fail
    let err = set_price_source_twap("uusdc", 3333, 86400, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "expecting pool 3333 to contain exactly two coins; found 3".to_string()
        }
    );

    // attempting to use not XYK pool
    let err = set_price_source_twap("uion", 4444, 86400, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "assets in pool 4444 do not have equal weights".to_string()
        }
    );

    // attempting to set window_size bigger than 172800 sec (48h)
    let err = set_price_source_twap("umars", 89, 172801, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "expecting window size to be within 172800 sec".to_string()
        }
    );

    // attempting to set downtime recovery to 0
    let err = set_price_source_twap(
        "umars",
        89,
        86400,
        Some(DowntimeDetector {
            downtime: Downtime::Duration30s,
            recovery: 0,
        }),
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "downtime recovery can't be 0".to_string()
        }
    );
}

#[test]
fn setting_price_source_arithmetic_twap_successfully() {
    let mut deps = helpers::setup_test();

    // properly set twap price source
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "umars".to_string(),
            price_source: OsmosisPriceSource::ArithmeticTwap {
                pool_id: 89,
                window_size: 86400,
                downtime_detector: None,
            },
        },
    )
    .unwrap();
    assert_eq!(res.messages.len(), 0);

    let res: PriceSourceResponse = helpers::query(
        deps.as_ref(),
        QueryMsg::PriceSource {
            denom: "umars".to_string(),
        },
    );
    assert_eq!(
        res.price_source,
        OsmosisPriceSource::ArithmeticTwap {
            pool_id: 89,
            window_size: 86400,
            downtime_detector: None
        }
    );

    // properly set twap price source with downtime detector
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "umars".to_string(),
            price_source: OsmosisPriceSource::ArithmeticTwap {
                pool_id: 89,
                window_size: 86400,
                downtime_detector: Some(DowntimeDetector {
                    downtime: Downtime::Duration30m,
                    recovery: 360u64,
                }),
            },
        },
    )
    .unwrap();
    assert_eq!(res.messages.len(), 0);

    let res: PriceSourceResponse = helpers::query(
        deps.as_ref(),
        QueryMsg::PriceSource {
            denom: "umars".to_string(),
        },
    );
    assert_eq!(
        res.price_source,
        OsmosisPriceSource::ArithmeticTwap {
            pool_id: 89,
            window_size: 86400,
            downtime_detector: Some(DowntimeDetector {
                downtime: Downtime::Duration30m,
                recovery: 360u64
            })
        }
    );
}

#[test]
fn setting_price_source_geometric_twap_with_invalid_params() {
    let mut deps = helpers::setup_test();

    let mut set_price_source_twap =
        |denom: &str,
         pool_id: u64,
         window_size: u64,
         downtime_detector: Option<DowntimeDetector>| {
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info("owner"),
                ExecuteMsg::SetPriceSource {
                    denom: denom.to_string(),
                    price_source: OsmosisPriceSource::GeometricTwap {
                        pool_id,
                        window_size,
                        downtime_detector,
                    },
                },
            )
        };

    // attempting to use a pool that does not contain the denom of interest; should fail
    let err = set_price_source_twap("umars", 1, 86400, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "pool 1 does not contain umars".to_string()
        }
    );

    // attempting to use a pool that does not contain the base denom, uosmo; should fail
    let err = set_price_source_twap("uatom", 64, 86400, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "pool 64 does not contain the base denom uosmo".to_string()
        }
    );

    // attempting to use a pool that contains more than two assets; should fail
    let err = set_price_source_twap("uusdc", 3333, 86400, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "expecting pool 3333 to contain exactly two coins; found 3".to_string()
        }
    );

    // attempting to use not XYK pool
    let err = set_price_source_twap("uion", 4444, 86400, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "assets in pool 4444 do not have equal weights".to_string()
        }
    );

    // attempting to set window_size bigger than 172800 sec (48h)
    let err = set_price_source_twap("umars", 89, 172801, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "expecting window size to be within 172800 sec".to_string()
        }
    );

    // attempting to set downtime recovery to 0
    let err = set_price_source_twap(
        "umars",
        89,
        86400,
        Some(DowntimeDetector {
            downtime: Downtime::Duration30s,
            recovery: 0,
        }),
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "downtime recovery can't be 0".to_string()
        }
    );
}

#[test]
fn setting_price_source_geometric_twap_successfully() {
    let mut deps = helpers::setup_test();

    // properly set twap price source
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "umars".to_string(),
            price_source: OsmosisPriceSource::GeometricTwap {
                pool_id: 89,
                window_size: 86400,
                downtime_detector: None,
            },
        },
    )
    .unwrap();
    assert_eq!(res.messages.len(), 0);

    let res: PriceSourceResponse = helpers::query(
        deps.as_ref(),
        QueryMsg::PriceSource {
            denom: "umars".to_string(),
        },
    );
    assert_eq!(
        res.price_source,
        OsmosisPriceSource::GeometricTwap {
            pool_id: 89,
            window_size: 86400,
            downtime_detector: None
        }
    );

    // properly set twap price source with downtime detector
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "umars".to_string(),
            price_source: OsmosisPriceSource::GeometricTwap {
                pool_id: 89,
                window_size: 86400,
                downtime_detector: Some(DowntimeDetector {
                    downtime: Downtime::Duration30m,
                    recovery: 360u64,
                }),
            },
        },
    )
    .unwrap();
    assert_eq!(res.messages.len(), 0);

    let res: PriceSourceResponse = helpers::query(
        deps.as_ref(),
        QueryMsg::PriceSource {
            denom: "umars".to_string(),
        },
    );
    assert_eq!(
        res.price_source,
        OsmosisPriceSource::GeometricTwap {
            pool_id: 89,
            window_size: 86400,
            downtime_detector: Some(DowntimeDetector {
                downtime: Downtime::Duration30m,
                recovery: 360u64
            })
        }
    );
}

#[test]
fn setting_price_source_staked_geometric_twap_with_invalid_params() {
    let mut deps = helpers::setup_test();

    let mut set_price_source_twap =
        |denom: &str,
         transitive_denom: &str,
         pool_id: u64,
         window_size: u64,
         downtime_detector: Option<DowntimeDetector>| {
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info("owner"),
                ExecuteMsg::SetPriceSource {
                    denom: denom.to_string(),
                    price_source: OsmosisPriceSource::StakedGeometricTwap {
                        transitive_denom: transitive_denom.to_string(),
                        pool_id,
                        window_size,
                        downtime_detector,
                    },
                },
            )
        };

    // attempting to use a pool that does not contain the denom of interest; should fail
    let err = set_price_source_twap("ustatom", "umars", 803, 86400, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "pool 803 does not contain the base denom umars".to_string()
        }
    );
    let err = set_price_source_twap("umars", "uatom", 803, 86400, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "pool 803 does not contain umars".to_string()
        }
    );

    // attempting to use a pool that contains more than two assets; should fail
    let err = set_price_source_twap("ustatom", "uatom", 3333, 86400, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "expecting pool 3333 to contain exactly two coins; found 3".to_string()
        }
    );

    // attempting to use not XYK pool
    let err = set_price_source_twap("ustatom", "uatom", 4444, 86400, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "assets in pool 4444 do not have equal weights".to_string()
        }
    );

    // attempting to set window_size bigger than 172800 sec (48h)
    let err = set_price_source_twap("ustatom", "uatom", 803, 172801, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "expecting window size to be within 172800 sec".to_string()
        }
    );

    // attempting to set downtime recovery to 0
    let err = set_price_source_twap(
        "ustatom",
        "uatom",
        803,
        86400,
        Some(DowntimeDetector {
            downtime: Downtime::Duration30s,
            recovery: 0,
        }),
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "downtime recovery can't be 0".to_string()
        }
    );
}

#[test]
fn setting_price_source_staked_geometric_twap_successfully() {
    let mut deps = helpers::setup_test();

    // properly set twap price source
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "ustatom".to_string(),
            price_source: OsmosisPriceSource::StakedGeometricTwap {
                transitive_denom: "uatom".to_string(),
                pool_id: 803,
                window_size: 86400,
                downtime_detector: None,
            },
        },
    )
    .unwrap();
    assert_eq!(res.messages.len(), 0);

    let res: PriceSourceResponse = helpers::query(
        deps.as_ref(),
        QueryMsg::PriceSource {
            denom: "ustatom".to_string(),
        },
    );
    assert_eq!(
        res.price_source,
        OsmosisPriceSource::StakedGeometricTwap {
            transitive_denom: "uatom".to_string(),
            pool_id: 803,
            window_size: 86400,
            downtime_detector: None
        }
    );

    // properly set twap price source with downtime detector
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "ustatom".to_string(),
            price_source: OsmosisPriceSource::StakedGeometricTwap {
                transitive_denom: "uatom".to_string(),
                pool_id: 803,
                window_size: 86400,
                downtime_detector: Some(DowntimeDetector {
                    downtime: Downtime::Duration30m,
                    recovery: 360u64,
                }),
            },
        },
    )
    .unwrap();
    assert_eq!(res.messages.len(), 0);

    let res: PriceSourceResponse = helpers::query(
        deps.as_ref(),
        QueryMsg::PriceSource {
            denom: "ustatom".to_string(),
        },
    );
    assert_eq!(
        res.price_source,
        OsmosisPriceSource::StakedGeometricTwap {
            transitive_denom: "uatom".to_string(),
            pool_id: 803,
            window_size: 86400,
            downtime_detector: Some(DowntimeDetector {
                downtime: Downtime::Duration30m,
                recovery: 360u64
            })
        }
    );
}

#[test]
fn setting_price_source_xyk_lp() {
    let mut deps = helpers::setup_test();

    let mut set_price_source_xyk_lp = |denom: &str, pool_id: u64| {
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("owner"),
            ExecuteMsg::SetPriceSource {
                denom: denom.to_string(),
                price_source: OsmosisPriceSource::XykLiquidityToken {
                    pool_id,
                },
            },
        )
    };

    // attempting to use a pool that contains more than two assets; should fail
    let err = set_price_source_xyk_lp("uusdc_uusdt_udai_lp", 3333).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "expecting pool 3333 to contain exactly two coins; found 3".to_string()
        }
    );

    // attempting to use not XYK pool
    let err = set_price_source_xyk_lp("uion_uosmo_lp", 4444).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "assets in pool 4444 do not have equal weights".to_string()
        }
    );

    // properly set xyk lp price source
    let res = set_price_source_xyk_lp("uosmo_umars_lp", 89).unwrap();
    assert_eq!(res.messages.len(), 0);

    let res: PriceSourceResponse = helpers::query(
        deps.as_ref(),
        QueryMsg::PriceSource {
            denom: "uosmo_umars_lp".to_string(),
        },
    );
    assert_eq!(
        res.price_source,
        OsmosisPriceSource::XykLiquidityToken {
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
                denom: "uatom".to_string(),
                price_source: OsmosisPriceSource::Spot {
                    pool_id: 1
                }
            },
            PriceSourceResponse {
                denom: "umars".to_string(),
                price_source: OsmosisPriceSource::Spot {
                    pool_id: 89
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
