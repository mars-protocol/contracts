use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use mars_utils::{error::ValidationError, helpers::decimal_param_le_one};

use crate::execute::{assert_hls_lqt_gte_max_ltv, assert_lqt_gte_max_ltv};

#[cw_serde]
pub struct RoverPermissions {
    pub whitelisted: bool,
}

#[cw_serde]
pub struct RedBankPermissions {
    pub deposit_enabled: bool,
    pub borrow_enabled: bool,
    pub deposit_cap: Uint128,
}

#[cw_serde]
pub struct AssetPermissions {
    pub rover: RoverPermissions,
    pub red_bank: RedBankPermissions,
}

#[cw_serde]
pub struct HighLeverageStrategyParams {
    pub max_loan_to_value: Decimal,
    pub liquidation_threshold: Decimal,
}

#[cw_serde]
pub struct AssetParams {
    pub permissions: AssetPermissions,
    pub hls: HighLeverageStrategyParams,
    pub max_loan_to_value: Decimal,
    pub liquidation_threshold: Decimal,
    pub liquidation_bonus: Decimal,
}

impl AssetParams {
    pub fn validate(&self) -> Result<(), ValidationError> {
        decimal_param_le_one(self.max_loan_to_value, "max_loan_to_value")?;
        decimal_param_le_one(self.liquidation_threshold, "liquidation_threshold")?;
        assert_lqt_gte_max_ltv(self.max_loan_to_value, self.liquidation_threshold)?;

        decimal_param_le_one(self.liquidation_bonus, "liquidation_bonus")?;

        decimal_param_le_one(self.hls.max_loan_to_value, "hls_max_loan_to_value")?;
        decimal_param_le_one(self.hls.liquidation_threshold, "hls_liquidation_threshold")?;
        assert_hls_lqt_gte_max_ltv(self.hls.max_loan_to_value, self.hls.liquidation_threshold)?;

        Ok(())
    }
}

#[cw_serde]
pub struct AssetParamsResponse {
    pub denom: String,
    pub params: AssetParams,
}

#[cw_serde]
pub struct VaultConfigResponse {
    pub addr: Addr,
    pub config: VaultConfig,
}

#[cw_serde]
pub struct VaultConfig {
    pub deposit_cap: Coin,
    pub max_loan_to_value: Decimal,
    pub liquidation_threshold: Decimal,
    pub whitelisted: bool,
}

impl VaultConfig {
    pub fn validate(&self) -> Result<(), ValidationError> {
        decimal_param_le_one(self.max_loan_to_value, "max_loan_to_value")?;
        decimal_param_le_one(self.liquidation_threshold, "liquidation_threshold")?;
        assert_lqt_gte_max_ltv(self.max_loan_to_value, self.liquidation_threshold)?;
        Ok(())
    }
}

#[cw_serde]
pub enum AssetParamsUpdate {
    AddOrUpdate {
        denom: String,
        params: AssetParams,
    },
}

#[cw_serde]
pub enum VaultConfigUpdate {
    AddOrUpdate {
        addr: String,
        config: VaultConfig,
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
