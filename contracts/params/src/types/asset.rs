use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Decimal, Uint128};
use mars_utils::helpers::{decimal_param_le_one, validate_native_denom};

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

#[cw_serde]
pub struct AssetParamsBase<T> {
    pub denom: String,
    pub credit_manager: CmSettings<T>,
    pub red_bank: RedBankSettings,
    pub max_loan_to_value: Decimal,
    pub liquidation_threshold: Decimal,
    pub liquidation_bonus: Decimal,
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
        }
    }
}

impl AssetParamsUnchecked {
    pub fn check(&self, api: &dyn Api) -> ContractResult<AssetParams> {
        validate_native_denom(&self.denom)?;

        decimal_param_le_one(self.max_loan_to_value, "max_loan_to_value")?;
        decimal_param_le_one(self.liquidation_threshold, "liquidation_threshold")?;
        assert_lqt_gt_max_ltv(self.max_loan_to_value, self.liquidation_threshold)?;

        decimal_param_le_one(self.liquidation_bonus, "liquidation_bonus")?;

        if let Some(hls) = self.credit_manager.hls.as_ref() {
            decimal_param_le_one(hls.max_loan_to_value, "hls_max_loan_to_value")?;
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
            liquidation_bonus: self.liquidation_bonus,
        })
    }
}
