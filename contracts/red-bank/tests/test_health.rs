use std::collections::HashMap;

use cosmwasm_std::testing::mock_env;
use cosmwasm_std::{Addr, Decimal, Uint128};

use mars_outpost::red_bank::{Market, Position};
use mars_red_bank::health::{compute_position_health, get_user_positions_map};
use mars_red_bank::interest_rates::SCALING_FACTOR;
use mars_testing::mock_dependencies;

mod helpers;

#[test]
fn test_user_positions_map() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();
    let user_addr = Addr::unchecked("larry");
    let oracle_addr = Addr::unchecked("oracle");

    // initialize markets and oracle prices
    ["atom", "osmo", "scrt", "juno", "steak"].iter().for_each(|token| {
        let denom = format!("u{}", token);
        let ma_token = format!("ma{}", token);

        helpers::th_init_market(
            deps.as_mut(),
            &denom,
            &Market {
                denom: denom.clone(),
                ma_token_address: Addr::unchecked(&ma_token),
                borrow_index: Decimal::one(),
                liquidity_index: Decimal::one(),
                borrow_rate: Decimal::zero(),
                liquidity_rate: Decimal::zero(),
                indexes_last_updated: env.block.time.seconds(),
                ..Default::default()
            },
        );

        deps.querier.set_oracle_price(&denom, Decimal::one());
    });

    // user has ATOM collateral and enabled
    // expect to have a non-zero collateral amount in the positions map
    helpers::set_collateral(deps.as_mut(), &user_addr, "uatom", true);
    deps.querier.set_cw20_balances(
        Addr::unchecked("maatom"),
        &[(user_addr.clone(), Uint128::new(10000) * SCALING_FACTOR)],
    );

    // user has OSMO collateral but disabled
    // expect to have a zero collateral amount in the positions map
    helpers::set_collateral(deps.as_mut(), &user_addr, "uosmo", false);
    deps.querier.set_cw20_balances(
        Addr::unchecked("maosmo"),
        &[(user_addr.clone(), Uint128::new(20000) * SCALING_FACTOR)],
    );

    // user has SCRT debt and no uncollateralized loan limit
    // expect to have a non-zero debt amount in the positions map
    helpers::set_debt(deps.as_mut(), &user_addr, "uscrt", Uint128::new(30000) * SCALING_FACTOR);

    // user has JUNO debt which does not exceed uncollateralized loan limit
    // expect to have a zero debt amount in the positions map
    helpers::set_debt(deps.as_mut(), &user_addr, "ujuno", Uint128::new(40000) * SCALING_FACTOR);
    helpers::set_uncollatateralized_loan_limit(deps.as_mut(), &user_addr, "ujuno", 69420u128);

    // user has STEAK debt which exceeds uncollateralized loan limit
    // expect to have a non-zero debt amount in the positions map
    helpers::set_debt(deps.as_mut(), &user_addr, "usteak", Uint128::new(50000) * SCALING_FACTOR);
    helpers::set_uncollatateralized_loan_limit(deps.as_mut(), &user_addr, "usteak", 12345u128);

    let positions = get_user_positions_map(deps.as_ref(), &env, &user_addr, &oracle_addr).unwrap();

    assert_eq!(positions.get("uatom").unwrap().collateral_amount, Uint128::new(10000));
    assert!(positions.get("uosmo").unwrap().collateral_amount.is_zero());
    assert_eq!(positions.get("uscrt").unwrap().debt_amount, Uint128::new(30000));
    assert!(positions.get("ujuno").unwrap().debt_amount.is_zero());
    assert_eq!(positions.get("usteak").unwrap().debt_amount, Uint128::new(37655));
}

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

    // Has no debt (or, has debts but they do not exceed the C2C limit)
    let mut osmo_position = default_osmo_position();
    osmo_position.collateral_amount = Uint128::from(500u128);
    let positions = HashMap::from([("osmo".to_string(), osmo_position)]);

    let health = compute_position_health(&positions).unwrap();

    assert_eq!(health.total_collateral_value, Decimal::from_atomics(11827u128, 1).unwrap());
    assert_eq!(health.total_debt_value, Decimal::zero());
    assert_eq!(health.max_ltv_health_factor, None);
    assert_eq!(health.liquidation_health_factor, None);
    assert!(!health.is_liquidatable());
    assert!(!health.is_above_max_ltv());

    // Has ATOM debt, but no OSMO debt (or, has OSMO debt but it does not exceed the C2C limit)
    let mut osmo_position = default_osmo_position();
    osmo_position.debt_amount = Uint128::zero();
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

    // Has both ATOM and OSMO debts
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
        max_ltv: Decimal::from_atomics(50u128, 2).unwrap(),
        liquidation_threshold: Decimal::from_atomics(55u128, 2).unwrap(),
        asset_price: Decimal::from_atomics(23654u128, 4).unwrap(),
        ..Default::default()
    }
}

fn default_atom_position() -> Position {
    Position {
        denom: "atom".to_string(),
        max_ltv: Decimal::from_atomics(70u128, 2).unwrap(),
        liquidation_threshold: Decimal::from_atomics(75u128, 2).unwrap(),
        asset_price: Decimal::from_atomics(102u128, 1).unwrap(),
        ..Default::default()
    }
}
