use std::vec;

use cosmwasm_std::{CheckedFromRatioError, CheckedMultiplyRatioError, Decimal, Uint128};
use mars_health::{
    error::HealthError,
    health::{Health, Position},
};

// Test to compute the health of a position where collateral is greater
// than zero, and debt is zero
//
// Action:  User deposits 300 osmo
/// Health: liquidatable: false
///         above_max_ltv: false
#[test]
fn collateral_no_debt() {
    let positions = vec![Position {
        denom: "osmo".to_string(),
        collateral_amount: Uint128::new(300),
        price: Decimal::from_atomics(23654u128, 4).unwrap(),
        ..Default::default()
    }];

    let health = Health::compute_health(&positions).unwrap();

    assert_eq!(health.total_collateral_value, Uint128::new(709));
    assert_eq!(health.total_debt_value, Uint128::zero());
    assert_eq!(health.max_ltv_health_factor, None);
    assert_eq!(health.liquidation_health_factor, None);
    assert!(!health.is_liquidatable());
    assert!(!health.is_above_max_ltv());
}

// Test to compute the health of a position where collateral is zero,
// and debt is greater than zero
//
// Action:  User borrows 100 osmo
// Health:  liquidatable: true
///         above_max_ltv: true
#[test]
fn debt_no_collateral() {
    let positions = vec![Position {
        denom: "osmo".to_string(),
        debt_amount: Uint128::new(100),
        collateral_amount: Uint128::zero(),
        price: Decimal::from_atomics(23654u128, 4).unwrap(),
        max_ltv: Decimal::from_atomics(50u128, 2).unwrap(),
        liquidation_threshold: Decimal::from_atomics(55u128, 2).unwrap(),
    }];

    let health = Health::compute_health(&positions).unwrap();

    assert_eq!(health.total_collateral_value, Uint128::zero());
    assert_eq!(health.total_debt_value, Uint128::new(236));
    assert_eq!(health.liquidation_health_factor, Some(Decimal::zero()));
    assert_eq!(health.max_ltv_health_factor, Some(Decimal::zero()));
    assert!(health.is_liquidatable());
    assert!(health.is_above_max_ltv());
}

/// Test Terra Ragnarok case (collateral and debt are zero)
/// Position:  Collateral: [(atom:10)]
///            Debt: [(atom:2)]
/// Health:    liquidatable: false
///            above_max_ltv: false
/// New price: atom price goes to zero
/// Health:    liquidatable: false
///            above_max_ltv: false
#[test]
fn no_collateral_no_debt() {
    let positions = vec![Position {
        denom: "atom".to_string(),
        collateral_amount: Uint128::new(10),
        debt_amount: Uint128::new(2),
        price: Decimal::from_atomics(102u128, 1).unwrap(),
        max_ltv: Decimal::from_atomics(70u128, 2).unwrap(),
        liquidation_threshold: Decimal::from_atomics(75u128, 2).unwrap(),
    }];

    let health = Health::compute_health(&positions).unwrap();

    assert_eq!(health.total_collateral_value, Uint128::new(102));
    assert_eq!(health.total_debt_value, Uint128::new(20));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_atomics(3550000000000000000u128, 18).unwrap())
    );
    assert_eq!(health.liquidation_health_factor, Some(Decimal::from_atomics(380u128, 2).unwrap()));
    assert!(!health.is_liquidatable());
    assert!(!health.is_above_max_ltv());

    let new_positions = vec![Position {
        denom: "atom".to_string(),
        collateral_amount: Uint128::new(10),
        debt_amount: Uint128::new(2),
        price: Decimal::zero(),
        ..Default::default()
    }];

    let health = Health::compute_health(&new_positions).unwrap();

    assert_eq!(health.total_collateral_value, Uint128::zero());
    assert_eq!(health.total_debt_value, Uint128::zero());
    assert_eq!(health.max_ltv_health_factor, None);
    assert_eq!(health.liquidation_health_factor, None);
    assert!(!health.is_liquidatable());
    assert!(!health.is_above_max_ltv());
}

/// Test to compute a healthy position (not liquidatable and below max ltv)
/// Position: User Collateral: [(atom:100), (osmo:300)]
///           User Debt: [(osmo:100)]
/// Health:   liquidatable: false
///           above_max_ltv: false
#[test]
fn healthy_health_factor() {
    let positions = vec![
        Position {
            denom: "osmo".to_string(),
            debt_amount: Uint128::new(100),
            collateral_amount: Uint128::new(300),
            price: Decimal::from_atomics(23654u128, 4).unwrap(),
            max_ltv: Decimal::from_atomics(50u128, 2).unwrap(),
            liquidation_threshold: Decimal::from_atomics(55u128, 2).unwrap(),
        },
        Position {
            denom: "atom".to_string(),
            debt_amount: Uint128::zero(),
            collateral_amount: Uint128::new(100),
            price: Decimal::from_atomics(102u128, 1).unwrap(),
            max_ltv: Decimal::from_atomics(70u128, 2).unwrap(),
            liquidation_threshold: Decimal::from_atomics(75u128, 2).unwrap(),
        },
    ];

    let health = Health::compute_health(&positions).unwrap();

    assert_eq!(health.total_collateral_value, Uint128::new(1729));
    assert_eq!(health.total_debt_value, Uint128::new(236));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_atomics(4525423728813559322u128, 18).unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_atomics(4889830508474576271u128, 18).unwrap())
    );
    assert!(!health.is_liquidatable());
    assert!(!health.is_above_max_ltv());
}

