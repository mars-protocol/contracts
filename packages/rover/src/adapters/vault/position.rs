use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, QuerierWrapper, StdError, StdResult, Uint128};
use mars_math::FractionMath;

use crate::adapters::{
    oracle::Oracle,
    vault::{Vault, VaultPositionAmount},
};

#[cw_serde]
pub struct VaultUnlockingPosition {
    /// Unique identifier representing the unlocking position. Needed for `ExecuteMsg::WithdrawUnlocked {}` call.
    pub id: u64,
    /// Coins that are awaiting to be unlocked (underlying, not vault tokens)
    pub coin: Coin,
}

#[cw_serde]
pub struct VaultPosition {
    pub vault: Vault,
    pub amount: VaultPositionAmount,
}

#[cw_serde]
pub enum VaultPositionType {
    UNLOCKED,
    LOCKED,
    UNLOCKING,
}

#[cw_serde]
pub struct CoinValue {
    pub denom: String,
    pub amount: Uint128,
    pub value: Uint128,
}

#[cw_serde]
pub struct VaultPositionValue {
    /// value of locked or unlocked
    pub vault_coin: CoinValue,
    /// value of all unlocking positions
    pub base_coin: CoinValue,
}

impl VaultPositionValue {
    pub fn total_value(&self) -> StdResult<Uint128> {
        Ok(self.base_coin.value.checked_add(self.vault_coin.value)?)
    }
}

impl VaultPosition {
    pub fn query_values(
        &self,
        querier: &QuerierWrapper,
        oracle: &Oracle,
    ) -> StdResult<VaultPositionValue> {
        Ok(VaultPositionValue {
            vault_coin: self.vault_coin_value(querier, oracle)?,
            base_coin: self.base_coin_value(querier, oracle)?,
        })
    }

    fn vault_coin_value(&self, querier: &QuerierWrapper, oracle: &Oracle) -> StdResult<CoinValue> {
        let vault_info = self.vault.query_info(querier)?;

        let total_supply = self.vault.query_total_vault_coins_issued(querier)?;
        if total_supply.is_zero() {
            return Ok(CoinValue {
                denom: vault_info.vault_token,
                amount: Uint128::zero(),
                value: Uint128::zero(),
            });
        };

        let vault_coin_amount = self.amount.unlocked().checked_add(self.amount.locked())?;
        let amount_in_base_coin = self.vault.query_preview_redeem(querier, vault_coin_amount)?;
        let price_res = oracle.query_price(querier, &vault_info.base_token)?;
        let total_value = amount_in_base_coin
            .checked_mul_floor(price_res.price)
            .map_err(|_| StdError::generic_err("CheckedMultiplyFractionError"))?;
        Ok(CoinValue {
            denom: vault_info.vault_token,
            amount: vault_coin_amount,
            value: total_value,
        })
    }

    fn base_coin_value(&self, querier: &QuerierWrapper, oracle: &Oracle) -> StdResult<CoinValue> {
        let vault_info = self.vault.query_info(querier)?;
        let base_token_price = oracle.query_price(querier, &vault_info.base_token)?.price;

        let total_value = self.amount.unlocking().positions().iter().try_fold(
            Uint128::zero(),
            |acc, curr| -> StdResult<Uint128> {
                let value = curr
                    .coin
                    .amount
                    .checked_mul_floor(base_token_price)
                    .map_err(|_| StdError::generic_err("CheckedMultiplyFractionError"))?;
                Ok(acc.checked_add(value)?)
            },
        )?;

        Ok(CoinValue {
            denom: vault_info.base_token,
            amount: self.amount.unlocking().total(),
            value: total_value,
        })
    }
}
