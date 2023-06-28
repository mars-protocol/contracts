use std::cmp::min;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Decimal, Uint128};
use mars_params::types::{
    asset::{AssetParams, CmSettings},
    vault::VaultConfig,
};
use mars_rover::{msg::query::Positions, traits::Coins};
use mars_rover_health_types::{
    AccountKind, Health,
    HealthError::{
        DenomNotPresent, MissingHLSParams, MissingParams, MissingPrice, MissingVaultConfig,
        MissingVaultValues,
    },
    HealthResult,
};

use crate::{CollateralValue, DenomsData, VaultsData};

/// `HealthComputer` is a shared struct with the frontend that gets compiled to wasm.
/// For this reason, it uses a dependency-injection-like pattern where all required data is needed up front.
#[cw_serde]
pub struct HealthComputer {
    pub kind: AccountKind,
    pub positions: Positions,
    pub denoms_data: DenomsData,
    pub vaults_data: VaultsData,
}

impl HealthComputer {
    pub fn compute_health(&self) -> HealthResult<Health> {
        let CollateralValue {
            total_collateral_value,
            max_ltv_adjusted_collateral,
            liquidation_threshold_adjusted_collateral,
        } = self.total_collateral_value()?;

        let total_debt_value = self.total_debt_value()?;

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

    /// The max this account can withdraw of `withdraw_denom` and maintain max_ltv >= 1
    /// Note: This is an estimate. Guarantees to leave account healthy, but in edge cases,
    /// due to rounding, it may be slightly too conservative.
    pub fn max_withdraw_amount_estimate(&self, withdraw_denom: &str) -> HealthResult<Uint128> {
        let withdraw_coin = self
            .positions
            .deposits
            .iter()
            .find(|c| c.denom == withdraw_denom)
            .ok_or(DenomNotPresent(withdraw_denom.to_string()))?;

        let params = self
            .denoms_data
            .params
            .get(withdraw_denom)
            .ok_or(MissingParams(withdraw_denom.to_string()))?;

        // If no debt or coin is blacklisted (meaning does not contribute to max ltv hf),
        // the total amount deposited can be withdrawn
        if self.positions.debts.is_empty() || !params.credit_manager.whitelisted {
            return Ok(withdraw_coin.amount);
        }

        // Given the formula:
        //      max ltv health factor = max ltv adjusted value / debt value
        //          where: max ltv adjusted value = price * amount * max ltv
        // The max can be calculated as:
        //      1 = (total max ltv adjusted value - withdraw denom max ltv adjusted value) / debt value
        // Re-arranging this to isolate max withdraw amount renders:
        //      max withdraw amount = (total max ltv adjusted value - debt value) / (withdraw denom price * withdraw denom max ltv)
        let total_max_ltv_adjusted_value =
            self.total_collateral_value()?.max_ltv_adjusted_collateral;
        let debt_value = self.total_debt_value()?;
        let withdraw_denom_price = self
            .denoms_data
            .prices
            .get(withdraw_denom)
            .ok_or(MissingPrice(withdraw_denom.to_string()))?;

        let withdraw_denom_max_ltv = match self.kind {
            AccountKind::Default => params.max_loan_to_value,
            AccountKind::HighLeveredStrategy => {
                params
                    .credit_manager
                    .hls
                    .as_ref()
                    .ok_or(MissingHLSParams(withdraw_denom.to_string()))?
                    .max_loan_to_value
            }
        };

        if debt_value >= total_max_ltv_adjusted_value {
            return Ok(Uint128::zero());
        }

        // The formula in fact looks like this in practice:
        //      hf = rounddown(roundown(amount * price) * max ltv) / debt value
        // Which means re-arranging this to isolate withdraw amount is an estimate,
        // quite close, but never precisely right. For this reason, the - 1 below is meant
        // to err on the side of being more conservative vs aggressive.
        let max_withdraw_amount = total_max_ltv_adjusted_value
            .checked_sub(debt_value)?
            .checked_sub(Uint128::one())?
            .checked_div_floor(withdraw_denom_price.checked_mul(withdraw_denom_max_ltv)?)?;

        Ok(min(max_withdraw_amount, withdraw_coin.amount))
    }

    /// The max this account can borrow of `borrow_denom` and maintain max_ltv >= 1
    /// Note: This is an estimate. Guarantees to leave account healthy, but in edge cases,
    /// due to rounding, it may be slightly too conservative.
    pub fn max_borrow_amount_estimate(&self, borrow_denom: &str) -> HealthResult<Uint128> {
        // Given the formula:
        //      max ltv health factor = max ltv adjusted value / debt value
        //          where: max ltv adjusted value = price * amount * max ltv
        // The max borrow can be calculated as:
        //      1 = (max ltv adjusted value + (borrow denom amount * borrow denom price * borrow denom max ltv)) / (debt value + (borrow denom amount * borrow denom price))
        // Re-arranging this to isolate max borrow amount renders:
        //      max_borrow_denom_amount = (max_ltv_adjusted_value - debt_value) / (borrow_denom_price * (1 - borrow_denom_max_ltv))
        let total_max_ltv_adjusted_value =
            self.total_collateral_value()?.max_ltv_adjusted_collateral;
        let debt_value = self.total_debt_value()?;

        let params = self
            .denoms_data
            .params
            .get(borrow_denom)
            .ok_or(MissingParams(borrow_denom.to_string()))?;

        // Zero borrowable if unhealthy or not whitelisted
        if debt_value >= total_max_ltv_adjusted_value || !params.credit_manager.whitelisted {
            return Ok(Uint128::zero());
        }

        let borrow_denom_max_ltv = match self.kind {
            AccountKind::Default => params.max_loan_to_value,
            AccountKind::HighLeveredStrategy => {
                params
                    .credit_manager
                    .hls
                    .as_ref()
                    .ok_or(MissingHLSParams(borrow_denom.to_string()))?
                    .max_loan_to_value
            }
        };

        let borrow_denom_price = self
            .denoms_data
            .prices
            .get(borrow_denom)
            .cloned()
            .ok_or(MissingPrice(borrow_denom.to_string()))?;

        // The formula in fact looks like this in practice:
        //      hf = rounddown(roundown(amount * price) * max ltv) / debt value
        // Which means re-arranging this to isolate borrow amount is an estimate,
        // quite close, but never precisely right. For this reason, the - 1 below is meant
        // to err on the side of being more conservative vs aggressive.
        let max_borrow_amount = total_max_ltv_adjusted_value
            .checked_sub(debt_value)?
            .checked_sub(Uint128::one())?
            .checked_div_floor(
                Decimal::one()
                    .checked_sub(borrow_denom_max_ltv)?
                    .checked_mul(borrow_denom_price)?,
            )?;

        Ok(max_borrow_amount)
    }

    fn total_debt_value(&self) -> HealthResult<Uint128> {
        let mut total = Uint128::zero();
        for debt in &self.positions.debts {
            let coin_price =
                self.denoms_data.prices.get(&debt.denom).ok_or(MissingPrice(debt.denom.clone()))?;
            let debt_value = debt.amount.checked_mul_ceil(*coin_price)?;
            total = total.checked_add(debt_value)?;
        }
        Ok(total)
    }

    fn total_collateral_value(&self) -> HealthResult<CollateralValue> {
        let deposits = self.coins_value(&self.positions.deposits)?;
        let lends = self.coins_value(&self.positions.lends.to_coins())?;
        let vaults = self.vaults_value()?;

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

    fn coins_value(&self, coins: &[Coin]) -> HealthResult<CollateralValue> {
        let mut total_collateral_value = Uint128::zero();
        let mut max_ltv_adjusted_collateral = Uint128::zero();
        let mut liquidation_threshold_adjusted_collateral = Uint128::zero();

        for c in coins {
            let coin_price =
                self.denoms_data.prices.get(&c.denom).ok_or(MissingPrice(c.denom.clone()))?;
            let coin_value = c.amount.checked_mul_floor(*coin_price)?;
            total_collateral_value = total_collateral_value.checked_add(coin_value)?;

            let AssetParams {
                credit_manager:
                    CmSettings {
                        whitelisted,
                        hls,
                    },
                max_loan_to_value,
                liquidation_threshold,
                ..
            } = self.denoms_data.params.get(&c.denom).ok_or(MissingParams(c.denom.clone()))?;

            // If coin has been de-listed, drop MaxLTV to zero
            let checked_max_ltv = if *whitelisted {
                match self.kind {
                    AccountKind::Default => *max_loan_to_value,
                    AccountKind::HighLeveredStrategy => {
                        hls.as_ref().ok_or(MissingHLSParams(c.denom.clone()))?.max_loan_to_value
                    }
                }
            } else {
                Decimal::zero()
            };
            let max_ltv_adjusted = coin_value.checked_mul_floor(checked_max_ltv)?;
            max_ltv_adjusted_collateral =
                max_ltv_adjusted_collateral.checked_add(max_ltv_adjusted)?;

            let checked_liquidation_threshold = match self.kind {
                AccountKind::Default => *liquidation_threshold,
                AccountKind::HighLeveredStrategy => {
                    hls.as_ref().ok_or(MissingHLSParams(c.denom.clone()))?.liquidation_threshold
                }
            };
            let liq_adjusted = coin_value.checked_mul_floor(checked_liquidation_threshold)?;
            liquidation_threshold_adjusted_collateral =
                liquidation_threshold_adjusted_collateral.checked_add(liq_adjusted)?;
        }
        Ok(CollateralValue {
            total_collateral_value,
            max_ltv_adjusted_collateral,
            liquidation_threshold_adjusted_collateral,
        })
    }

    fn vaults_value(&self) -> HealthResult<CollateralValue> {
        let mut total_collateral_value = Uint128::zero();
        let mut max_ltv_adjusted_collateral = Uint128::zero();
        let mut liquidation_threshold_adjusted_collateral = Uint128::zero();

        for v in &self.positions.vaults {
            // Step 1: Calculate Vault coin values
            let values = self
                .vaults_data
                .vault_values
                .get(&v.vault.address)
                .ok_or(MissingVaultValues(v.vault.address.to_string()))?;

            total_collateral_value = total_collateral_value.checked_add(values.vault_coin.value)?;

            let VaultConfig {
                addr,
                max_loan_to_value,
                liquidation_threshold,
                whitelisted,
                hls,
                ..
            } = self
                .vaults_data
                .vault_configs
                .get(&v.vault.address)
                .ok_or(MissingVaultConfig(v.vault.address.to_string()))?;

            let base_params = self
                .denoms_data
                .params
                .get(&values.base_coin.denom)
                .ok_or(MissingParams(values.base_coin.denom.clone()))?;

            // If vault or base token has been de-listed, drop MaxLTV to zero
            let checked_vault_max_ltv = if *whitelisted && base_params.credit_manager.whitelisted {
                match self.kind {
                    AccountKind::Default => *max_loan_to_value,
                    AccountKind::HighLeveredStrategy => {
                        hls.as_ref().ok_or(MissingHLSParams(addr.to_string()))?.max_loan_to_value
                    }
                }
            } else {
                Decimal::zero()
            };

            max_ltv_adjusted_collateral = values
                .vault_coin
                .value
                .checked_mul_floor(checked_vault_max_ltv)?
                .checked_add(max_ltv_adjusted_collateral)?;

            let checked_liquidation_threshold = match self.kind {
                AccountKind::Default => *liquidation_threshold,
                AccountKind::HighLeveredStrategy => {
                    hls.as_ref().ok_or(MissingHLSParams(addr.to_string()))?.liquidation_threshold
                }
            };

            liquidation_threshold_adjusted_collateral = values
                .vault_coin
                .value
                .checked_mul_floor(checked_liquidation_threshold)?
                .checked_add(liquidation_threshold_adjusted_collateral)?;

            // Step 2: Calculate Base coin values
            let res = self.coins_value(&[Coin {
                denom: values.base_coin.denom.clone(),
                amount: v.amount.unlocking().total(),
            }])?;
            total_collateral_value =
                total_collateral_value.checked_add(res.total_collateral_value)?;
            max_ltv_adjusted_collateral =
                max_ltv_adjusted_collateral.checked_add(res.max_ltv_adjusted_collateral)?;
            liquidation_threshold_adjusted_collateral =
                liquidation_threshold_adjusted_collateral
                    .checked_add(res.liquidation_threshold_adjusted_collateral)?;
        }

        Ok(CollateralValue {
            total_collateral_value,
            max_ltv_adjusted_collateral,
            liquidation_threshold_adjusted_collateral,
        })
    }
}
