use crate::query::MarsQuerier;
use cosmwasm_std::{Addr, Coin, Decimal, QuerierWrapper, StdResult};
use mars_outpost::{math::divide_decimal_by_decimal, red_bank::Market};
use std::collections::HashMap;

#[derive(Default, Debug, Clone)]
pub struct Position {
    pub denom: String,
    pub price: Decimal,
    pub collateral_amount: Decimal,
    pub debt_amount: Decimal,
    pub max_ltv: Decimal,
    pub liquidation_threshold: Decimal,
}

#[derive(Default, Debug, PartialEq, Eq)]
pub struct Health {
    /// The sum of the value of all debts
    pub total_debt_value: Decimal,
    /// The sum of the value of all collaterals
    pub total_collateral_value: Decimal,
    /// The sum of the value of all colletarals adjusted by their Max LTV
    pub max_ltv_adjusted_collateral: Decimal,
    /// The sum of the vallue of all colletarals adjusted by their Liquidation Threshold
    pub lqdt_threshold_adjusted_collateral: Decimal,
    /// The sum of the value of all collaterals multiplied by their max LTV, over the total value of debt
    pub max_ltv_health_factor: Option<Decimal>,
    /// The sum of the value of all collaterals multiplied by their liquidation threshold over the total value of debt
    pub liquidation_health_factor: Option<Decimal>,
}

impl Health {
    /// Compute the health of a token's position
    /// max_tvl = maximum loan to value
    /// lqdt = liquidation threshold
    pub fn compute_from_coins(
        querier: &QuerierWrapper,
        oracle_addr: &Addr,
        red_bank_addr: &Addr,
        collateral: &[Coin],
        debt: &[Coin],
    ) -> StdResult<Health> {
        let mut positions: HashMap<String, Position> = HashMap::new();
        let querier = MarsQuerier::new(querier, oracle_addr.clone(), red_bank_addr.clone());

        collateral.iter().try_for_each::<_, StdResult<_>>(|c| {
            match positions.get_mut(&c.denom) {
                Some(p) => {
                    p.collateral_amount = Decimal::from_ratio(c.amount, 1u128);
                }
                None => {
                    let Market {
                        max_loan_to_value,
                        liquidation_threshold,
                        ..
                    } = querier.query_market(&c.denom)?;

                    positions.insert(
                        c.denom.clone(),
                        Position {
                            denom: c.denom.clone(),
                            collateral_amount: Decimal::from_ratio(c.amount, 1u128),
                            debt_amount: Decimal::zero(),
                            price: querier.query_price(&c.denom)?,
                            max_ltv: max_loan_to_value,
                            liquidation_threshold,
                        },
                    );
                }
            }
            Ok(())
        })?;

        debt.iter().try_for_each::<_, StdResult<_>>(|c| {
            match positions.get_mut(&c.denom) {
                Some(p) => {
                    p.debt_amount = Decimal::from_ratio(c.amount, 1u128);
                }
                None => {
                    let Market {
                        max_loan_to_value,
                        liquidation_threshold,
                        ..
                    } = querier.query_market(&c.denom)?;

                    positions.insert(
                        c.denom.clone(),
                        Position {
                            denom: c.denom.clone(),
                            collateral_amount: Decimal::zero(),
                            debt_amount: Decimal::from_ratio(c.amount, 1u128),
                            price: querier.query_price(&c.denom)?,
                            max_ltv: max_loan_to_value,
                            liquidation_threshold,
                        },
                    );
                }
            }
            Ok(())
        })?;

        Self::compute_health(&positions.into_values().collect::<Vec<_>>())
    }

