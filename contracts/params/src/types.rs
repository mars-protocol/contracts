use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Coin, Decimal, Uint128};
use mars_utils::{
    error::ValidationError,
    helpers::{decimal_param_le_one, validate_native_denom},
};

use crate::{
    error::ContractResult,
    execute::{assert_hls_lqt_gt_max_ltv, assert_lqt_gt_max_ltv},
};

#[cw_serde]
pub struct HighLeverageStrategyParams {
    pub max_loan_to_value: Decimal,
    pub liquidation_threshold: Decimal,
}

#[cw_serde]
pub struct RoverSettings {
    pub whitelisted: bool,
    pub hls: HighLeverageStrategyParams,
}

#[cw_serde]
pub struct RedBankSettings {
    pub deposit_enabled: bool,
    pub borrow_enabled: bool,
    pub deposit_cap: Uint128,
}

#[cw_serde]
pub struct AssetParams {
    pub denom: String,
    pub rover: RoverSettings,
    pub red_bank: RedBankSettings,
    pub max_loan_to_value: Decimal,
    pub liquidation_threshold: Decimal,
    pub liquidation_bonus: Decimal,
}

impl AssetParams {
    pub fn validate(&self) -> Result<(), ValidationError> {
        validate_native_denom(&self.denom)?;

        decimal_param_le_one(self.max_loan_to_value, "max_loan_to_value")?;
        decimal_param_le_one(self.liquidation_threshold, "liquidation_threshold")?;
        assert_lqt_gt_max_ltv(self.max_loan_to_value, self.liquidation_threshold)?;

        decimal_param_le_one(self.liquidation_bonus, "liquidation_bonus")?;

        decimal_param_le_one(self.rover.hls.max_loan_to_value, "hls_max_loan_to_value")?;
        decimal_param_le_one(self.rover.hls.liquidation_threshold, "hls_liquidation_threshold")?;
        assert_hls_lqt_gt_max_ltv(
            self.rover.hls.max_loan_to_value,
            self.rover.hls.liquidation_threshold,
        )?;

        Ok(())
    }
}

#[cw_serde]
pub struct VaultConfigBase<T> {
    pub addr: T,
    pub deposit_cap: Coin,
    pub max_loan_to_value: Decimal,
    pub liquidation_threshold: Decimal,
    pub whitelisted: bool,
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
        }
    }
}

impl VaultConfigUnchecked {
    pub fn check(&self, api: &dyn Api) -> ContractResult<VaultConfig> {
        decimal_param_le_one(self.max_loan_to_value, "max_loan_to_value")?;
        decimal_param_le_one(self.liquidation_threshold, "liquidation_threshold")?;
        assert_lqt_gt_max_ltv(self.max_loan_to_value, self.liquidation_threshold)?;

        Ok(VaultConfig {
            addr: api.addr_validate(&self.addr)?,
            deposit_cap: self.deposit_cap.clone(),
            max_loan_to_value: self.max_loan_to_value,
            liquidation_threshold: self.liquidation_threshold,
            whitelisted: self.whitelisted,
        })
    }
}

#[cw_serde]
pub enum AssetParamsUpdate {
    AddOrUpdate {
        params: AssetParams,
    },
}

#[cw_serde]
pub enum VaultConfigUpdate {
    AddOrUpdate {
        config: VaultConfigUnchecked,
    },
    Remove {
        addr: String,
    },
}

#[cw_serde]
pub enum RoverEmergencyUpdate {
    SetZeroMaxLtvOnVault(String),
    SetZeroDepositCapOnVault(String),
    DisallowCoin(String),
}

#[cw_serde]
pub enum RedBankEmergencyUpdate {
    DisableBorrowing(String),
}

#[cw_serde]
pub enum EmergencyUpdate {
    Rover(RoverEmergencyUpdate),
    RedBank(RedBankEmergencyUpdate),
}
