use cosmwasm_std::{
    to_binary, Addr, Api, BalanceResponse, BankQuery, Coin, CosmosMsg, Decimal, QuerierWrapper,
    QueryRequest, StdResult, Uint128, WasmMsg, WasmQuery,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::adapters::Oracle;
use crate::error::ContractResult;
use crate::extensions::Stringify;
use crate::msg::vault::{ExecuteMsg, QueryMsg, VaultInfo};
use crate::Shares;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub struct VaultPosition {
    pub unlocked: Uint128,
    pub locked: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct VaultBase<T>(T);

impl<T> VaultBase<T> {
    pub fn new(address: T) -> VaultBase<T> {
        VaultBase(address)
    }

    pub fn address(&self) -> &T {
        &self.0
    }
}

pub type VaultUnchecked = VaultBase<String>;
pub type Vault = VaultBase<Addr>;

impl From<&Vault> for VaultUnchecked {
    fn from(vault: &Vault) -> Self {
        Self(vault.address().to_string())
    }
}

impl VaultUnchecked {
    pub fn check(&self, api: &dyn Api) -> StdResult<Vault> {
        Ok(VaultBase(api.addr_validate(&self.0)?))
    }
}

impl From<Vault> for VaultUnchecked {
    fn from(v: Vault) -> Self {
        Self(v.0.to_string())
    }
}

impl Stringify for Vec<VaultUnchecked> {
    fn to_string(&self) -> String {
        self.iter()
            .map(|v| v.address().clone())
            .collect::<Vec<String>>()
            .join(", ")
    }
}

impl Vault {
    pub fn deposit_msg(&self, funds: &[Coin]) -> StdResult<CosmosMsg> {
        let deposit_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.address().to_string(),
            funds: funds.to_vec(),
            msg: to_binary(&ExecuteMsg::Deposit {})?,
        });
        Ok(deposit_msg)
    }

    pub fn query_vault_info(&self, querier: &QuerierWrapper) -> StdResult<VaultInfo> {
        querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: self.0.to_string(),
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
        let assets = self.query_redeem_preview(querier, balance)?;
        oracle.query_total_value(querier, &assets)
    }

    pub fn query_redeem_preview(
        &self,
        querier: &QuerierWrapper,
        shares: Shares,
    ) -> StdResult<Vec<Coin>> {
        let response: Vec<Coin> = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: self.0.to_string(),
            msg: to_binary(&QueryMsg::PreviewRedeem { shares })?,
        }))?;
        Ok(response)
    }
}