/// Test to compute a position that is not liquidatable but above max ltv
/// Position: User Collateral: [(atom:50), (osmo:300)]
///           User Debt: [(atom:50)]
/// Health:   liquidatable: false
///           above_max_ltv: true
#[test]
fn above_max_ltv_not_liquidatable() {
    let positions = vec![
        Position {
            denom: "osmo".to_string(),
            debt_amount: Uint128::zero(),
            collateral_amount: Uint128::new(300),
            price: Decimal::from_atomics(23654u128, 4).unwrap(),
            max_ltv: Decimal::from_atomics(50u128, 2).unwrap(),
            liquidation_threshold: Decimal::from_atomics(55u128, 2).unwrap(),
        },
        Position {
            denom: "atom".to_string(),
            debt_amount: Uint128::new(50),
            collateral_amount: Uint128::new(50),
            price: Decimal::from_atomics(24u128, 0).unwrap(),
            max_ltv: Decimal::from_atomics(70u128, 2).unwrap(),
            liquidation_threshold: Decimal::from_atomics(75u128, 2).unwrap(),
        },
    ];

    let health = Health::compute_health(&positions).unwrap();

    assert_eq!(health.total_collateral_value, Uint128::new(1909));
    assert_eq!(health.total_debt_value, Uint128::new(1200));
    assert_eq!(health.max_ltv_health_factor, Some(Decimal::from_atomics(995000u128, 6).unwrap()));
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_atomics(1074166666666666666u128, 18).unwrap())
    );
    assert!(!health.is_liquidatable());
    assert!(health.is_above_max_ltv());
}

/// Test to compute a position that is liquidatable and above max tlv
/// Position: User Collateral: [(atom:50), (osmo:300)]
///           User Debt: [(atom:50)]
/// Health:   liquidatable: true
///           above_max_ltv: true
#[test]
fn liquidatable() {
    let positions = vec![
        Position {
            denom: "osmo".to_string(),
            debt_amount: Uint128::zero(),
            collateral_amount: Uint128::new(300),
            price: Decimal::from_atomics(23654u128, 4).unwrap(),
            max_ltv: Decimal::from_atomics(50u128, 2).unwrap(),
            liquidation_threshold: Decimal::from_atomics(55u128, 2).unwrap(),
        },
        Position {
            denom: "atom".to_string(),
            debt_amount: Uint128::new(50),
            collateral_amount: Uint128::new(50),
            price: Decimal::from_atomics(35u128, 0).unwrap(),
            max_ltv: Decimal::from_atomics(70u128, 2).unwrap(),
            liquidation_threshold: Decimal::from_atomics(75u128, 2).unwrap(),
        },
    ];

    let health = Health::compute_health(&positions).unwrap();

    assert_eq!(health.total_collateral_value, Uint128::new(2459));
    assert_eq!(health.total_debt_value, Uint128::new(1750));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_atomics(902285714285714285u128, 18).unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_atomics(972000000000000000u128, 18).unwrap())
    );
    assert!(health.is_liquidatable());
    assert!(health.is_above_max_ltv());
}

#[test]
fn health_errors() {
    let positions = vec![Position {
        denom: "osmo".to_string(),
        debt_amount: Uint128::zero(),
        collateral_amount: Uint128::MAX,
        price: Decimal::MAX,
        max_ltv: Decimal::from_atomics(50u128, 2).unwrap(),
        liquidation_threshold: Decimal::from_atomics(55u128, 2).unwrap(),
    }];

    let res_err = Health::compute_health(&positions).unwrap_err();
    assert_eq!(res_err, HealthError::CheckedMultiplyRatio(CheckedMultiplyRatioError::Overflow));

    let positions = vec![Position {
        denom: "osmo".to_string(),
        debt_amount: Uint128::one(),
        collateral_amount: Uint128::MAX,
        price: Decimal::one(),
        max_ltv: Decimal::percent(100),
        liquidation_threshold: Decimal::percent(100),
    }];

    let res_err = Health::compute_health(&positions).unwrap_err();
    assert_eq!(res_err, HealthError::CheckedFromRatio(CheckedFromRatioError::Overflow));
}
