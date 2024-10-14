use std::cmp::min;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Decimal, Fraction, Uint128};
use mars_types::{
    credit_manager::Positions,
    health::{
        AccountKind, BorrowTarget, Health,
        HealthError::{
            MissingAmount, MissingHLSParams, MissingParams, MissingPrice, MissingVaultConfig,
            MissingVaultValues,
        },
        HealthResult, LiquidationPriceKind, SwapKind,
    },
    params::{AssetParams, CmSettings, HlsAssetType, VaultConfig},
};
#[cfg(feature = "javascript")]
use tsify::Tsify;

use crate::{CollateralValue, DenomsData, VaultsData};

/// `HealthComputer` is a shared struct with the frontend that gets compiled to wasm.
/// For this reason, it uses a dependency-injection-like pattern where all required data is needed up front.
#[cw_serde]
#[cfg_attr(feature = "javascript", derive(Tsify))]
#[cfg_attr(feature = "javascript", tsify(into_wasm_abi, from_wasm_abi))]
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
        // Both deposits and lends should be considered, as the funds can automatically be un-lent and
        // and also used to withdraw.
        let withdraw_coin = self.get_coin_from_deposits_and_lends(withdraw_denom)?;
        if withdraw_coin.amount.is_zero() {
            return Ok(Uint128::zero());
        };

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
            AccountKind::FundManager {
                ..
            } => params.max_loan_to_value,
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

    pub fn max_swap_amount_estimate(
        &self,
        from_denom: &str,
        to_denom: &str,
        kind: &SwapKind,
        slippage: Decimal,
    ) -> HealthResult<Uint128> {
        // Both deposits and lends should be considered, as the funds can automatically be un-lent and
        // and also used to swap.
        let from_coin = self.get_coin_from_deposits_and_lends(from_denom)?;

        // If no debt the total amount deposited can be swapped (only for default swaps)
        if kind == &SwapKind::Default && self.positions.debts.is_empty() {
            return Ok(from_coin.amount);
        }

        let total_max_ltv_adjusted_value =
            self.total_collateral_value()?.max_ltv_adjusted_collateral;

        let debt_value = self.total_debt_value()?;

        if debt_value >= total_max_ltv_adjusted_value {
            return Ok(Uint128::zero());
        }

        let from_ltv = self.get_coin_max_ltv(from_denom)?;
        let to_ltv = self.get_coin_max_ltv(to_denom)?;

        // Don't allow swapping when one of the assets is not whitelisted
        if from_ltv == Decimal::zero() || to_ltv == Decimal::zero() {
            return Ok(Uint128::zero());
        }

        let from_price =
            self.denoms_data.prices.get(from_denom).ok_or(MissingPrice(from_denom.to_string()))?;

        // An asset that has a price of 1 and max ltv of 0.5 has a collateral_value of 0.5.
        // Swapping that asset for an asset with the same price, but 0.8 max ltv results in a collateral_value of 0.8.
        // Therefore, when the asset that is swapped to has a higher or equal max ltv than the asset swapped from,
        // the collateral value will increase and we can allow the full balance to be swapped.
        // The ltv_out is adjusted for slippage, as the swap_out_value can drop by the slippage.
        let to_ltv_slippage_corrected = to_ltv.checked_mul(Decimal::one() - slippage)?;
        let swappable_amount = if to_ltv_slippage_corrected >= from_ltv {
            from_coin.amount
        } else {
            // In order to calculate the output of the swap, the formula looks like this:
            //     1 = (collateral_value + to_amount * to_price * to_ltv - from_amount * from_price * from_ltv) / debt_value
            // The unknown variables here are to_amount and from_amount. In order to only have 1 unknown variable, from_amount,
            // to_amount can be replaced by:
            //     to_amount = slippage * from_amount * from_price / to_price
            // This results in the following formula:
            //     1 = (collateral_value + slippage * from_amount * from_price / to_price * to_price * to_ltv - from_amount * from_price * from_ltv) / debt_value
            //     debt_value = collateral_value + slippage * from_amount * from_price * to_ltv - from_amount * from_price * from_ltv
            //     slippage * from_amount * from_price * to_ltv - from_amount * from_price * from_ltv = debt_value - collateral_value
            //     from_amount * (slippage * from_price * to_ltv - from_price * from_ltv) = debt_value - collateral_value
            // Rearranging this formula to isolate from_amount results in the following formula:
            //     from_amount = (debt_value - collateral_value) / (from_price * (slippage * to_ltv - from_ltv))
            // Rearranging to avoid negative numbers for the denominator (to_ltv_slippage_corrected < from_ltv):
            //     from_amount = (collateral_value - debt_value) / (from_price * (from_ltv - slippage * to_ltv)
            let amount = total_max_ltv_adjusted_value
                .checked_sub(debt_value)?
                .checked_sub(Uint128::one())?
                .checked_div_floor(from_price.checked_mul(from_ltv - to_ltv_slippage_corrected)?)?;

            // Cap the swappable amount at the current balance of the coin
            min(amount, from_coin.amount)
        };

        match kind {
            SwapKind::Default => Ok(swappable_amount),

            SwapKind::Margin => {
                // If the swappable amount is less than the available amount, no need to further calculate
                // the margin borrow amount.
                if swappable_amount < from_coin.amount {
                    return Ok(swappable_amount);
                }

                let from_coin_value = from_coin.amount.checked_mul_floor(*from_price)?;

                // This represents the max ltv adjusted value of the coin being swapped from
                let swap_from_ltv_value = from_coin_value.checked_mul_floor(from_ltv)?;

                // The from_denom is always taken on as debt, as the trade is in the bullish direction
                // of the to_denom (expecting it to outpace the borrow rate from the from_denom)
                let swap_to_ltv_value = from_coin_value.checked_mul_floor(to_ltv)?;

                let total_max_ltv_adjust_value_after_swap = total_max_ltv_adjusted_value
                    .checked_add(swap_to_ltv_value)?
                    .checked_sub(swap_from_ltv_value)?;

                // The total swappable amount for margin is represented by the available coin balance + the
                // the maximum amount that can be borrowed (and then swapped).
                // This is represented by the formula:
                //     1 = (collateral_after_swap + slippage * borrow_amount * borrow_price * to_ltv) / (debt + borrow_amount * borrow_price)
                //     debt + borrow_amount * borrow_price = collateral_after_swap + slippage * borrow_amount * borrow_price * to_ltv
                //     borrow_amount * borrow_price - slippage * borrow_amount * borrow_price * to_ltv = collateral_after_swap - debt
                //     borrow_amount * borrow_price * (1 - slippage * to_ltv) = collateral_after_swap - debt
                // Rearranging this results in:
                //     borrow_amount = (collateral_after_swap - debt) / (borrow_price * (1 - slippage * to_ltv))
                let borrow_amount = total_max_ltv_adjust_value_after_swap
                    .checked_sub(debt_value)?
                    .checked_sub(Uint128::one())?
                    .checked_div_floor(
                        Decimal::one()
                            .checked_sub(to_ltv_slippage_corrected)?
                            .checked_mul(*from_price)?,
                    )?;

                // The total amount that can be swapped is then the balance of the coin + the additional amount
                // that can be borrowed.
                Ok(borrow_amount.checked_add(from_coin.amount)?)
            }
        }
    }

    /// The max this account can borrow of `borrow_denom` and maintain max_ltv >= 1
    /// Note: This is an estimate. Guarantees to leave account healthy, but in edge cases,
    /// due to rounding, it may be slightly too conservative.
    pub fn max_borrow_amount_estimate(
        &self,
        borrow_denom: &str,
        target: &BorrowTarget,
    ) -> HealthResult<Uint128> {
        // Given the formula:
        //      max ltv health factor = max ltv adjusted value / debt value
        //          where: max ltv adjusted value = price * amount * max ltv
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
            AccountKind::FundManager {
                ..
            } => params.max_loan_to_value,
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

        // The formulas look like this in practice:
        //      hf = rounddown(roundown(amount * price) * max ltv) / debt value
        // Which means re-arranging this to isolate borrow amount is an estimate,
        // quite close, but never precisely right. For this reason, the - 1 of the formulas
        // below are meant to err on the side of being more conservative vs aggressive.
        let max_borrow_amount = match target {
            // The max borrow for deposit can be calculated as:
            //      1 = (max ltv adjusted value + (borrow denom amount * borrow denom price * borrow denom max ltv)) / (debt value + (borrow denom amount * borrow denom price))
            // Re-arranging this to isolate borrow denom amount renders:
            //      max_borrow_denom_amount = (max_ltv_adjusted_value - debt_value) / (borrow_denom_price * (1 - borrow_denom_max_ltv))
            BorrowTarget::Deposit => total_max_ltv_adjusted_value
                .checked_sub(debt_value)?
                .checked_sub(Uint128::one())?
                .checked_div_floor(
                    Decimal::one()
                        .checked_sub(borrow_denom_max_ltv)?
                        .checked_mul(borrow_denom_price)?,
                )?,

            // Borrowing assets to wallet does not count towards collateral. It only adds to debts.
            // Hence, the max borrow to wallet can be calculated as:
            //      1 = (max ltv adjusted value) / (debt value + (borrow denom amount * borrow denom price))
            // Re-arranging this to isolate borrow denom amount renders:
            //      borrow denom amount = (max ltv adjusted value - debt_value) / denom_price
            BorrowTarget::Wallet => total_max_ltv_adjusted_value
                .checked_sub(debt_value)?
                .checked_sub(Uint128::one())?
                .checked_div_floor(borrow_denom_price)?,

            // When borrowing assets to add to a vault, the amount deposited into the vault counts towards collateral.
            // The health factor can be calculated as:
            //     1 = (max ltv adjusted value + (borrow amount * borrow price * vault max ltv)) / (debt value + (borrow amount * borrow price))
            // Re-arranging this to isolate borrow amount renders:
            //     borrow amount = (max ltv adjusted value - debt value) / (borrow price * (1 - vault max ltv)
            BorrowTarget::Vault {
                address,
            } => {
                let VaultConfig {
                    addr,
                    max_loan_to_value,
                    whitelisted,
                    hls,
                    ..
                } = self
                    .vaults_data
                    .vault_configs
                    .get(address)
                    .ok_or(MissingVaultConfig(address.to_string()))?;

                // If vault or base token has been de-listed, drop MaxLTV to zero
                let checked_vault_max_ltv = if *whitelisted {
                    match self.kind {
                        AccountKind::Default => *max_loan_to_value,
                        AccountKind::FundManager {
                            ..
                        } => *max_loan_to_value,
                        AccountKind::HighLeveredStrategy => {
                            hls.as_ref()
                                .ok_or(MissingHLSParams(addr.to_string()))?
                                .max_loan_to_value
                        }
                    }
                } else {
                    Decimal::zero()
                };

                // The max borrow for deposit can be calculated as:
                //      1 = (total_max_ltv_adjusted_value + (max_borrow_denom_amount * borrow_denom_price * checked_vault_max_ltv)) / (debt_value + (max_borrow_denom_amount * borrow_denom_price))
                // Re-arranging this to isolate borrow denom amount renders:
                //      max_borrow_denom_amount = (total_max_ltv_adjusted_value - debt_value) / (borrow_denom_price * (1 - checked_vault_max_ltv))
                // Which means re-arranging this to isolate borrow amount is an estimate,
                // quite close, but never precisely right. For this reason, the - 1 of the formulas
                // below are meant to err on the side of being more conservative vs aggressive.
                total_max_ltv_adjusted_value
                    .checked_sub(debt_value)?
                    .checked_sub(Uint128::one())?
                    .checked_div_floor(
                    borrow_denom_price
                        .checked_mul(Decimal::one().checked_sub(checked_vault_max_ltv)?)?,
                )?
            }

            BorrowTarget::Swap {
                slippage,
                denom_out,
            } => {
                let denom_out_ltv = self.get_coin_max_ltv(denom_out).unwrap();

                // The max borrow for swap can be calculated as:
                //      1 = (total_max_ltv_adjusted_value + (denom_amount_out * denom_price_out * denom_out_ltv)) / (debt_value + (max_borrow_denom_amount * borrow_denom_price))
                // denom_amount_out can be replaced by:
                //      denom_amount_out = slippage * max_borrow_denom_amount * borrow_denom_price / denom_price_out
                // This results in the following formula:
                //      1 = (total_max_ltv_adjusted_value + (slippage * max_borrow_denom_amount * borrow_denom_price * denom_out_ltv)) / (debt_value + (max_borrow_denom_amount * borrow_denom_price))
                // Re-arranging this to isolate borrow denom amount renders:
                //      max_borrow_denom_amount = (total_max_ltv_adjusted_value - debt_value) / (borrow_denom_price * (1 - slippage * denom_out_ltv))
                let out_ltv_slippage_corrected =
                    denom_out_ltv.checked_mul(Decimal::one() - slippage)?;
                total_max_ltv_adjusted_value
                    .checked_sub(debt_value)?
                    .checked_sub(Uint128::one())?
                    .checked_div_floor(
                    Decimal::one()
                        .checked_sub(out_ltv_slippage_corrected)?
                        .checked_mul(borrow_denom_price)?,
                )?
            }
        };

        Ok(max_borrow_amount)
    }

    pub fn liquidation_price(
        &self,
        denom: &str,
        kind: &LiquidationPriceKind,
    ) -> HealthResult<Uint128> {
        let collateral_ltv_value = self.total_collateral_value()?.max_ltv_adjusted_collateral;
        let total_debt_value = self.total_debt_value()?;
        if total_debt_value.is_zero() {
            return Ok(Uint128::zero());
        }

        let current_price =
            self.denoms_data.prices.get(denom).ok_or(MissingPrice(denom.to_string()))?;

        if total_debt_value >= collateral_ltv_value {
            return Ok(Uint128::one() * *current_price);
        }

        match kind {
            LiquidationPriceKind::Asset => {
                let asset_amount = self.get_coin_from_deposits_and_lends(denom)?.amount;
                if asset_amount.is_zero() {
                    return Err(MissingAmount(denom.to_string()));
                }

                let asset_ltv = self.get_coin_max_ltv(denom)?;

                let asset_ltv_value =
                    asset_amount.checked_mul_floor(current_price.checked_mul(asset_ltv)?)?;
                let debt_with_asset_ltv_value = total_debt_value.checked_add(asset_ltv_value)?;

                if debt_with_asset_ltv_value <= collateral_ltv_value {
                    return Ok(Uint128::zero());
                }

                let debt_without = debt_with_asset_ltv_value - collateral_ltv_value;

                // liquidation_price = (debt_value - collateral_ltv_value + asset_ltv_value) / (asset_amount * asset_ltv)
                Ok(Uint128::one()
                    * Decimal::checked_from_ratio(debt_without, asset_amount)?.checked_mul(
                        Decimal::from_ratio(asset_ltv.denominator(), asset_ltv.numerator()),
                    )?)
            }

            LiquidationPriceKind::Debt => {
                let debt_amount = self
                    .positions
                    .debts
                    .iter()
                    .find(|c| c.denom == denom)
                    .ok_or(MissingAmount(denom.to_string()))?
                    .amount;
                if debt_amount.is_zero() {
                    return Err(MissingAmount(denom.to_string()));
                }

                // Liquidation_price = (collateral_ltv_value - total_debt_value + debt_value_asset / asset_amount
                let debt_value = debt_amount.checked_mul_ceil(*current_price)?;
                let net_collateral_value_without_debt =
                    collateral_ltv_value.checked_add(debt_value)?.checked_sub(total_debt_value)?;

                Ok(net_collateral_value_without_debt / debt_amount)
            }
        }
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
        let lends = self.coins_value(&self.positions.lends)?;
        let vaults = self.vaults_value()?;
        let staked_lp = self.coins_value(&self.positions.staked_astro_lps)?;

        Ok(CollateralValue {
            total_collateral_value: deposits
                .total_collateral_value
                .checked_add(vaults.total_collateral_value)?
                .checked_add(lends.total_collateral_value)?
                .checked_add(staked_lp.total_collateral_value)?,
            max_ltv_adjusted_collateral: deposits
                .max_ltv_adjusted_collateral
                .checked_add(vaults.max_ltv_adjusted_collateral)?
                .checked_add(lends.max_ltv_adjusted_collateral)?
                .checked_add(staked_lp.max_ltv_adjusted_collateral)?,
            liquidation_threshold_adjusted_collateral: deposits
                .liquidation_threshold_adjusted_collateral
                .checked_add(vaults.liquidation_threshold_adjusted_collateral)?
                .checked_add(lends.liquidation_threshold_adjusted_collateral)?
                .checked_add(staked_lp.liquidation_threshold_adjusted_collateral)?,
        })
    }

    fn coins_value(&self, coins: &[Coin]) -> HealthResult<CollateralValue> {
        let mut total_collateral_value = Uint128::zero();
        let mut max_ltv_adjusted_collateral = Uint128::zero();
        let mut liquidation_threshold_adjusted_collateral = Uint128::zero();

        for c in coins {
            let Some(AssetParams {
                credit_manager:
                    CmSettings {
                        hls,
                        ..
                    },
                liquidation_threshold,
                ..
            }) = self.coin_contribution_to_collateral(c)?
            else {
                continue;
            };

            let coin_price =
                self.denoms_data.prices.get(&c.denom).ok_or(MissingPrice(c.denom.clone()))?;
            let coin_value = c.amount.checked_mul_floor(*coin_price)?;
            total_collateral_value = total_collateral_value.checked_add(coin_value)?;

            let checked_max_ltv = self.get_coin_max_ltv(&c.denom)?;

            let max_ltv_adjusted = coin_value.checked_mul_floor(checked_max_ltv)?;
            max_ltv_adjusted_collateral =
                max_ltv_adjusted_collateral.checked_add(max_ltv_adjusted)?;

            let checked_liquidation_threshold = match self.kind {
                AccountKind::Default => *liquidation_threshold,
                AccountKind::FundManager {
                    ..
                } => *liquidation_threshold,
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

    fn coin_contribution_to_collateral(&self, coin: &Coin) -> HealthResult<Option<&AssetParams>> {
        let Some(asset_params) = self.denoms_data.params.get(&coin.denom) else {
            // If the coin is not found (whitelisted), it is not considered for collateral
            return Ok(None);
        };

        match self.kind {
            AccountKind::HighLeveredStrategy => {
                // HLS should have 0 or 1 debt denom in the account. If there are more than 1 we can safely calculate the collateral value
                // because the rule will be checked in the Credit Manager contract and won't allow more than 1 debt denom in the account.
                if !self.positions.debts.is_empty() {
                    let mut correlations = vec![];
                    for debt in self.positions.debts.iter() {
                        let debt_params = self
                            .denoms_data
                            .params
                            .get(&debt.denom)
                            .ok_or(MissingParams(debt.denom.clone()))?;
                        let debt_hls = debt_params
                            .credit_manager
                            .hls
                            .as_ref()
                            .ok_or(MissingHLSParams(debt.denom.clone()))?;

                        // collect all the correlations of the debts
                        correlations.extend(&debt_hls.correlations);
                    }

                    // If the collateral is not correlated with any of the debts, skip it.
                    // It doesn't contribute to the collateral value.
                    if !correlations.contains(&&HlsAssetType::Coin {
                        denom: coin.denom.clone(),
                    }) {
                        return Ok(None);
                    }
                } else if asset_params.credit_manager.hls.is_none() {
                    // Only collateral with hls params can be used in an HLS account and can contribute to the collateral value
                    return Ok(None);
                }
            }
            AccountKind::Default => {}
            AccountKind::FundManager {
                ..
            } => {}
        }

        Ok(Some(asset_params))
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
                    AccountKind::FundManager {
                        ..
                    } => *max_loan_to_value,
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
                AccountKind::FundManager {
                    ..
                } => *liquidation_threshold,
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

    fn get_coin_max_ltv(&self, denom: &str) -> HealthResult<Decimal> {
        let params = self.denoms_data.params.get(denom).ok_or(MissingParams(denom.to_string()))?;

        // If the coin has been de-listed, drop MaxLTV to zero
        if !params.credit_manager.whitelisted {
            return Ok(Decimal::zero());
        }

        match self.kind {
            AccountKind::Default => Ok(params.max_loan_to_value),
            AccountKind::FundManager {
                ..
            } => Ok(params.max_loan_to_value),
            AccountKind::HighLeveredStrategy => Ok(params
                .credit_manager
                .hls
                .as_ref()
                .ok_or(MissingHLSParams(denom.to_string()))?
                .max_loan_to_value),
        }
    }

    fn get_coin_from_deposits_and_lends(&self, denom: &str) -> HealthResult<Coin> {
        let deposited_coin = self.positions.deposits.iter().find(|c| c.denom == denom);
        let deposited_amount = deposited_coin.unwrap_or(&Coin::default()).amount;

        let lent_coin = self.positions.lends.iter().find(|c| c.denom == denom);
        let lent_amount = lent_coin.unwrap_or(&Coin::default()).amount;

        Ok(Coin {
            denom: denom.to_string(),
            amount: deposited_amount.checked_add(lent_amount)?,
        })
    }
}
