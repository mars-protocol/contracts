use crate::error::{MarsHealthError, MarsHealthResult};
use cosmwasm_std::{Addr, Coin, Decimal, QuerierWrapper};
use mars_outpost::{
    math::divide_decimal_by_decimal,
    oracle::QueryMsg as OracleQueryMsg,
    red_bank::{Market, QueryMsg as RedBankQueryMsg},
};
pub struct HealthFactor {
    max_ltv_hf: Decimal,
    liq_threshold_hf: Decimal,
    total_debt_value: Decimal,
    total_collateral_value: Decimal,
}

impl HealthFactor {
    /// Compute the health of a token's position
    /// max_tvl = maximum loan to value
    /// lqdt = liquidation threshold
    pub fn compute(
        querier: &QuerierWrapper,
        oracle_addr: &Addr,
        redbank_addr: &Addr,
        assets: Vec<Coin>,
        debts: Vec<Coin>,
    ) -> MarsHealthResult<HealthFactor> {
        let total_debt_value = debts
            .iter()
            .try_fold(Decimal::zero(), |total, coin| {
                let price: Decimal = querier.query_wasm_smart(
                    oracle_addr,
                    &OracleQueryMsg::Price {
                        denom: coin.denom.clone(),
                    },
                )?;
                let value = price.checked_mul(Decimal::new(coin.amount))?;
                Ok(total + value)
            })
            .and_then(|total| {
                // A health factor can't be computed when debt is zero (dividing by zero)
                if total.is_zero() {
                    return Err(MarsHealthError::InvalidDebt {});
                }
                Ok(total)
            })?;

        let (
            total_collateral_value,
            total_collaterol_max_ltv_adjusted,
            total_collaterol_lqdt_threshold_adjusted,
        ) = assets.iter().try_fold::<_, _, MarsHealthResult<_>>(
            (Decimal::zero(), Decimal::zero(), Decimal::zero()),
            |(total_value, total_max_ltv, total_lqdt_threshold), coin| {
                let price: Decimal = querier.query_wasm_smart(
                    oracle_addr,
                    &OracleQueryMsg::Price {
                        denom: coin.denom.clone(),
                    },
                )?;

                let Market {
                    max_loan_to_value,
                    liquidation_threshold,
                    ..
                } = querier.query_wasm_smart(
                    redbank_addr,
                    &RedBankQueryMsg::Market {
                        denom: coin.denom.clone(),
                    },
                )?;

                let value = price.checked_mul(Decimal::new(coin.amount))?;
                Ok((
                    total_value + value,
                    total_max_ltv + value.checked_mul(max_loan_to_value)?,
                    total_lqdt_threshold + value.checked_mul(liquidation_threshold)?,
                ))
            },
        )?;

        Ok(HealthFactor {
            max_ltv_hf: divide_decimal_by_decimal(total_collaterol_max_ltv_adjusted, total_debt_value)?,
            liq_threshold_hf: divide_decimal_by_decimal(
                total_collaterol_lqdt_threshold_adjusted,
                total_debt_value,
            )?,
            total_debt_value,
            total_collateral_value,
        })
    }

    #[inline]
    pub fn is_liquidatable(&self) -> bool {
        self.liq_threshold_hf <= Decimal::one()
    }

    #[inline]
    pub fn is_healthy(&self) -> bool {
        self.max_ltv_hf > Decimal::one()
    }
}
