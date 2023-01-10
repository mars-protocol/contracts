use std::hash::Hash;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, Addr, Api, BalanceResponse, BankQuery, Coin, CosmosMsg, QuerierWrapper,
    QueryRequest, StdError, StdResult, SubMsg, Uint128, WasmMsg, WasmQuery,
};
use cosmwasm_vault_standard::{
    extensions::{
        force_unlock::ForceUnlockExecuteMsg::{ForceRedeem, ForceWithdrawUnlocking},
        lockup::{
            LockupExecuteMsg::{Unlock, WithdrawUnlocked},
            LockupQueryMsg,
            LockupQueryMsg::LockupDuration,
            UnlockingPosition,
        },
    },
    msg::{ExtensionExecuteMsg, ExtensionQueryMsg, VaultStandardExecuteMsg, VaultStandardQueryMsg},
    VaultInfoResponse,
};
use cw_utils::Duration;
use mars_math::FractionMath;

use crate::{adapters::oracle::Oracle, traits::Stringify};

pub const VAULT_REQUEST_REPLY_ID: u64 = 10_001;

pub type ExecuteMsg = VaultStandardExecuteMsg<ExtensionExecuteMsg>;
pub type QueryMsg = VaultStandardQueryMsg<ExtensionQueryMsg>;

#[cw_serde]
#[derive(Eq, Hash)]
pub struct VaultBase<T> {
    pub address: T,
}

impl<T> VaultBase<T> {
    pub fn new(address: T) -> Self {
        Self {
            address,
        }
    }
}

pub type VaultUnchecked = VaultBase<String>;
pub type Vault = VaultBase<Addr>;

impl From<&Vault> for VaultUnchecked {
    fn from(vault: &Vault) -> Self {
        Self {
            address: vault.address.to_string(),
        }
    }
}

impl VaultUnchecked {
    pub fn check(&self, api: &dyn Api) -> StdResult<Vault> {
        Ok(VaultBase::new(api.addr_validate(&self.address)?))
    }
}

impl From<Vault> for VaultUnchecked {
    fn from(v: Vault) -> Self {
        Self {
            address: v.address.to_string(),
        }
    }
}

impl Stringify for Vec<VaultUnchecked> {
    fn to_string(&self) -> String {
        self.iter().map(|v| v.address.clone()).collect::<Vec<String>>().join(", ")
    }
}

