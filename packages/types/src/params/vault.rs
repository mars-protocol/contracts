use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Coin, Decimal};
use mars_utils::helpers::decimal_param_le_one;

use super::{
    assertions::{assert_hls_lqt_gt_max_ltv, assert_lqt_gt_max_ltv},
    hls::HlsParamsBase,
};
use crate::error::MarsError;

#[cw_serde]
pub struct VaultConfigBase<T> {
    pub addr: T,
    pub deposit_cap: Coin,
    pub max_loan_to_value: Decimal,
    pub liquidation_threshold: Decimal,
    pub whitelisted: bool,
    pub hls: Option<HlsParamsBase<T>>,
}

pub type VaultConfigUnchecked = VaultConfigBase<String>;
pub type VaultConfig = VaultConfigBase<Addr>;

impl From<VaultConfig> for VaultConfigUnchecked {
    fn from(v: VaultConfig) -> Self {
        VaultConfigUnchecked {
            addr: v.addr.to_string(),
            deposit_cap: v.deposit_cap,
            max_loan_to_value: v.max_loan_to_value,
            liquidation_threshold: v.liquidation_threshold,
            whitelisted: v.whitelisted,
            hls: v.hls.map(Into::into),
        }
    }
}

impl VaultConfigUnchecked {
    pub fn check(&self, api: &dyn Api) -> Result<VaultConfig, MarsError> {
        decimal_param_le_one(self.max_loan_to_value, "max_loan_to_value")?;
        decimal_param_le_one(self.liquidation_threshold, "liquidation_threshold")?;
        assert_lqt_gt_max_ltv(self.max_loan_to_value, self.liquidation_threshold)?;

        // High levered strategies
        if let Some(hls) = self.hls.as_ref() {
            decimal_param_le_one(hls.max_loan_to_value, "hls_max_loan_to_value")?;
            decimal_param_le_one(hls.liquidation_threshold, "hls_liquidation_threshold")?;
            assert_hls_lqt_gt_max_ltv(hls.max_loan_to_value, hls.liquidation_threshold)?;
        }

        Ok(VaultConfig {
            addr: api.addr_validate(&self.addr)?,
            deposit_cap: self.deposit_cap.clone(),
            max_loan_to_value: self.max_loan_to_value,
            liquidation_threshold: self.liquidation_threshold,
            whitelisted: self.whitelisted,
            hls: self.hls.as_ref().map(|hls| hls.check(api)).transpose()?,
        })
    }
}
