use cosmwasm_std::{Decimal, Uint128};
use mars_outpost::red_bank::Position;
use std::collections::HashMap;

use mars_red_bank::health::compute_position_health;

#[test]
fn test_health_position() {
    // No Debt No Collateral
    let positions = HashMap::new();
    let health = compute_position_health(&positions).unwrap();

    assert_eq!(health.total_collateral_value, Decimal::zero());
    assert_eq!(health.total_debt_value, Decimal::zero());
    assert_eq!(health.max_ltv_health_factor, None);
    assert_eq!(health.liquidation_health_factor, None);
    assert!(!health.is_liquidatable());
    assert!(!health.is_above_max_ltv());

    // Debt only uncollateralized
    let mut osmo_position = default_osmo_position();
    osmo_position.uncollateralized_debt = true;
    osmo_position.debt_amount = Uint128::from(100u128);
    osmo_position.collateral_amount = Uint128::from(500u128);
    let positions = HashMap::from([("osmo".to_string(), osmo_position)]);

    let health = compute_position_health(&positions).unwrap();

    assert_eq!(health.total_collateral_value, Decimal::from_atomics(11827u128, 1).unwrap());
    assert_eq!(health.total_debt_value, Decimal::zero());
    assert_eq!(health.max_ltv_health_factor, None);
    assert_eq!(health.liquidation_health_factor, None);
    assert!(!health.is_liquidatable());
    assert!(!health.is_above_max_ltv());

    // Debt (Collateralized + Uncollateralized)
    let mut osmo_position = default_osmo_position();
    osmo_position.uncollateralized_debt = true;
    osmo_position.debt_amount = Uint128::from(500u128);
    osmo_position.collateral_amount = Uint128::from(2500u128);

    let mut atom_position = default_atom_position();
    atom_position.debt_amount = Uint128::from(200u128);

    let positions =
        HashMap::from([("osmo".to_string(), osmo_position), ("atom".to_string(), atom_position)]);
    let health = compute_position_health(&positions).unwrap();

    assert_eq!(health.total_collateral_value, Decimal::from_atomics(59135u128, 1).unwrap());
    assert_eq!(health.total_debt_value, Decimal::from_ratio(2040u128, 1u128));
    assert!(!health.is_liquidatable());
    assert!(!health.is_above_max_ltv());

    // Debt (Collateralized)
    let mut osmo_position = default_osmo_position();
    osmo_position.debt_amount = Uint128::from(500u128);
    osmo_position.collateral_amount = Uint128::from(2500u128);

    let mut atom_position = default_atom_position();
    atom_position.debt_amount = Uint128::from(200u128);

    let positions =
        HashMap::from([("osmo".to_string(), osmo_position), ("atom".to_string(), atom_position)]);
    let health = compute_position_health(&positions).unwrap();

    assert_eq!(health.total_collateral_value, Decimal::from_atomics(59135u128, 1).unwrap());
    assert_eq!(health.total_debt_value, Decimal::from_atomics(32227u128, 1).unwrap());
    assert!(!health.is_liquidatable());
    assert!(health.is_above_max_ltv());
}

fn default_osmo_position() -> Position {
    Position {
        denom: "osmo".to_string(),
        uncollateralized_debt: false,
        max_ltv: Decimal::from_atomics(50u128, 2).unwrap(),
        liquidation_threshold: Decimal::from_atomics(55u128, 2).unwrap(),
        asset_price: Decimal::from_atomics(23654u128, 4).unwrap(),
        ..Default::default()
    }
}

fn default_atom_position() -> Position {
    Position {
        denom: "atom".to_string(),
        uncollateralized_debt: false,
        max_ltv: Decimal::from_atomics(70u128, 2).unwrap(),
        liquidation_threshold: Decimal::from_atomics(75u128, 2).unwrap(),
        asset_price: Decimal::from_atomics(102u128, 1).unwrap(),
        ..Default::default()
    }
}
