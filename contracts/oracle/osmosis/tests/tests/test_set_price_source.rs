use std::str::FromStr;

use cosmwasm_std::{testing::mock_env, Addr, Decimal};
use mars_oracle_base::ContractError;
use mars_oracle_osmosis::{
    contract::entry::execute,
    msg::{ExecuteMsg, PriceSourceResponse},
    DowntimeDetector, OsmosisPriceSourceChecked, OsmosisPriceSourceUnchecked, RedemptionRate, Twap,
    TwapKind,
};
use mars_owner::OwnerError::NotOwner;
use mars_testing::mock_info;
use mars_types::oracle::QueryMsg;
use mars_utils::error::ValidationError;
use osmosis_std::types::osmosis::downtimedetector::v1beta1::Downtime;
use pyth_sdk_cw::PriceIdentifier;
use test_case::test_case;

use super::helpers;

#[test]
fn setting_price_source_by_non_owner() {
    let mut deps = helpers::setup_test_with_pools();

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake"),
        ExecuteMsg::SetPriceSource {
            denom: "uosmo".to_string(),
            price_source: OsmosisPriceSourceUnchecked::Fixed {
                price: Decimal::one(),
            },
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Owner(NotOwner {}))
}

#[test]
fn setting_price_source_fixed() {
    let mut deps = helpers::setup_test_with_pools();

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "uosmo".to_string(),
            price_source: OsmosisPriceSourceUnchecked::Fixed {
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
        OsmosisPriceSourceChecked::Fixed {
            price: Decimal::one()
        }
    );
}

#[test]
fn setting_price_source_incorrect_denom() {
    let mut deps = helpers::setup_test_with_pools();

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "!*jadfaefc".to_string(),
            price_source: OsmosisPriceSourceUnchecked::Fixed {
                price: Decimal::one(),
            },
        },
    );
    assert_eq!(
        res,
        Err(ContractError::Validation(ValidationError::InvalidDenom {
            reason: "First character is not ASCII alphabetic".to_string()
        }))
    );

    let res_two = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "ahdbufenf&*!-".to_string(),
            price_source: OsmosisPriceSourceUnchecked::Fixed {
                price: Decimal::one(),
            },
        },
    );
    assert_eq!(
        res_two,
        Err(ContractError::Validation(ValidationError::InvalidDenom {
            reason: "Not all characters are ASCII alphanumeric or one of:  /  :  .  _  -"
                .to_string()
        }))
    );

    let res_three = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "ab".to_string(),
            price_source: OsmosisPriceSourceUnchecked::Fixed {
                price: Decimal::one(),
            },
        },
    );
    assert_eq!(
        res_three,
        Err(ContractError::Validation(ValidationError::InvalidDenom {
            reason: "Invalid denom length".to_string()
        }))
    );
}

#[test]
fn setting_price_source_spot() {
    let mut deps = helpers::setup_test_with_pools();

    let mut set_price_source_spot = |denom: &str, pool_id: u64| {
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("owner"),
            ExecuteMsg::SetPriceSource {
                denom: denom.to_string(),
                price_source: OsmosisPriceSourceUnchecked::Spot {
                    pool_id,
                },
            },
        )
    };

    // attempting to set price source for base denom; should fail
    let err = set_price_source_spot("uosmo", 1).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "denom and base denom can't be the same".to_string()
        }
    );

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

    // attempting to use a StableSwap pool that contains more than two assets; should fail
    let err = set_price_source_spot("uatom", 6666).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "expecting pool 6666 to contain exactly two coins; found 3".to_string()
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
        OsmosisPriceSourceChecked::Spot {
            pool_id: 89,
        }
    );
}

