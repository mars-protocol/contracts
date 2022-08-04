use cosmwasm_std::{
    to_binary, Addr, Api, Decimal, QuerierWrapper, QueryRequest, StdResult, WasmQuery,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use mock_oracle::msg::QueryMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OracleBase<T>(pub T);

pub type OracleUnchecked = OracleBase<String>;
pub type Oracle = OracleBase<Addr>;

impl From<Oracle> for OracleUnchecked {
    fn from(oracle: Oracle) -> Self {
        Self(oracle.0.to_string())
    }
}

impl OracleUnchecked {
    pub fn check(&self, api: &dyn Api) -> StdResult<Oracle> {
        Ok(OracleBase(api.addr_validate(&self.0)?))
    }
}

impl Oracle {
    pub fn query_price(&self, querier: &QuerierWrapper, denom: &str) -> StdResult<Decimal> {
        let response: Decimal = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: self.0.to_string(),
            msg: to_binary(&QueryMsg::AssetPrice {
                denom: denom.to_string(),
            })?,
        }))?;
        Ok(response)
    }
}
