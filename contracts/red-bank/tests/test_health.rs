use std::collections::HashMap;

use cosmwasm_std::{CheckedMultiplyRatioError, Decimal, Uint128};
use mars_health::error::HealthError;
use mars_red_bank::{error::ContractError, health::compute_position_health};
use mars_types::red_bank::Position;

#[test]
fn health_position() {
    // No Debt No Collateral
    let positions = HashMap::new();
    let health = compute_position_health(&positions).unwrap();

    assert_eq!(health.total_collateral_value, Uint128::zero());
    assert_eq!(health.total_debt_value, Uint128::zero());
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

    assert_eq!(health.total_collateral_value, Uint128::from(1182u128));
    assert_eq!(health.total_debt_value, Uint128::zero());
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

    assert_eq!(health.total_collateral_value, Uint128::from(5913u128));
    assert_eq!(health.total_debt_value, Uint128::from(2040u128));
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

    assert_eq!(health.total_collateral_value, Uint128::from(5913u128));
    assert_eq!(health.total_debt_value, Uint128::from(3222u128));
    assert!(!health.is_liquidatable());
    assert!(health.is_above_max_ltv());
}

#[test]
fn health_error_if_overflow() {
    let mut osmo_position = default_osmo_position();
    osmo_position.collateral_amount = Uint128::MAX;
    osmo_position.asset_price = Decimal::MAX;
    let positions = HashMap::from([("osmo".to_string(), osmo_position)]);
    let res_err = compute_position_health(&positions).unwrap_err();
    assert_eq!(
        res_err,
        ContractError::Health(HealthError::CheckedMultiplyRatio(
            CheckedMultiplyRatioError::Overflow
        ))
    );
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