#[test]
fn setting_price_source_arithmetic_twap_with_invalid_params() {
    let mut deps = helpers::setup_test_with_pools();

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
                    price_source: OsmosisPriceSourceUnchecked::ArithmeticTwap {
                        pool_id,
                        window_size,
                        downtime_detector,
                    },
                },
            )
        };

    // attempting to set price source for base denom; should fail
    let err = set_price_source_twap("uosmo", 1, 86400, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "denom and base denom can't be the same".to_string()
        }
    );

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

    // attempting to use a StableSwap pool that contains more than two assets; should fail
    let err = set_price_source_twap("uatom", 6666, 86400, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "expecting pool 6666 to contain exactly two coins; found 3".to_string()
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
    let mut deps = helpers::setup_test_with_pools();

    // properly set twap price source
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "umars".to_string(),
            price_source: OsmosisPriceSourceUnchecked::ArithmeticTwap {
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
        OsmosisPriceSourceChecked::ArithmeticTwap {
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
            price_source: OsmosisPriceSourceUnchecked::ArithmeticTwap {
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
        OsmosisPriceSourceChecked::ArithmeticTwap {
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
    let mut deps = helpers::setup_test_with_pools();

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
                    price_source: OsmosisPriceSourceUnchecked::GeometricTwap {
                        pool_id,
                        window_size,
                        downtime_detector,
                    },
                },
            )
        };

    // attempting to set price source for base denom; should fail
    let err = set_price_source_twap("uosmo", 1, 86400, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "denom and base denom can't be the same".to_string()
        }
    );

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

    // attempting to use a StableSwap pool that contains more than two assets; should fail
    let err = set_price_source_twap("uatom", 6666, 86400, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "expecting pool 6666 to contain exactly two coins; found 3".to_string()
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
    let mut deps = helpers::setup_test_with_pools();

    // properly set twap price source
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "umars".to_string(),
            price_source: OsmosisPriceSourceUnchecked::GeometricTwap {
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
        OsmosisPriceSourceChecked::GeometricTwap {
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
            price_source: OsmosisPriceSourceUnchecked::GeometricTwap {
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
        OsmosisPriceSourceChecked::GeometricTwap {
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
    let mut deps = helpers::setup_test_with_pools();

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
                    price_source: OsmosisPriceSourceUnchecked::StakedGeometricTwap {
                        transitive_denom: transitive_denom.to_string(),
                        pool_id,
                        window_size,
                        downtime_detector,
                    },
                },
            )
        };

    // attempting to set price source for base denom; should fail
    let err = set_price_source_twap("uosmo", "uosmo", 1, 86400, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "denom and base denom can't be the same".to_string()
        }
    );

    // attempting to set price source with invalid transitive denom; should fail
    let err = set_price_source_twap("ustatom", "!*jadfaefc", 803, 86400, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::Validation(ValidationError::InvalidDenom {
            reason: "First character is not ASCII alphabetic".to_string()
        })
    );

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
    let err = set_price_source_twap("uion", "uosmo", 4444, 86400, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "assets in pool 4444 do not have equal weights".to_string()
        }
    );

    // attempting to use a StableSwap pool that contains more than two assets; should fail
    let err = set_price_source_twap("uatom", "uusdc", 6666, 86400, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "expecting pool 6666 to contain exactly two coins; found 3".to_string()
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
    let mut deps = helpers::setup_test_with_pools();

    // properly set twap price source
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "ustatom".to_string(),
            price_source: OsmosisPriceSourceUnchecked::StakedGeometricTwap {
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
        OsmosisPriceSourceChecked::StakedGeometricTwap {
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
            price_source: OsmosisPriceSourceUnchecked::StakedGeometricTwap {
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
        OsmosisPriceSourceChecked::StakedGeometricTwap {
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
fn setting_price_source_lsd_with_invalid_params() {
    let mut deps = helpers::setup_test_with_pools();

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
                    price_source: OsmosisPriceSourceUnchecked::Lsd {
                        transitive_denom: transitive_denom.to_string(),
                        twap: Twap {
                            pool_id,
                            window_size,
                            downtime_detector,
                            kind: TwapKind::GeometricTwap {},
                        },
                        redemption_rate: RedemptionRate {
                            contract_addr: "dummy_addr".to_string(),
                            max_staleness: 100,
                        },
                    },
                },
            )
        };

    // attempting to set price source for base denom; should fail
    let err = set_price_source_twap("uosmo", "uosmo", 1, 86400, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "denom and base denom can't be the same".to_string()
        }
    );

    // attempting to set price source with invalid transitive denom; should fail
    let err = set_price_source_twap("ustatom", "!*jadfaefc", 3333, 86400, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::Validation(ValidationError::InvalidDenom {
            reason: "First character is not ASCII alphabetic".to_string()
        })
    );

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
    let err = set_price_source_twap("uion", "uosmo", 4444, 86400, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "assets in pool 4444 do not have equal weights".to_string()
        }
    );

    // attempting to use a StableSwap pool that contains more than two assets; should fail
    let err = set_price_source_twap("uatom", "uusdc", 6666, 86400, None).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "expecting pool 6666 to contain exactly two coins; found 3".to_string()
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

#[test_case(
    TwapKind::ArithmeticTwap {},
    Some(DowntimeDetector {
        downtime: Downtime::Duration30m,
        recovery: 360,
    });
    "set LSD price source with arithmetic TWAP and downtime detector"
)]
#[test_case(
    TwapKind::ArithmeticTwap {},
    None;
    "set LSD price source with arithmetic TWAP and without downtime detector"
)]
#[test_case(
    TwapKind::GeometricTwap {},
    Some(DowntimeDetector {
        downtime: Downtime::Duration30m,
        recovery: 360,
    });
    "set LSD price source with geometric TWAP and downtime detector"
)]
#[test_case(
    TwapKind::GeometricTwap {},
    None;
    "set LSD price source with geometric TWAP and without downtime detector"
)]
fn asserting_lsd_price_source(twap_kind: TwapKind, downtime_detector: Option<DowntimeDetector>) {
    let mut deps = helpers::setup_test_with_pools();

    // properly set twap price source
    let unchecked_lsd_ps = OsmosisPriceSourceUnchecked::Lsd {
        transitive_denom: "uatom".to_string(),
        twap: Twap {
            pool_id: 803,
            window_size: 86400,
            downtime_detector,
            kind: twap_kind,
        },
        redemption_rate: RedemptionRate {
            contract_addr: "dummy_addr".to_string(),
            max_staleness: 100,
        },
    };
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "ustatom".to_string(),
            price_source: unchecked_lsd_ps.clone(),
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
    let checked_lsd_ps = unchecked_to_checked_lsd(unchecked_lsd_ps);
    assert_eq!(res.price_source, checked_lsd_ps);
}

