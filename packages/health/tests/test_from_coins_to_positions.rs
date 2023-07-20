use std::collections::HashMap;

use cosmwasm_std::{
    coin, coins, testing::MockQuerier, Addr, Decimal, QuerierWrapper, StdError, Uint128,
};
use mars_health::{
    health::{Health, Position},
    query::MarsQuerier,
};
use mars_red_bank_types::red_bank::Market;
use mars_testing::MarsMockQuerier;

// Test converting a collection of coins (collateral and debts) to a map of `Position`
#[test]
fn from_coins_to_positions() {
    let oracle_addr = Addr::unchecked("oracle");
    let red_bank_addr = Addr::unchecked("red_bank");
    let mock_querier = mock_setup();
    let querier_wrapper = QuerierWrapper::new(&mock_querier);
    let querier = MarsQuerier::new(&querier_wrapper, &oracle_addr, &red_bank_addr);

    // 1. Collateral and no debt
    let collateral = coins(300, "osmo");
    let positions = Health::positions_from_coins(&querier, &collateral, &[]).unwrap();

    assert_eq!(
        positions,
        HashMap::from([(
            "osmo".to_string(),
            Position {
                denom: "osmo".to_string(),
                price: Decimal::from_atomics(23654u128, 4).unwrap(),
                collateral_amount: Uint128::from(300u128),
                debt_amount: Uint128::zero(),
                max_ltv: Decimal::from_atomics(50u128, 2).unwrap(),
                liquidation_threshold: Decimal::from_atomics(55u128, 2).unwrap()
            }
        )])
    );

    // 2. Debt and no Collateral
    let debt = coins(300, "osmo");
    let positions = Health::positions_from_coins(&querier, &[], &debt).unwrap();

    assert_eq!(
        positions,
        HashMap::from([(
            "osmo".to_string(),
            Position {
                denom: "osmo".to_string(),
                price: Decimal::from_atomics(23654u128, 4).unwrap(),
                collateral_amount: Uint128::zero(),
                debt_amount: Uint128::new(300),
                max_ltv: Decimal::from_atomics(50u128, 2).unwrap(),
                liquidation_threshold: Decimal::from_atomics(55u128, 2).unwrap()
            }
        )])
    );

    // 3. No Debt and no Collateral
    let positions = Health::positions_from_coins(&querier, &[], &[]).unwrap();

    assert_eq!(positions, HashMap::new());

    // 3. Multiple Coins
    let collateral = vec![coin(500, "osmo"), coin(200, "atom"), coin(0, "osmo")];
    let debt = vec![coin(200, "atom"), coin(150, "atom"), coin(115, "osmo")];
    let positions = Health::positions_from_coins(&querier, &collateral, &debt).unwrap();

    assert_eq!(
        positions,
        HashMap::from([
            (
                "osmo".to_string(),
                Position {
                    denom: "osmo".to_string(),
                    price: Decimal::from_atomics(23654u128, 4).unwrap(),
                    collateral_amount: Uint128::new(500),
                    debt_amount: Uint128::new(115),
                    max_ltv: Decimal::from_atomics(50u128, 2).unwrap(),
                    liquidation_threshold: Decimal::from_atomics(55u128, 2).unwrap()
                }
            ),
            (
                "atom".to_string(),
                Position {
                    denom: "atom".to_string(),
                    price: Decimal::from_atomics(102u128, 1).unwrap(),
                    collateral_amount: Uint128::new(200),
                    debt_amount: Uint128::new(350),
                    max_ltv: Decimal::from_atomics(70u128, 2).unwrap(),
                    liquidation_threshold: Decimal::from_atomics(75u128, 2).unwrap()
                }
            )
        ])
    );

    // 4. Multiple Coins
    let collateral = coins(250, "invalid_denom");
    let debt = vec![coin(200, "atom"), coin(150, "atom"), coin(115, "osmo")];
    let positions = Health::positions_from_coins(&querier, &collateral, &debt).unwrap_err();

    assert_eq!(
        positions,
        StdError::generic_err(
            "Querier contract error: [mock]: could not find the market for invalid_denom"
        )
    );
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
