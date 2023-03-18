use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Decimal, Uint128};
use mars_red_bank_types::red_bank::Market;
use mars_rover::{msg::query::Positions, traits::Coins};
use mars_rover_health_types::{
    Health,
    HealthError::{MissingMarket, MissingPrice, MissingVaultConfig, MissingVaultValues},
    HealthResult,
};

use crate::{CollateralValue, DenomsData, VaultsData};

/// `HealthComputer` is a shared struct with the frontend that gets compiled to wasm.
/// For this reason, it uses a dependency-injection-like pattern where all required data is needed up front.
#[cw_serde]
pub struct HealthComputer {
    pub positions: Positions,
    pub denoms_data: DenomsData,
    pub vaults_data: VaultsData,
    pub allowed_coins: Vec<String>,
}

impl HealthComputer {
    pub fn compute_health(&self) -> HealthResult<Health> {
        let CollateralValue {
            total_collateral_value,
            max_ltv_adjusted_collateral,
            liquidation_threshold_adjusted_collateral,
        } = self.calculate_collateral_value()?;

        let total_debt_value = self.calculate_total_debt_value()?;

        let max_ltv_health_factor = if total_debt_value.is_zero() {
            None
        } else {
            Some(Decimal::checked_from_ratio(max_ltv_adjusted_collateral, total_debt_value)?)
        };

        let liquidation_health_factor = if total_debt_value.is_zero() {
            None
        } else {
            Some(Decimal::checked_from_ratio(
                liquidation_threshold_adjusted_collateral,
                total_debt_value,
            )?)
        };

        Ok(Health {
            total_debt_value,
            total_collateral_value,
            max_ltv_adjusted_collateral,
            liquidation_threshold_adjusted_collateral,
            max_ltv_health_factor,
            liquidation_health_factor,
        })
    }

    fn calculate_total_debt_value(&self) -> HealthResult<Uint128> {
        let mut total = Uint128::zero();
        for debt in &self.positions.debts {
            let coin_price =
                self.denoms_data.prices.get(&debt.denom).ok_or(MissingPrice(debt.denom.clone()))?;
            let debt_value = debt.amount.checked_mul_floor(*coin_price)?;
            total = total.checked_add(debt_value)?;
        }
        Ok(total)
    }

    fn calculate_collateral_value(&self) -> HealthResult<CollateralValue> {
        let deposits = self.calculate_coins_value(&self.positions.deposits)?;
        let lends = self.calculate_coins_value(&self.positions.lends.to_coins())?;
        let vaults = self.calculate_vaults_value()?;

        Ok(CollateralValue {
            total_collateral_value: deposits
                .total_collateral_value
                .checked_add(vaults.total_collateral_value)?
                .checked_add(lends.total_collateral_value)?,
            max_ltv_adjusted_collateral: deposits
                .max_ltv_adjusted_collateral
                .checked_add(vaults.max_ltv_adjusted_collateral)?
                .checked_add(lends.max_ltv_adjusted_collateral)?,
            liquidation_threshold_adjusted_collateral: deposits
                .liquidation_threshold_adjusted_collateral
                .checked_add(vaults.liquidation_threshold_adjusted_collateral)?
                .checked_add(lends.liquidation_threshold_adjusted_collateral)?,
        })
    }

    fn calculate_coins_value(&self, coins: &[Coin]) -> HealthResult<CollateralValue> {
        let mut total_collateral_value = Uint128::zero();
        let mut max_ltv_adjusted_collateral = Uint128::zero();
        let mut liquidation_threshold_adjusted_collateral = Uint128::zero();

        for c in coins {
            let coin_price =
                self.denoms_data.prices.get(&c.denom).ok_or(MissingPrice(c.denom.clone()))?;
            let coin_value = c.amount.checked_mul_floor(*coin_price)?;
            total_collateral_value = total_collateral_value.checked_add(coin_value)?;

            let &Market {
                max_loan_to_value,
                liquidation_threshold,
                ..
            } = self.denoms_data.markets.get(&c.denom).ok_or(MissingMarket(c.denom.clone()))?;

            // If coin has been de-listed, drop MaxLTV to zero
            let checked_max_ltv = if self.allowed_coins.contains(&c.denom) {
                max_loan_to_value
            } else {
                Decimal::zero()
            };
            let max_ltv_adjusted = coin_value.checked_mul_floor(checked_max_ltv)?;
            max_ltv_adjusted_collateral =
                max_ltv_adjusted_collateral.checked_add(max_ltv_adjusted)?;

            let liq_adjusted = coin_value.checked_mul_floor(liquidation_threshold)?;
            liquidation_threshold_adjusted_collateral =
                liquidation_threshold_adjusted_collateral.checked_add(liq_adjusted)?;
        }
        Ok(CollateralValue {
            total_collateral_value,
            max_ltv_adjusted_collateral,
            liquidation_threshold_adjusted_collateral,
        })
    }

    fn calculate_vaults_value(&self) -> HealthResult<CollateralValue> {
        let mut total_collateral_value = Uint128::zero();
        let mut max_ltv_adjusted_collateral = Uint128::zero();
        let mut liquidation_threshold_adjusted_collateral = Uint128::zero();

        for v in &self.positions.vaults {
            let values = self
                .vaults_data
                .vault_values
                .get(&v.vault.address)
                .ok_or(MissingVaultValues(v.vault.address.to_string()))?;

            total_collateral_value = total_collateral_value.checked_add(values.total_value()?)?;

            let config = self
                .vaults_data
                .vault_configs
                .get(&v.vault.address)
                .ok_or(MissingVaultConfig(v.vault.address.to_string()))?;

            // If vault or base token has been de-listed, drop MaxLTV to zero
            let base_token_whitelisted = self.allowed_coins.contains(&values.base_coin.denom);
            let checked_vault_max_ltv = if config.whitelisted && base_token_whitelisted {
                config.max_ltv
            } else {
                Decimal::zero()
            };

            max_ltv_adjusted_collateral = values
                .vault_coin
                .value
                .checked_mul_floor(checked_vault_max_ltv)?
                .checked_add(max_ltv_adjusted_collateral)?;

            liquidation_threshold_adjusted_collateral = values
                .vault_coin
                .value
                .checked_mul_floor(config.liquidation_threshold)?
                .checked_add(liquidation_threshold_adjusted_collateral)?;

            let &Market {
                max_loan_to_value,
                liquidation_threshold,
                ..
            } = self
                .denoms_data
                .markets
                .get(&values.base_coin.denom)
                .ok_or(MissingMarket(values.base_coin.denom.clone()))?;

            // If base token has been de-listed, drop MaxLTV to zero
            let checked_base_max_ltv = if base_token_whitelisted {
                max_loan_to_value
            } else {
                Decimal::zero()
            };

            max_ltv_adjusted_collateral = values
                .base_coin
                .value
                .checked_mul_floor(checked_base_max_ltv)?
                .checked_add(max_ltv_adjusted_collateral)?;

            liquidation_threshold_adjusted_collateral = values
                .base_coin
                .value
                .checked_mul_floor(liquidation_threshold)?
                .checked_add(liquidation_threshold_adjusted_collateral)?;
        }

        Ok(CollateralValue {
            total_collateral_value,
            max_ltv_adjusted_collateral,
            liquidation_threshold_adjusted_collateral,
        })
    }
}