fn unchecked_to_checked_lsd(ps: OsmosisPriceSourceUnchecked) -> OsmosisPriceSourceChecked {
    if let OsmosisPriceSourceUnchecked::Lsd {
        transitive_denom,
        twap,
        redemption_rate,
    } = ps
    {
        OsmosisPriceSourceChecked::Lsd {
            transitive_denom,
            twap,
            redemption_rate: RedemptionRate {
                contract_addr: Addr::unchecked(redemption_rate.contract_addr),
                max_staleness: redemption_rate.max_staleness,
            },
        }
    } else {
        panic!("invalid price source type")
    }
}

#[test]
fn setting_price_source_xyk_lp() {
    let mut deps = helpers::setup_test_with_pools();

    let mut set_price_source_xyk_lp = |denom: &str, pool_id: u64| {
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("owner"),
            ExecuteMsg::SetPriceSource {
                denom: denom.to_string(),
                price_source: OsmosisPriceSourceUnchecked::XykLiquidityToken {
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

    // attempting to use StableSwap pool
    let err = set_price_source_xyk_lp("atom_uosmo_lp", 5555).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "StableSwap pool not supported. Pool id 5555".to_string()
        }
    );

    // attempting to use ConcentratedLiquid pool
    let err = set_price_source_xyk_lp("ujuno_uosmo_lp", 7777).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "ConcentratedLiquidity pool not supported. Pool id 7777".to_string()
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
        OsmosisPriceSourceChecked::XykLiquidityToken {
            pool_id: 89,
        }
    );
}

