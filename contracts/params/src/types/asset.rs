use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Decimal, Uint128};
use mars_utils::{
    error::ValidationError,
    helpers::{decimal_param_le_one, decimal_param_lt_one, validate_native_denom},
};

use crate::{
    error::ContractResult,
    execute::{assert_hls_lqt_gt_max_ltv, assert_lqt_gt_max_ltv},
    types::hls::HlsParamsBase,
};

#[cw_serde]
pub struct CmSettings<T> {
    pub whitelisted: bool,
    pub hls: Option<HlsParamsBase<T>>,
}

#[cw_serde]
pub struct RedBankSettings {
    pub deposit_enabled: bool,
    pub borrow_enabled: bool,
    pub deposit_cap: Uint128,
}

/// The LB will depend on the Health Factor and a couple other parameters as follows:
/// Liquidation Bonus = min(
///     b + (slope * (1 - HF)),
///     max(
///         min(CR - 1, max_lb),
///         min_lb
///     )
/// )
#[cw_serde]
pub struct LiquidationBonus {
    /// Marks the level at which the LB starts when HF drops marginally below 1.
    /// If set at 1%, at HF = 0.999 the LB will be 1%. If set at 0%, the LB starts increasing from 0% as the HF drops below 1.
    pub starting_lb: Decimal,
    /// Defines the slope at which the LB increases as the HF decreases.
    /// The higher the slope, the faster the LB increases as the HF decreases.
    pub slope: Decimal,
    /// Minimum LB that will be granted to liquidators even when the position is undercollateralized.
    pub min_lb: Decimal,
    /// Maximum LB that can be granted to a liquidator; in other words, the maxLB establishes a ceiling to the LB.
    /// This is a precautionary parameter to mitigate liquidated users being over-punished.
    pub max_lb: Decimal,
}

impl LiquidationBonus {
    pub fn validate(&self) -> Result<(), ValidationError> {
        assert_starting_lb_within_range(self.starting_lb)?;
        assert_lb_slope_within_range(self.slope)?;
        assert_min_lb_within_range(self.min_lb)?;
        assert_max_lb_within_range(self.max_lb)?;
        assert_max_lb_gt_min_lb(self.min_lb, self.max_lb)?;
        Ok(())
    }
}

fn assert_starting_lb_within_range(b: Decimal) -> Result<(), ValidationError> {
    if b > Decimal::percent(10) {
        return Err(ValidationError::InvalidParam {
            param_name: "starting_lb".to_string(),
            invalid_value: b.to_string(),
            predicate: "[0, 0.1]".to_string(),
        });
    }
    Ok(())
}

fn assert_lb_slope_within_range(slope: Decimal) -> Result<(), ValidationError> {
    if slope < Decimal::one() || slope > Decimal::from_ratio(5u8, 1u8) {
        return Err(ValidationError::InvalidParam {
            param_name: "slope".to_string(),
            invalid_value: slope.to_string(),
            predicate: "[1, 5]".to_string(),
        });
    }
    Ok(())
}

fn assert_min_lb_within_range(min_lb: Decimal) -> Result<(), ValidationError> {
    if min_lb > Decimal::percent(10) {
        return Err(ValidationError::InvalidParam {
            param_name: "min_lb".to_string(),
            invalid_value: min_lb.to_string(),
            predicate: "[0, 0.1]".to_string(),
        });
    }
    Ok(())
}

fn assert_max_lb_within_range(max_lb: Decimal) -> Result<(), ValidationError> {
    if max_lb < Decimal::percent(5) || max_lb > Decimal::percent(30) {
        return Err(ValidationError::InvalidParam {
            param_name: "max_lb".to_string(),
            invalid_value: max_lb.to_string(),
            predicate: "[0.05, 0.3]".to_string(),
        });
    }
    Ok(())
}

fn assert_max_lb_gt_min_lb(min_lb: Decimal, max_lb: Decimal) -> Result<(), ValidationError> {
    if min_lb > max_lb {
        return Err(ValidationError::InvalidParam {
            param_name: "max_lb".to_string(),
            invalid_value: max_lb.to_string(),
            predicate: format!("> {} (min LB)", min_lb),
        });
    }
    Ok(())
}

#[cw_serde]
pub struct AssetParamsBase<T> {
    pub denom: String,
    pub credit_manager: CmSettings<T>,
    pub red_bank: RedBankSettings,
    pub max_loan_to_value: Decimal,
    pub liquidation_threshold: Decimal,
    pub liquidation_bonus: LiquidationBonus,
    pub protocol_liquidation_fee: Decimal,
}

pub type AssetParams = AssetParamsBase<Addr>;
pub type AssetParamsUnchecked = AssetParamsBase<String>;

impl From<AssetParams> for AssetParamsUnchecked {
    fn from(p: AssetParams) -> Self {
        Self {
            denom: p.denom,
            credit_manager: CmSettings {
                whitelisted: p.credit_manager.whitelisted,
                hls: p.credit_manager.hls.map(Into::into),
            },
            red_bank: p.red_bank,
            max_loan_to_value: p.max_loan_to_value,
            liquidation_threshold: p.liquidation_threshold,
            liquidation_bonus: p.liquidation_bonus,
            protocol_liquidation_fee: p.protocol_liquidation_fee,
        }
    }
}

impl AssetParamsUnchecked {
    pub fn check(&self, api: &dyn Api) -> ContractResult<AssetParams> {
        validate_native_denom(&self.denom)?;

        decimal_param_lt_one(self.max_loan_to_value, "max_loan_to_value")?;
        decimal_param_le_one(self.liquidation_threshold, "liquidation_threshold")?;
        assert_lqt_gt_max_ltv(self.max_loan_to_value, self.liquidation_threshold)?;

        self.liquidation_bonus.validate()?;
        decimal_param_lt_one(self.protocol_liquidation_fee, "protocol_liquidation_fee")?;

        if let Some(hls) = self.credit_manager.hls.as_ref() {
            decimal_param_lt_one(hls.max_loan_to_value, "hls_max_loan_to_value")?;
            decimal_param_le_one(hls.liquidation_threshold, "hls_liquidation_threshold")?;
            assert_hls_lqt_gt_max_ltv(hls.max_loan_to_value, hls.liquidation_threshold)?;
        }

        let hls = self.credit_manager.hls.as_ref().map(|hls| hls.check(api)).transpose()?;

        Ok(AssetParams {
            denom: self.denom.clone(),
            credit_manager: CmSettings {
                whitelisted: self.credit_manager.whitelisted,
                hls,
            },
            red_bank: self.red_bank.clone(),
            max_loan_to_value: self.max_loan_to_value,
            liquidation_threshold: self.liquidation_threshold,
            liquidation_bonus: self.liquidation_bonus.clone(),
            protocol_liquidation_fee: self.protocol_liquidation_fee,
        })
    }
}
