use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Decimal};

use crate::error::ContractError;
use crate::error::ContractError::InvalidVaultConfig;

#[cw_serde]
pub struct VaultConfig {
    pub deposit_cap: Coin,
    pub max_ltv: Decimal,
    pub liquidation_threshold: Decimal,
    pub whitelisted: bool,
}

impl VaultConfig {
    pub fn check(&self) -> Result<(), ContractError> {
        let max_ltv_too_big = self.max_ltv > Decimal::one();
        let lqt_too_big = self.liquidation_threshold > Decimal::one();
        let max_ltv_bigger_than_lqt = self.max_ltv > self.liquidation_threshold;

        if max_ltv_too_big || lqt_too_big || max_ltv_bigger_than_lqt {
            return Err(InvalidVaultConfig {});
        }
        Ok(())
    }
}