impl Vault {
    pub fn deposit_msg(&self, coin: &Coin) -> StdResult<CosmosMsg> {
        let deposit_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.address.to_string(),
            funds: vec![coin.clone()],
            msg: to_binary(&ExecuteMsg::Deposit {
                amount: coin.amount,
                recipient: None,
            })?,
        });
        Ok(deposit_msg)
    }

    pub fn withdraw_msg(&self, querier: &QuerierWrapper, amount: Uint128) -> StdResult<CosmosMsg> {
        let vault_info = self.query_info(querier)?;
        let withdraw_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.address.to_string(),
            funds: vec![Coin {
                denom: vault_info.vault_token,
                amount,
            }],
            msg: to_binary(&ExecuteMsg::Redeem {
                recipient: None,
                amount,
            })?,
        });
        Ok(withdraw_msg)
    }

    pub fn force_withdraw_locked_msg(
        &self,
        querier: &QuerierWrapper,
        amount: Uint128,
    ) -> StdResult<CosmosMsg> {
        let vault_info = self.query_info(querier)?;
        let withdraw_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.address.to_string(),
            funds: vec![Coin {
                denom: vault_info.vault_token,
                amount,
            }],
            msg: to_binary(&ExecuteMsg::VaultExtension(ExtensionExecuteMsg::ForceUnlock(
                ForceRedeem {
                    recipient: None,
                    amount,
                },
            )))?,
        });
        Ok(withdraw_msg)
    }

    pub fn force_withdraw_unlocking_msg(
        &self,
        lockup_id: u64,
        amount: Option<Uint128>,
    ) -> StdResult<CosmosMsg> {
        let withdraw_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.address.to_string(),
            funds: vec![],
            msg: to_binary(&ExecuteMsg::VaultExtension(ExtensionExecuteMsg::ForceUnlock(
                ForceWithdrawUnlocking {
                    lockup_id,
                    amount,
                    recipient: None,
                },
            )))?,
        });
        Ok(withdraw_msg)
    }

    pub fn request_unlock_msg(&self, coin: Coin) -> StdResult<SubMsg> {
        let request_msg = SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: self.address.to_string(),
                funds: vec![coin.clone()],
                msg: to_binary(&ExecuteMsg::VaultExtension(ExtensionExecuteMsg::Lockup(Unlock {
                    amount: coin.amount,
                })))?,
            }),
            VAULT_REQUEST_REPLY_ID,
        );
        Ok(request_msg)
    }

    pub fn withdraw_unlocked_msg(&self, lockup_id: u64) -> StdResult<CosmosMsg> {
        let withdraw_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.address.to_string(),
            funds: vec![],
            msg: to_binary(&ExecuteMsg::VaultExtension(ExtensionExecuteMsg::Lockup(
                WithdrawUnlocked {
                    recipient: None,
                    lockup_id,
                },
            )))?,
        });
        Ok(withdraw_msg)
    }

    pub fn query_info(&self, querier: &QuerierWrapper) -> StdResult<VaultInfoResponse> {
        querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: self.address.to_string(),
            msg: to_binary(&QueryMsg::Info {})?,
        }))
    }

    pub fn query_lockup_duration(&self, querier: &QuerierWrapper) -> StdResult<Duration> {
        querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: self.address.to_string(),
            msg: to_binary(&QueryMsg::VaultExtension(ExtensionQueryMsg::Lockup(
                LockupDuration {},
            )))?,
        }))
    }

    pub fn query_unlocking_position(
        &self,
        querier: &QuerierWrapper,
        lockup_id: u64,
    ) -> StdResult<UnlockingPosition> {
        querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: self.address.to_string(),
            msg: to_binary(&QueryMsg::VaultExtension(ExtensionQueryMsg::Lockup(
                LockupQueryMsg::UnlockingPosition {
                    lockup_id,
                },
            )))?,
        }))
    }

    pub fn query_balance(&self, querier: &QuerierWrapper, addr: &Addr) -> StdResult<Uint128> {
        let vault_info = self.query_info(querier)?;
        let res: BalanceResponse = querier.query(&QueryRequest::Bank(BankQuery::Balance {
            address: addr.to_string(),
            denom: vault_info.vault_token,
        }))?;
        Ok(res.amount.amount)
    }

    pub fn query_preview_redeem(
        &self,
        querier: &QuerierWrapper,
        amount: Uint128,
    ) -> StdResult<Uint128> {
        querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: self.address.to_string(),
            msg: to_binary(&QueryMsg::PreviewRedeem {
                amount,
            })?,
        }))
    }

    pub fn query_total_vault_coins_issued(&self, querier: &QuerierWrapper) -> StdResult<Uint128> {
        querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: self.address.to_string(),
            msg: to_binary(&QueryMsg::TotalVaultTokenSupply {})?,
        }))
    }

    pub fn query_value(
        &self,
        querier: &QuerierWrapper,
        oracle: &Oracle,
        amount: Uint128,
    ) -> StdResult<Uint128> {
        let total_supply = self.query_total_vault_coins_issued(querier)?;
        if total_supply.is_zero() {
            return Ok(Uint128::zero());
        };

        let total_underlying = self.query_preview_redeem(querier, total_supply)?;
        let amount_in_underlying = amount
            .checked_multiply_ratio(total_underlying, total_supply)
            .map_err(|_| StdError::generic_err("CheckedMultiplyRatioError"))?;
        let vault_info = self.query_info(querier)?;
        let price_res = oracle.query_price(querier, &vault_info.base_token)?;
        let amount_value = amount_in_underlying
            .checked_mul_floor(price_res.price)
            .map_err(|_| StdError::generic_err("CheckedMultiplyFractionError"))?;
        Ok(amount_value)
    }
}