    /// Compute the health of a collection of `AssetPosition`
    pub fn compute_health(positions: &[Position]) -> StdResult<Health> {
        let mut health = positions.iter().try_fold::<_, _, StdResult<Health>>(
            Health::default(),
            |mut h, p| {
                let collateral_value = p.collateral_amount.checked_mul(p.price)?;
                h.total_debt_value += p.debt_amount.checked_mul(p.price)?;
                h.total_collateral_value += collateral_value;
                h.max_ltv_adjusted_collateral += collateral_value.checked_mul(p.max_ltv)?;
                h.lqdt_threshold_adjusted_collateral +=
                    collateral_value.checked_mul(p.liquidation_threshold)?;
                Ok(h)
            },
        )?;

        // If there aren't any debts a health factor can't be computed (divide by zero)
        if health.total_debt_value > Decimal::zero() {
            health.max_ltv_health_factor = Some(divide_decimal_by_decimal(
                health.max_ltv_adjusted_collateral,
                health.total_debt_value,
            )?);
            health.liquidation_health_factor = Some(divide_decimal_by_decimal(
                health.lqdt_threshold_adjusted_collateral,
                health.total_debt_value,
            )?);
        }

        Ok(health)
    }

    #[inline]
    pub fn is_liquidatable(&self) -> bool {
        self.liquidation_health_factor.map_or(false, |hf| hf < Decimal::one())
    }

    #[inline]
    pub fn is_healthy(&self) -> bool {
        self.max_ltv_health_factor.map_or(true, |hf| hf > Decimal::one())
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use cosmwasm_std::{coins, testing::MockQuerier, Uint128};
    use mars_outpost::red_bank::Market;
    use mars_testing::MarsMockQuerier;

    use super::*;
    // Action:
    //      - User deposits 300 osmo
    //
    // Expected Result:
    //      - Health: MarsHealthError::TotalDebtIsZero
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
        assert!(health.is_healthy());
    }

    // Action: User requested to borrrow 100 osmo. Zero deposits
    // Collateral:  [0]
    // Debt:        [100 OSMO]
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
        assert!(!health.is_healthy());
    }

    // Step 1: User deposits 300 OSMO
    //         User borrows 50 OSMO
    // Collateral:  [300 OSMO]
    // Debt:        [50 OSMO]
    #[test]
    fn test_healthy_health_factor_1() {
        let positions = vec![Position {
            denom: "osmo".to_string(),
            debt_amount: Decimal::from_atomics(50u128, 0).unwrap(),
            collateral_amount: Decimal::from_atomics(300u128, 0).unwrap(),
            price: Decimal::from_atomics(23654u128, 4).unwrap(),
            max_ltv: Decimal::from_atomics(50u128, 2).unwrap(),
            liquidation_threshold: Decimal::from_atomics(55u128, 2).unwrap(),
        }];

        let health = Health::compute_health(&positions).unwrap();

        assert_eq!(
            health.total_collateral_value,
            Decimal::from_atomics(Uint128::new(70962), 2).unwrap()
        );
        assert_eq!(health.total_debt_value, Decimal::from_atomics(11827u128, 2).unwrap());
        assert_eq!(
            health.liquidation_health_factor,
            Some(Decimal::from_atomics(33u128, 1).unwrap())
        );
        assert_eq!(health.max_ltv_health_factor, Some(Decimal::from_atomics(3u128, 0).unwrap()));
        assert!(!health.is_liquidatable());
        assert!(health.is_healthy());
    }

    // Step 1: User deposits 300 OSMO
    //         User borrows 50 ATOM
    // Collateral:  [300 OSMO]
    // Debt:        [50 OSMO]
    #[test]
    fn test_healthy_health_factor_2() {
        let mock_querier = mock_setup();

        let collateral = coins(300, "osmo");
        let debt = coins(50, "osmo");

        let health = Health::compute_from_coins(
            &QuerierWrapper::new(&mock_querier),
            &Addr::unchecked("oracle"),
            &Addr::unchecked("red_bank"),
            &collateral,
            &debt,
        )
        .unwrap();

        assert_eq!(
            health.total_collateral_value,
            Decimal::from_atomics(Uint128::new(70962), 2).unwrap()
        );
        assert_eq!(health.total_debt_value, Decimal::from_atomics(11827u128, 2).unwrap());
        assert_eq!(
            health.liquidation_health_factor,
            Some(Decimal::from_atomics(33u128, 1).unwrap())
        );
        assert_eq!(health.max_ltv_health_factor, Some(Decimal::from_atomics(3u128, 0).unwrap()));
        assert!(!health.is_liquidatable());
        assert!(health.is_healthy());
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
}
