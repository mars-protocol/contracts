use crate::{
    asset::Asset,
    error::{MarsHealthError, MarsHealthResult},
    query::MarsQuerier,
};
use cosmwasm_std::{Addr, Coin, Decimal, QuerierWrapper};
use mars_outpost::math::divide_decimal_by_decimal;

#[derive(Debug, PartialEq, Eq)]
pub struct HealthFactor {
    pub max_ltv_hf: Decimal,
    pub liq_threshold_hf: Decimal,
    pub total_debt_value: Decimal,
    pub total_collateral_value: Decimal,
}

impl HealthFactor {
    /// Compute the health of a token's position
    /// max_tvl = maximum loan to value
    /// lqdt = liquidation threshold
    pub fn compute_from_coins(
        querier: &QuerierWrapper,
        oracle_addr: &Addr,
        redbank_addr: &Addr,
        assets: &[Coin],
        debts: &[Coin],
    ) -> MarsHealthResult<HealthFactor> {
        let querier = MarsQuerier::new(querier, oracle_addr.clone(), redbank_addr.clone());
        let debts = Asset::try_assets_from_coins(&querier, debts)?;
        let assets = Asset::try_assets_from_coins(&querier, assets)?;

        Self::compute_health_factor(&assets, &debts)
    }

    pub fn compute_health_factor(
        assets: &[Asset],
        debts: &[Asset],
    ) -> MarsHealthResult<HealthFactor> {
        let total_debt_value = debts
            .iter()
            .try_fold(Decimal::zero(), |total, asset| Ok(total + asset.value()?))
            .and_then(|total| {
                // A health factor can't be computed when debt is zero (dividing by zero)
                if total.is_zero() {
                    return Err(MarsHealthError::TotalDebtIsZero {});
                }
                Ok(total)
            })?;

        let (
            total_collateral_value,
            total_collaterol_max_ltv_adjusted,
            total_collaterol_lqdt_threshold_adjusted,
        ) = assets.iter().try_fold::<_, _, MarsHealthResult<_>>(
            (Decimal::zero(), Decimal::zero(), Decimal::zero()),
            |(total_value, total_max_ltv, total_lqdt_threshold), asset| {
                Ok((
                    total_value + asset.value()?,
                    total_max_ltv + asset.value_max_ltv_adjusted()?,
                    total_lqdt_threshold + asset.value_liq_threshold_adjusted()?,
                ))
            },
        )?;

        Ok(HealthFactor {
            max_ltv_hf: divide_decimal_by_decimal(
                total_collaterol_max_ltv_adjusted,
                total_debt_value,
            )?,
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
