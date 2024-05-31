use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Int128, Int256, StdError, StdResult, Uint128};
use cw_storage_plus::{Item, Map};
use mars_owner::Owner;
use mars_types::adapters::{account_nft::AccountNft, health::HealthContract, oracle::Oracle};

use crate::msg::{PerformanceFeeConfig, UnlockState};

#[cw_serde]
#[derive(Default)]
pub struct PerformanceFeeState {
    pub updated_at: u64,
    pub liquidity: Uint128,
    pub accumulated_pnl: Int128,
    pub accumulated_fee: Uint128,
}

impl PerformanceFeeState {
    pub fn calculate_applied_linear_interest_rate(
        &self,
        current_time: u64,
        config: &PerformanceFeeConfig,
    ) -> StdResult<Decimal> {
        let time_diff_in_sec = current_time - self.updated_at;
        let time_diff_in_hours = time_diff_in_sec / 3600;
        Ok(config
            .performance_fee_percentage
            .checked_mul(Decimal::from_ratio(time_diff_in_hours, 1u128))?)
    }

    pub fn update_fee_and_pnl(
        &mut self,
        current_time: u64,
        total_staked_base_tokens: Uint128,
        config: &PerformanceFeeConfig,
    ) -> StdResult<()> {
        if self.updated_at == u64::MAX {
            self.updated_at = current_time;
            self.accumulated_pnl = Int128::zero();
            self.accumulated_fee = Uint128::zero();
            Ok(())
        } else {
            let accumulated_pnl_i256 = Int256::from(self.accumulated_pnl)
                + (Int256::from(total_staked_base_tokens) - Int256::from(self.liquidity));
            let accumulated_pnl_i128: Int128 = accumulated_pnl_i256.try_into()?;

            let accumulated_fee = if accumulated_pnl_i128 > Int128::zero() {
                let rate = self.calculate_applied_linear_interest_rate(current_time, config)?;
                accumulated_pnl_i128.unsigned_abs() * rate
            } else {
                Uint128::zero()
            };

            self.accumulated_pnl = accumulated_pnl_i128;
            self.accumulated_fee = accumulated_fee;

            Ok(())
        }
    }

    pub fn update_by_deposit(
        &mut self,
        total_staked_base_tokens: Uint128,
        deposit_amt: Uint128,
    ) -> StdResult<()> {
        let updated_liquidity = total_staked_base_tokens + deposit_amt - self.accumulated_fee;
        self.liquidity = updated_liquidity;
        Ok(())
    }

    pub fn update_by_withdraw(
        &mut self,
        total_staked_base_tokens: Uint128,
        withdraw_amt: Uint128,
    ) -> StdResult<()> {
        let updated_liquidity = total_staked_base_tokens - withdraw_amt - self.accumulated_fee;
        self.liquidity = updated_liquidity;
        Ok(())
    }

    pub fn update_by_manager(
        &mut self,
        current_time: u64,
        total_staked_base_tokens: Uint128,
        config: &PerformanceFeeConfig,
    ) -> StdResult<()> {
        let time_diff = current_time - self.updated_at;
        if time_diff < config.performance_fee_interval {
            return Err(StdError::generic_err(
                "Cannot update by manager before fee max holding period",
            ));
        }

        if self.accumulated_fee.is_zero() {
            return Err(StdError::generic_err(
                "Cannot update by manager before user has accumulated fees",
            ));
        }

        let updated_liquidity = total_staked_base_tokens - self.accumulated_fee;

        self.updated_at = current_time;
        self.accumulated_pnl = Int128::zero();
        self.accumulated_fee = Uint128::zero();
        self.liquidity = updated_liquidity;

        Ok(())
    }
}

pub const OWNER: Owner = Owner::new("owner");

pub const CREDIT_MANAGER: Item<String> = Item::new("cm_addr");
pub const VAULT_ACC_ID: Item<String> = Item::new("vault_acc_id");

pub const ORACLE: Item<Oracle> = Item::new("oracle");
pub const HEALTH: Item<HealthContract> = Item::new("health");
pub const ACCOUNT_NFT: Item<AccountNft> = Item::new("account_nft");

pub const TITLE: Item<String> = Item::new("title");
pub const SUBTITLE: Item<String> = Item::new("subtitle");
pub const DESCRIPTION: Item<String> = Item::new("desc");

pub const COOLDOWN_PERIOD: Item<u64> = Item::new("cooldown_period");
pub const UNLOCKS: Map<String, Vec<UnlockState>> = Map::new("unlocks");

pub const PERFORMANCE_FEE_CONFIG: Item<PerformanceFeeConfig> = Item::new("performance_fee_config");
pub const PERFORMANCE_FEE_STATE: Item<PerformanceFeeState> = Item::new("performance_fee_state");
