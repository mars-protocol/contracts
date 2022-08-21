use cosmwasm_std::{coin, coins, testing::MockQuerier, Addr, Decimal, QuerierWrapper};
use mars_health::health::Health;
use mars_outpost::red_bank::Market;
use mars_testing::MarsMockQuerier;

// Test to compute the health of a position where collateral is greater
// than zero, and debt is zero
//
// Position: Collateral: [(osmo:300)]
// Health:   collateral value: 709,62
///          debt value: 0
///          liquidatable: false
///          above_max_ltv: false
#[test]
fn test_collateral_no_debt() {
    let collateral = coins(300, "osmo");
    let mock_querier = mock_setup();
    let oracle_addr = Addr::unchecked("oracle");
    let red_bank_addr = Addr::unchecked("red_bank");

    let health = Health::compute_health_from_coins(
        &QuerierWrapper::new(&mock_querier),
        &oracle_addr,
        &red_bank_addr,
        &collateral,
        &[],
    )
    .unwrap();

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
// Position: Debt:[(osmo:100)]
// Health:   collateral value: 0
///          debt value: 709,62
///          liquidatable: true
///          above_max_ltv: true
#[test]
fn test_debt_no_collateral() {
    let debts = coins(100, "osmo");
    let mock_querier = mock_setup();
    let oracle_addr = Addr::unchecked("oracle");
    let red_bank_addr = Addr::unchecked("red_bank");

    let health = Health::compute_health_from_coins(
        &QuerierWrapper::new(&mock_querier),
        &oracle_addr,
        &red_bank_addr,
        &[],
        &debts,
    )
    .unwrap();

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
/// Health:    collateral value: 102
///            debt value 20.4
///            liquidatable: false
///            above_max_ltv: false
/// New price: atom price goes to zero
/// Health:    collateral value: 0
///            debt value 0
///            liquidatable: false
///            above_max_ltv: false
#[test]
fn test_no_collateral_no_debt() {
    let collateral = coins(10, "atom");
    let debts = coins(2, "atom");
    let mut mock_querier = mock_setup();
    let oracle_addr = Addr::unchecked("oracle");
    let red_bank_addr = Addr::unchecked("red_bank");

    let health = Health::compute_health_from_coins(
        &QuerierWrapper::new(&mock_querier),
        &oracle_addr,
        &red_bank_addr,
        &collateral,
        &debts,
    )
    .unwrap();

    assert_eq!(health.total_collateral_value, Decimal::from_atomics(102u128, 0).unwrap());
    assert_eq!(health.total_debt_value, Decimal::from_atomics(204u128, 1).unwrap());
    assert_eq!(health.max_ltv_health_factor, Some(Decimal::from_atomics(35u128, 1).unwrap()));
    assert_eq!(health.liquidation_health_factor, Some(Decimal::from_atomics(375u128, 2).unwrap()));
    assert!(!health.is_liquidatable());
    assert!(!health.is_above_max_ltv());

    mock_querier.set_oracle_price("atom", Decimal::zero());

    let health = Health::compute_health_from_coins(
        &QuerierWrapper::new(&mock_querier),
        &oracle_addr,
        &red_bank_addr,
        &[],
        &debts,
    )
    .unwrap();

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
/// Health:   collateral value: 809.62
///           debt value 236.54
///           liquidatable: false
///           above_max_ltv: false
#[test]
fn test_healthy_health_factor() {
    let collateral = vec![coin(100, "atom"), coin(200, "osmo"), coin(100, "osmo")];
    let debts = coins(100, "osmo");
    let mock_querier = mock_setup();
    let oracle_addr = Addr::unchecked("oracle");
    let red_bank_addr = Addr::unchecked("red_bank");

    let health = Health::compute_health_from_coins(
        &QuerierWrapper::new(&mock_querier),
        &oracle_addr,
        &red_bank_addr,
        &collateral,
        &debts,
    )
    .unwrap();

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
/// Health:   collateral value: 809.62
///           debt value 236.54
///           liquidatable: false
///           above_max_ltv: false
#[test]
fn test_above_max_ltv_not_liquidatable() {
    let collateral = vec![coin(50, "atom"), coin(200, "osmo"), coin(100, "osmo")];
    let debts = coins(50, "atom");
    let mut mock_querier = mock_setup();
    mock_querier.set_oracle_price("atom", Decimal::from_atomics(24u128, 0).unwrap());
    let oracle_addr = Addr::unchecked("oracle");
    let red_bank_addr = Addr::unchecked("red_bank");

    let health = Health::compute_health_from_coins(
        &QuerierWrapper::new(&mock_querier),
        &oracle_addr,
        &red_bank_addr,
        &collateral,
        &debts,
    )
    .unwrap();

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
/// Health:   collateral value: 809.62
///           debt value 236.54
///           liquidatable: false
///           above_max_ltv: false
#[test]
fn test_liquidatable() {
    let collateral = vec![coin(50, "atom"), coin(200, "osmo"), coin(100, "osmo")];
    let debts = coins(50, "atom");
    let mut mock_querier = mock_setup();
    mock_querier.set_oracle_price("atom", Decimal::from_atomics(35u128, 0).unwrap());
    let oracle_addr = Addr::unchecked("oracle");
    let red_bank_addr = Addr::unchecked("red_bank");

    let health = Health::compute_health_from_coins(
        &QuerierWrapper::new(&mock_querier),
        &oracle_addr,
        &red_bank_addr,
        &collateral,
        &debts,
    )
    .unwrap();

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

//  ----------------------------------------
//  |  ASSET  |  PRICE  |  MAX LTV  |  LT  |
//  ----------------------------------------
//  |  OSMO   | 2.3654  |    50     |  55  |
//  ----------------------------------------
//  |  ATOM   |   10.2  |    70     |  75  |
//  ----------------------------------------
fn mock_setup() -> MarsMockQuerier {
    let mut mock_querier = MarsMockQuerier::new(MockQuerier::new(&[]));
    // Set Markets
    let osmo_market = Market {
        denom: "osmo".to_string(),
        max_loan_to_value: Decimal::from_atomics(50u128, 2).unwrap(),
        liquidation_threshold: Decimal::from_atomics(55u128, 2).unwrap(),
        ..Default::default()
    };
    mock_querier.set_redbank_market(osmo_market);
    let atom_market = Market {
        denom: "atom".to_string(),
        max_loan_to_value: Decimal::from_atomics(70u128, 2).unwrap(),
        liquidation_threshold: Decimal::from_atomics(75u128, 2).unwrap(),
        ..Default::default()
    };
    mock_querier.set_redbank_market(atom_market);

    // Set prices in the oracle
    mock_querier.set_oracle_price("osmo", Decimal::from_atomics(23654u128, 4).unwrap());
    mock_querier.set_oracle_price("atom", Decimal::from_atomics(102u128, 1).unwrap());

    mock_querier
}
