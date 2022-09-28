use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, Addr, Api, BalanceResponse, BankQuery, Coin, CosmosMsg, Decimal, OverflowError,
    QuerierWrapper, QueryRequest, StdResult, Uint128, WasmMsg, WasmQuery,
};

use crate::adapters::Oracle;
use crate::error::ContractResult;
use crate::msg::vault::{ExecuteMsg, QueryMsg, VaultInfo};
use crate::traits::Stringify;

#[cw_serde]
#[derive(Default)]
pub struct VaultPositionState {
    pub unlocked: Uint128,
    pub locked: Uint128,
}

impl VaultPositionState {
    pub fn total(&self) -> Result<Uint128, OverflowError> {
        self.locked.checked_add(self.unlocked)
    }
}

#[cw_serde]
pub struct VaultPosition {
    pub vault: Vault,
    pub state: VaultPositionState,
}

#[cw_serde]
pub struct VaultBase<T> {
    pub address: T,
}

impl<T> VaultBase<T> {
    pub fn new(address: T) -> Self {
        Self { address }
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
        self.iter()
            .map(|v| v.address.clone())
            .collect::<Vec<String>>()
            .join(", ")
    }
}

impl Vault {
    pub fn deposit_msg(&self, funds: &[Coin]) -> StdResult<CosmosMsg> {
        let deposit_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.address.to_string(),
            funds: funds.to_vec(),
            msg: to_binary(&ExecuteMsg::Deposit {})?,
        });
        Ok(deposit_msg)
    }

    pub fn withdraw_msg(
        &self,
        querier: &QuerierWrapper,
        amount: Uint128,
        force: bool,
    ) -> StdResult<CosmosMsg> {
        let vault_info = self.query_vault_info(querier)?;
        let withdraw_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.address.to_string(),
            funds: vec![Coin {
                denom: vault_info.token_denom,
                amount,
            }],
            msg: to_binary(
                &(if force {
                    ExecuteMsg::ForceWithdraw {}
                } else {
                    ExecuteMsg::Withdraw {}
                }),
            )?,
        });
        Ok(withdraw_msg)
    }

    pub fn query_vault_info(&self, querier: &QuerierWrapper) -> StdResult<VaultInfo> {
        querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: self.address.to_string(),
            msg: to_binary(&QueryMsg::Info {})?,
        }))
    }

    pub fn query_balance(&self, querier: &QuerierWrapper, addr: &Addr) -> StdResult<Uint128> {
        let vault_info = self.query_vault_info(querier)?;
        let res: BalanceResponse = querier.query(&QueryRequest::Bank(BankQuery::Balance {
            address: addr.to_string(),
            denom: vault_info.token_denom,
        }))?;
        Ok(res.amount.amount)
    }

    pub fn query_total_value(
        &self,
        querier: &QuerierWrapper,
        oracle: &Oracle,
        addr: &Addr,
    ) -> ContractResult<Decimal> {
        let balance = self.query_balance(querier, addr)?;
        let assets = self.query_preview_redeem(querier, balance)?;
        oracle.query_total_value(querier, &assets)
    }

    pub fn query_preview_redeem(
        &self,
        querier: &QuerierWrapper,
        amount: Uint128,
    ) -> StdResult<Vec<Coin>> {
        querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: self.address.to_string(),
            msg: to_binary(&QueryMsg::PreviewRedeem { amount })?,
        }))
    }

    pub fn query_total_vault_coins_issued(&self, querier: &QuerierWrapper) -> StdResult<Uint128> {
        querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: self.address.to_string(),
            msg: to_binary(&QueryMsg::TotalVaultCoinsIssued {})?,
        }))
    }
}
