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
fn max_ltv_less_than_or_equal_to_one() {
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
            predicate: "<= 1".to_string(),
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
fn liquidation_bonus_less_than_or_equal_to_one() {
    let mut mock = MockEnv::new().build().unwrap();
    let mut params = default_asset_params("denom_xyz");
    params.liquidation_bonus = Decimal::from_str("1.1235").unwrap();

    let res = mock.update_asset_params(
        &mock.query_owner(),
        AssetParamsUpdate::AddOrUpdate {
            params,
        },
    );
    assert_err(
        res,
        Validation(InvalidParam {
            param_name: "liquidation_bonus".to_string(),
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
fn hls_max_ltv_less_than_or_equal_to_one() {
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
            predicate: "<= 1".to_string(),
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
