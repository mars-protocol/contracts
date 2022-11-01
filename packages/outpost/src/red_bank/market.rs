use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, StdResult, Uint128};

use crate::error::MarsError;
use crate::helpers::{decimal_param_le_one, decimal_param_lt_one};
use crate::red_bank::InterestRateModel;

#[cw_serde]
pub struct Market {
    /// Denom of the asset
    pub denom: String,

    /// Max base asset that can be borrowed per "base asset" collateral when using the asset as collateral
    pub max_loan_to_value: Decimal,
    /// Base asset amount in debt position per "base asset" of asset collateral that if surpassed makes the user's position liquidatable.
    pub liquidation_threshold: Decimal,
    /// Bonus amount of collateral liquidator get when repaying user's debt (Will get collateral
    /// from user in an amount equal to debt repayed + bonus)
    pub liquidation_bonus: Decimal,
    /// Portion of the borrow rate that is kept as protocol rewards
    pub reserve_factor: Decimal,

    /// model (params + internal state) that defines how interest rate behaves
    pub interest_rate_model: InterestRateModel,

    /// Borrow index (Used to compute borrow interest)
    pub borrow_index: Decimal,
    /// Liquidity index (Used to compute deposit interest)
    pub liquidity_index: Decimal,
    /// Rate charged to borrowers
    pub borrow_rate: Decimal,
    /// Rate paid to depositors
    pub liquidity_rate: Decimal,
    /// Timestamp (seconds) where indexes and where last updated
    pub indexes_last_updated: u64,

    /// Total collateral scaled for the market's currency
    pub collateral_total_scaled: Uint128,
    /// Total debt scaled for the market's currency
    pub debt_total_scaled: Uint128,

    /// If false cannot deposit
    pub deposit_enabled: bool,
    /// If false cannot borrow
    pub borrow_enabled: bool,
    /// Deposit Cap (defined in terms of the asset)
    pub deposit_cap: Uint128,
}

impl Default for Market {
    fn default() -> Self {
        Market {
            denom: "".to_string(),
            borrow_index: Decimal::one(),
            liquidity_index: Decimal::one(),
            borrow_rate: Decimal::zero(),
            liquidity_rate: Decimal::zero(),
            max_loan_to_value: Decimal::zero(),
            reserve_factor: Decimal::zero(),
            indexes_last_updated: 0,
            collateral_total_scaled: Uint128::zero(),
            debt_total_scaled: Uint128::zero(),
            liquidation_threshold: Decimal::one(),
            liquidation_bonus: Decimal::zero(),
            interest_rate_model: InterestRateModel::default(),
            deposit_enabled: true,
            borrow_enabled: true,
            // By default the cap should be unlimited (no cap)
            deposit_cap: Uint128::MAX,
        }
    }
}

impl Market {
    pub fn validate(&self) -> Result<(), MarsError> {
        decimal_param_lt_one(self.reserve_factor, "reserve_factor")?;
        decimal_param_le_one(self.max_loan_to_value, "max_loan_to_value")?;
        decimal_param_le_one(self.liquidation_threshold, "liquidation_threshold")?;
        decimal_param_le_one(self.liquidation_bonus, "liquidation_bonus")?;

        // liquidation_threshold should be greater than max_loan_to_value
        if self.liquidation_threshold <= self.max_loan_to_value {
            return Err(MarsError::InvalidParam {
                param_name: "liquidation_threshold".to_string(),
                invalid_value: self.liquidation_threshold.to_string(),
                predicate: format!("> {} (max LTV)", self.max_loan_to_value),
            });
        }

        self.interest_rate_model.validate()?;

        Ok(())
    }

    pub fn update_interest_rates(&mut self, current_utilization_rate: Decimal) -> StdResult<()> {
        self.borrow_rate = self
            .interest_rate_model
            .get_borrow_rate(current_utilization_rate)?;

        self.liquidity_rate = self.interest_rate_model.get_liquidity_rate(
            self.borrow_rate,
            current_utilization_rate,
            self.reserve_factor,
        )?;

        Ok(())
    }

    pub fn increase_collateral(&mut self, amount_scaled: Uint128) -> StdResult<()> {
        self.collateral_total_scaled = self.collateral_total_scaled.checked_add(amount_scaled)?;
        Ok(())
    }

    pub fn increase_debt(&mut self, amount_scaled: Uint128) -> StdResult<()> {
        self.debt_total_scaled = self.debt_total_scaled.checked_add(amount_scaled)?;
        Ok(())
    }

    pub fn decrease_collateral(&mut self, amount_scaled: Uint128) -> StdResult<()> {
        self.collateral_total_scaled = self.collateral_total_scaled.checked_sub(amount_scaled)?;
        Ok(())
    }

    pub fn decrease_debt(&mut self, amount_scaled: Uint128) -> StdResult<()> {
        self.debt_total_scaled = self.debt_total_scaled.checked_sub(amount_scaled)?;
        Ok(())
    }
}
