use cosmwasm_std::Decimal;
use mars_health::health::{Health, Position};
use std::vec;

// Test to compute the health of a position where collateral is greater
// than zero, and debt is zero
//
// Action:  User deposits 300 osmo
/// Health: liquidatable: false
///         above_max_ltv: false
#[test]
fn test_collateral_no_debt() {
    let positions = vec![Position {
        denom: "osmo".to_string(),
        collateral_amount: Decimal::from_atomics(300u128, 0).unwrap(),
        price: Decimal::from_atomics(23654u128, 4).unwrap(),
        ..Default::default()
    }];

    let health = Health::compute_health(&positions).unwrap();

    assert_eq!(health.total_collateral_value, Decimal::from_atomics(70962u128, 2).unwrap());
    assert_eq!(health.total_debt_value, Decimal::zero());
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
fn test_debt_no_collateral() {
    let positions = vec![Position {
        denom: "osmo".to_string(),
        debt_amount: Decimal::from_atomics(100u128, 0).unwrap(),
        collateral_amount: Decimal::zero(),
        price: Decimal::from_atomics(23654u128, 4).unwrap(),
        max_ltv: Decimal::from_atomics(50u128, 2).unwrap(),
        liquidation_threshold: Decimal::from_atomics(55u128, 2).unwrap(),
    }];

    let health = Health::compute_health(&positions).unwrap();

    assert_eq!(health.total_collateral_value, Decimal::zero());
    assert_eq!(health.total_debt_value, Decimal::from_atomics(23654u128, 2).unwrap());
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
fn test_no_collateral_no_debt() {
    let positions = vec![Position {
        denom: "atom".to_string(),
        collateral_amount: Decimal::from_atomics(10u128, 0).unwrap(),
        debt_amount: Decimal::from_atomics(2u128, 0).unwrap(),
        price: Decimal::from_atomics(102u128, 1).unwrap(),
        max_ltv: Decimal::from_atomics(70u128, 2).unwrap(),
        liquidation_threshold: Decimal::from_atomics(75u128, 2).unwrap(),
    }];

    let health = Health::compute_health(&positions).unwrap();

    assert_eq!(health.total_collateral_value, Decimal::from_atomics(102u128, 0).unwrap());
    assert_eq!(health.total_debt_value, Decimal::from_atomics(204u128, 1).unwrap());
    assert_eq!(health.max_ltv_health_factor, Some(Decimal::from_atomics(35u128, 1).unwrap()));
    assert_eq!(health.liquidation_health_factor, Some(Decimal::from_atomics(375u128, 2).unwrap()));
    assert!(!health.is_liquidatable());
    assert!(!health.is_above_max_ltv());

    let new_positions = vec![Position {
        denom: "atom".to_string(),
        collateral_amount: Decimal::from_atomics(10u128, 0).unwrap(),
        debt_amount: Decimal::from_atomics(2u128, 0).unwrap(),
        price: Decimal::zero(),
        ..Default::default()
    }];

    let health = Health::compute_health(&new_positions).unwrap();

    assert_eq!(health.total_collateral_value, Decimal::zero());
    assert_eq!(health.total_debt_value, Decimal::zero());
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
fn test_healthy_health_factor() {
    let positions = vec![
        Position {
            denom: "osmo".to_string(),
            debt_amount: Decimal::from_atomics(100u128, 0).unwrap(),
            collateral_amount: Decimal::from_atomics(300u128, 0).unwrap(),
            price: Decimal::from_atomics(23654u128, 4).unwrap(),
            max_ltv: Decimal::from_atomics(50u128, 2).unwrap(),
            liquidation_threshold: Decimal::from_atomics(55u128, 2).unwrap(),
        },
        Position {
            denom: "atom".to_string(),
            debt_amount: Decimal::zero(),
            collateral_amount: Decimal::from_atomics(100u128, 0).unwrap(),
            price: Decimal::from_atomics(102u128, 1).unwrap(),
            max_ltv: Decimal::from_atomics(70u128, 2).unwrap(),
            liquidation_threshold: Decimal::from_atomics(75u128, 2).unwrap(),
        },
    ];

    let health = Health::compute_health(&positions).unwrap();

    assert_eq!(health.total_collateral_value, Decimal::from_atomics(172962u128, 2).unwrap());
    assert_eq!(health.total_debt_value, Decimal::from_atomics(23654u128, 2).unwrap());
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_atomics(4518516952735266762u128, 18).unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_atomics(4884125306502071531u128, 18).unwrap())
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
fn test_above_max_ltv_not_liquidatable() {
    let positions = vec![
        Position {
            denom: "osmo".to_string(),
            debt_amount: Decimal::zero(),
            collateral_amount: Decimal::from_atomics(300u128, 0).unwrap(),
            price: Decimal::from_atomics(23654u128, 4).unwrap(),
            max_ltv: Decimal::from_atomics(50u128, 2).unwrap(),
            liquidation_threshold: Decimal::from_atomics(55u128, 2).unwrap(),
        },
        Position {
            denom: "atom".to_string(),
            debt_amount: Decimal::from_atomics(50u128, 0).unwrap(),
            collateral_amount: Decimal::from_atomics(50u128, 0).unwrap(),
            price: Decimal::from_atomics(24u128, 0).unwrap(),
            max_ltv: Decimal::from_atomics(70u128, 2).unwrap(),
            liquidation_threshold: Decimal::from_atomics(75u128, 2).unwrap(),
        },
    ];

    let health = Health::compute_health(&positions).unwrap();

    assert_eq!(health.total_collateral_value, Decimal::from_atomics(190962u128, 2).unwrap());
    assert_eq!(health.total_debt_value, Decimal::from_atomics(1200u128, 0).unwrap());
    assert_eq!(health.max_ltv_health_factor, Some(Decimal::from_atomics(995675u128, 6).unwrap()));
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_atomics(10752425u128, 7).unwrap())
    );
    assert!(!health.is_liquidatable());
    assert!(health.is_above_max_ltv());
}

/// Test to compute a position that is liquidatable and above max tlv
/// Position: User Collateral: [(atom:50), (osmo:300)]
///           User Debt: [(atom:50)]
/// Health:   liquidatable: true
///           above_max_ltv: trie
#[test]
fn test_liquidatable() {
    let positions = vec![
        Position {
            denom: "osmo".to_string(),
            debt_amount: Decimal::zero(),
            collateral_amount: Decimal::from_atomics(300u128, 0).unwrap(),
            price: Decimal::from_atomics(23654u128, 4).unwrap(),
            max_ltv: Decimal::from_atomics(50u128, 2).unwrap(),
            liquidation_threshold: Decimal::from_atomics(55u128, 2).unwrap(),
        },
        Position {
            denom: "atom".to_string(),
            debt_amount: Decimal::from_atomics(50u128, 0).unwrap(),
            collateral_amount: Decimal::from_atomics(50u128, 0).unwrap(),
            price: Decimal::from_atomics(35u128, 0).unwrap(),
            max_ltv: Decimal::from_atomics(70u128, 2).unwrap(),
            liquidation_threshold: Decimal::from_atomics(75u128, 2).unwrap(),
        },
    ];

    let health = Health::compute_health(&positions).unwrap();

    assert_eq!(health.total_collateral_value, Decimal::from_atomics(245962u128, 2).unwrap());
    assert_eq!(health.total_debt_value, Decimal::from_atomics(1750u128, 0).unwrap());
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_atomics(902748571428571428u128, 18).unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_atomics(973023428571428571u128, 18).unwrap())
    );
    assert!(health.is_liquidatable());
    assert!(health.is_above_max_ltv());
}