#[test]
fn setting_price_source_pyth_with_invalid_params() {
    let mut deps = helpers::setup_test();

    let mut set_price_source_pyth =
        |max_confidence: Decimal, max_deviation: Decimal, denom_decimals: u8| {
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info("owner"),
                ExecuteMsg::SetPriceSource {
                    denom: "uatom".to_string(),
                    price_source: OsmosisPriceSourceUnchecked::Pyth {
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
fn setting_price_source_pyth_if_missing_usd() {
    let mut deps = helpers::setup_test();

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "uatom".to_string(),
            price_source: OsmosisPriceSourceUnchecked::Pyth {
                contract_addr: "new_pyth_contract_addr".to_string(),
                price_feed_id: PriceIdentifier::from_hex(
                    "61226d39beea19d334f17c2febce27e12646d84675924ebb02b9cdaea68727e3",
                )
                .unwrap(),
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
fn setting_price_source_pyth_successfully() {
    let mut deps = helpers::setup_test();

    // price source used to convert USD to base_denom
    helpers::set_price_source(
        deps.as_mut(),
        "usd",
        OsmosisPriceSourceUnchecked::Fixed {
            price: Decimal::from_str("1000000").unwrap(),
        },
    );

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "uatom".to_string(),
            price_source: OsmosisPriceSourceUnchecked::Pyth {
                contract_addr: "new_pyth_contract_addr".to_string(),
                price_feed_id: PriceIdentifier::from_hex(
                    "61226d39beea19d334f17c2febce27e12646d84675924ebb02b9cdaea68727e3",
                )
                .unwrap(),
                max_staleness: 30,
                max_confidence: Decimal::percent(12),
                max_deviation: Decimal::percent(14),
                denom_decimals: 8,
            },
        },
    )
    .unwrap();
    assert_eq!(res.messages.len(), 0);

    let res: PriceSourceResponse = helpers::query(
        deps.as_ref(),
        QueryMsg::PriceSource {
            denom: "uatom".to_string(),
        },
    );
    assert_eq!(
        res.price_source,
        OsmosisPriceSourceChecked::Pyth {
            contract_addr: Addr::unchecked("new_pyth_contract_addr"),
            price_feed_id: PriceIdentifier::from_hex(
                "61226d39beea19d334f17c2febce27e12646d84675924ebb02b9cdaea68727e3"
            )
            .unwrap(),
            max_staleness: 30,
            max_confidence: Decimal::percent(12),
            max_deviation: Decimal::percent(14),
            denom_decimals: 8,
        },
    );
}

#[test]
fn querying_price_source() {
    let mut deps = helpers::setup_test_with_pools();

    helpers::set_price_source(
        deps.as_mut(),
        "uosmo",
        OsmosisPriceSourceUnchecked::Fixed {
            price: Decimal::one(),
        },
    );
    helpers::set_price_source(
        deps.as_mut(),
        "uatom",
        OsmosisPriceSourceUnchecked::Spot {
            pool_id: 1,
        },
    );
    helpers::set_price_source(
        deps.as_mut(),
        "umars",
        OsmosisPriceSourceUnchecked::Spot {
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
        OsmosisPriceSourceChecked::Spot {
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
                price_source: OsmosisPriceSourceChecked::Spot {
                    pool_id: 1
                }
            },
            PriceSourceResponse {
                denom: "umars".to_string(),
                price_source: OsmosisPriceSourceChecked::Spot {
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
                price_source: OsmosisPriceSourceChecked::Spot {
                    pool_id: 89
                }
            },
            PriceSourceResponse {
                denom: "uosmo".to_string(),
                price_source: OsmosisPriceSourceChecked::Fixed {
                    price: Decimal::one()
                }
            }
        ]
    );
}
