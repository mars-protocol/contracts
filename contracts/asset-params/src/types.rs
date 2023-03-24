use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Decimal, Uint128};
use mars_red_bank_types::red_bank::InterestRateModel;
use mars_utils::error::ValidationError;
use mars_utils::helpers::{decimal_param_le_one, decimal_param_lt_one};

#[cw_serde]
pub struct AssetParams {
    pub max_loan_to_value: Decimal,
    pub liquidation_threshold: Decimal,
    pub liquidation_bonus: Decimal,
    pub rover_whitelisted: bool,
    pub red_bank_deposit_enabled: bool,
    pub red_bank_borrow_enabled: bool,
    pub red_bank_deposit_cap: Uint128,
    pub interest_rate_model: InterestRateModel,
    pub reserve_factor: Decimal,
    pub uncollateralized_loan_limit: Uint128,
}

impl AssetParams{
    pub fn validate(&self) -> Result<(), ValidationError> {
        decimal_param_lt_one(self.reserve_factor, "reserve_factor")?;
        decimal_param_le_one(self.max_loan_to_value, "max_loan_to_value")?;
        decimal_param_le_one(self.liquidation_threshold, "liquidation_threshold")?;
        decimal_param_le_one(self.liquidation_bonus, "liquidation_bonus")?;

        // liquidation_threshold should be greater than max_loan_to_value
        if self.liquidation_threshold <= self.max_loan_to_value {
            return Err(ValidationError::InvalidParam {
                param_name: "liquidation_threshold".to_string(),
                invalid_value: self.liquidation_threshold.to_string(),
                predicate: format!("> {} (max LTV)", self.max_loan_to_value),
            });
        }

        self.interest_rate_model.validate()?;

        Ok(())
    }
}

#[cw_serde]
pub struct VaultConfigs {
    pub deposit_cap: Coin,
    pub max_loan_to_value: Decimal,
    pub liquidation_threshold: Decimal,
    pub rover_whitelisted: bool,
}

#[cw_serde]
pub struct ConfigResponse {
    /// The contract's owner
    pub owner: Option<String>,
    /// The contract's proposed owner
    pub proposed_new_owner: Option<String>,
    /// The contract's emergency owner
    pub emergency_owner: Option<String>,
    /// Maximum percentage of outstanding debt that can be covered by a liquidator
    pub close_factor: Decimal,
}