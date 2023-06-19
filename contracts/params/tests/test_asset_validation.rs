use std::str::FromStr;

use cosmwasm_std::Decimal;
use mars_params::{
    error::ContractError::Validation,
    msg::AssetParamsUpdate,
    types::hls::{HlsAssetType, HlsParamsUnchecked},
};
use mars_utils::error::ValidationError::{InvalidDenom, InvalidParam};

use crate::helpers::{assert_err, default_asset_params, MockEnv};

pub mod helpers;

#[test]
fn denom_must_be_native() {
    let mut mock = MockEnv::new().build().unwrap();
    let denom = "AA".to_string(); // Invalid native denom length

    let res = mock.update_asset_params(
        &mock.query_owner(),
        AssetParamsUpdate::AddOrUpdate {
            params: default_asset_params(&denom),
        },
    );
    assert_err(
        res,
        Validation(InvalidDenom {
            reason: "Invalid denom length".to_string(),
        }),
    );
}

#[test]
fn max_ltv_less_than_one() {
    let mut mock = MockEnv::new().build().unwrap();
    let mut params = default_asset_params("denom_xyz");
    params.max_loan_to_value = Decimal::from_str("1.1235").unwrap();

    let res = mock.update_asset_params(
        &mock.query_owner(),
        AssetParamsUpdate::AddOrUpdate {
            params,
        },
    );
    assert_err(
        res,
        Validation(InvalidParam {
            param_name: "max_loan_to_value".to_string(),
            invalid_value: "1.1235".to_string(),
            predicate: "< 1".to_string(),
        }),
    );
}

#[test]
fn liquidation_threshold_less_than_or_equal_to_one() {
    let mut mock = MockEnv::new().build().unwrap();
    let mut params = default_asset_params("denom_xyz");
    params.liquidation_threshold = Decimal::from_str("1.1235").unwrap();

    let res = mock.update_asset_params(
        &mock.query_owner(),
        AssetParamsUpdate::AddOrUpdate {
            params,
        },
    );
    assert_err(
        res,
        Validation(InvalidParam {
            param_name: "liquidation_threshold".to_string(),
            invalid_value: "1.1235".to_string(),
            predicate: "<= 1".to_string(),
        }),
    );
}

#[test]
fn liq_threshold_gt_max_ltv() {
    let mut mock = MockEnv::new().build().unwrap();
    let mut params = default_asset_params("denom_xyz");
    params.liquidation_threshold = Decimal::from_str("0.5").unwrap();
    params.max_loan_to_value = Decimal::from_str("0.6").unwrap();

    let res = mock.update_asset_params(
        &mock.query_owner(),
        AssetParamsUpdate::AddOrUpdate {
            params,
        },
    );
    assert_err(
        res,
        Validation(InvalidParam {
            param_name: "liquidation_threshold".to_string(),
            invalid_value: "0.5".to_string(),
            predicate: "> 0.6 (max LTV)".to_string(),
        }),
    );
}

#[test]
fn hls_max_ltv_less_than_one() {
    let mut mock = MockEnv::new().build().unwrap();
    let mut params = default_asset_params("denom_xyz");
    params.credit_manager.hls = Some(HlsParamsUnchecked {
        max_loan_to_value: Decimal::from_str("1.1235").unwrap(),
        liquidation_threshold: Decimal::from_str("0.5").unwrap(),
        correlations: vec![],
    });

    let res = mock.update_asset_params(
        &mock.query_owner(),
        AssetParamsUpdate::AddOrUpdate {
            params,
        },
    );
    assert_err(
        res,
        Validation(InvalidParam {
            param_name: "hls_max_loan_to_value".to_string(),
            invalid_value: "1.1235".to_string(),
            predicate: "< 1".to_string(),
        }),
    );
}

#[test]
fn hls_liquidation_threshold_less_than_or_equal_to_one() {
    let mut mock = MockEnv::new().build().unwrap();
    let mut params = default_asset_params("denom_xyz");
    params.credit_manager.hls = Some(HlsParamsUnchecked {
        max_loan_to_value: Decimal::from_str("0.6").unwrap(),
        liquidation_threshold: Decimal::from_str("1.1235").unwrap(),
        correlations: vec![],
    });

    let res = mock.update_asset_params(
        &mock.query_owner(),
        AssetParamsUpdate::AddOrUpdate {
            params,
        },
    );
    assert_err(
        res,
        Validation(InvalidParam {
            param_name: "hls_liquidation_threshold".to_string(),
            invalid_value: "1.1235".to_string(),
            predicate: "<= 1".to_string(),
        }),
    );
}

#[test]
fn hls_liq_threshold_gt_hls_max_ltv() {
    let mut mock = MockEnv::new().build().unwrap();
    let mut params = default_asset_params("denom_xyz");
    params.credit_manager.hls = Some(HlsParamsUnchecked {
        max_loan_to_value: Decimal::from_str("0.6").unwrap(),
        liquidation_threshold: Decimal::from_str("0.5").unwrap(),
        correlations: vec![],
    });

    let res = mock.update_asset_params(
        &mock.query_owner(),
        AssetParamsUpdate::AddOrUpdate {
            params,
        },
    );
    assert_err(
        res,
        Validation(InvalidParam {
            param_name: "hls_liquidation_threshold".to_string(),
            invalid_value: "0.5".to_string(),
            predicate: "> 0.6 (hls max LTV)".to_string(),
        }),
    );
}

#[test]
fn correlations_must_be_valid_denoms() {
    let mut mock = MockEnv::new().build().unwrap();
    let mut params = default_asset_params("denom_xyz");
    params.credit_manager.hls = Some(HlsParamsUnchecked {
        max_loan_to_value: Decimal::from_str("0.5").unwrap(),
        liquidation_threshold: Decimal::from_str("0.7").unwrap(),
        correlations: vec![HlsAssetType::Coin {
            denom: "AA".to_string(),
        }],
    });

    let res = mock.update_asset_params(
        &mock.query_owner(),
        AssetParamsUpdate::AddOrUpdate {
            params,
        },
    );
    assert_err(
        res,
        Validation(InvalidDenom {
            reason: "Invalid denom length".to_string(),
        }),
    );
}

#[test]
fn protocol_liquidation_fee_less_than_one() {
    let mut mock = MockEnv::new().build().unwrap();
    let mut params = default_asset_params("denom_xyz");
    params.protocol_liquidation_fee = Decimal::from_str("1").unwrap();

    let res = mock.update_asset_params(
        &mock.query_owner(),
        AssetParamsUpdate::AddOrUpdate {
            params,
        },
    );
    assert_err(
        res,
        Validation(InvalidParam {
            param_name: "protocol_liquidation_fee".to_string(),
            invalid_value: "1".to_string(),
            predicate: "< 1".to_string(),
        }),
    );
}

#[test]
fn liquidation_bonus_param_b_out_of_range() {
    let mut mock = MockEnv::new().build().unwrap();
    let mut params = default_asset_params("denom_xyz");
    params.liquidation_bonus.starting_lb = Decimal::from_str("0.101").unwrap();

    let res = mock.update_asset_params(
        &mock.query_owner(),
        AssetParamsUpdate::AddOrUpdate {
            params,
        },
    );
    assert_err(
        res,
        Validation(InvalidParam {
            param_name: "starting_lb".to_string(),
            invalid_value: "0.101".to_string(),
            predicate: "[0, 0.1]".to_string(),
        }),
    );
}

#[test]
fn liquidation_bonus_param_slope_out_of_range() {
    let mut mock = MockEnv::new().build().unwrap();
    let mut params = default_asset_params("denom_xyz");

    params.liquidation_bonus.slope = Decimal::from_str("0.99").unwrap();
    let res = mock.update_asset_params(
        &mock.query_owner(),
        AssetParamsUpdate::AddOrUpdate {
            params: params.clone(),
        },
    );
    assert_err(
        res,
        Validation(InvalidParam {
            param_name: "slope".to_string(),
            invalid_value: "0.99".to_string(),
            predicate: "[1, 5]".to_string(),
        }),
    );

    params.liquidation_bonus.slope = Decimal::from_str("5.01").unwrap();
    let res = mock.update_asset_params(
        &mock.query_owner(),
        AssetParamsUpdate::AddOrUpdate {
            params,
        },
    );
    assert_err(
        res,
        Validation(InvalidParam {
            param_name: "slope".to_string(),
            invalid_value: "5.01".to_string(),
            predicate: "[1, 5]".to_string(),
        }),
    );
}

#[test]
fn liquidation_bonus_param_min_lb_out_of_range() {
    let mut mock = MockEnv::new().build().unwrap();
    let mut params = default_asset_params("denom_xyz");
    params.liquidation_bonus.min_lb = Decimal::from_str("0.101").unwrap();

    let res = mock.update_asset_params(
        &mock.query_owner(),
        AssetParamsUpdate::AddOrUpdate {
            params,
        },
    );
    assert_err(
        res,
        Validation(InvalidParam {
            param_name: "min_lb".to_string(),
            invalid_value: "0.101".to_string(),
            predicate: "[0, 0.1]".to_string(),
        }),
    );
}

#[test]
fn liquidation_bonus_param_max_lb_out_of_range() {
    let mut mock = MockEnv::new().build().unwrap();
    let mut params = default_asset_params("denom_xyz");

    params.liquidation_bonus.max_lb = Decimal::from_str("0.0499").unwrap();
    let res = mock.update_asset_params(
        &mock.query_owner(),
        AssetParamsUpdate::AddOrUpdate {
            params: params.clone(),
        },
    );
    assert_err(
        res,
        Validation(InvalidParam {
            param_name: "max_lb".to_string(),
            invalid_value: "0.0499".to_string(),
            predicate: "[0.05, 0.3]".to_string(),
        }),
    );

    params.liquidation_bonus.max_lb = Decimal::from_str("0.31").unwrap();
    let res = mock.update_asset_params(
        &mock.query_owner(),
        AssetParamsUpdate::AddOrUpdate {
            params,
        },
    );
    assert_err(
        res,
        Validation(InvalidParam {
            param_name: "max_lb".to_string(),
            invalid_value: "0.31".to_string(),
            predicate: "[0.05, 0.3]".to_string(),
        }),
    );
}

#[test]
fn liquidation_bonus_param_max_lb_gt_min_lb() {
    let mut mock = MockEnv::new().build().unwrap();
    let mut params = default_asset_params("denom_xyz");
    params.liquidation_bonus.min_lb = Decimal::from_str("0.08").unwrap();
    params.liquidation_bonus.max_lb = Decimal::from_str("0.07").unwrap();

    let res = mock.update_asset_params(
        &mock.query_owner(),
        AssetParamsUpdate::AddOrUpdate {
            params,
        },
    );
    assert_err(
        res,
        Validation(InvalidParam {
            param_name: "max_lb".to_string(),
            invalid_value: "0.07".to_string(),
            predicate: "> 0.08 (min LB)".to_string(),
        }),
    );
}
