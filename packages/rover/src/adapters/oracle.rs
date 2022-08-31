use cosmwasm_std::{Addr, Api, QuerierWrapper, StdResult};
use mars_outpost::oracle::PriceResponse;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use mock_oracle::msg::QueryMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct OracleBase<T>(T);

impl<T> OracleBase<T> {
    pub fn new(address: T) -> OracleBase<T> {
        OracleBase(address)
    }

    pub fn address(&self) -> &T {
        &self.0
    }
}

pub type OracleUnchecked = OracleBase<String>;
pub type Oracle = OracleBase<Addr>;

impl From<Oracle> for OracleUnchecked {
    fn from(oracle: Oracle) -> Self {
        Self(oracle.address().to_string())
    }
}

impl OracleUnchecked {
    pub fn check(&self, api: &dyn Api) -> StdResult<Oracle> {
        Ok(OracleBase::new(api.addr_validate(self.address())?))
    }
}

impl Oracle {
    pub fn query_price(&self, querier: &QuerierWrapper, denom: &str) -> StdResult<PriceResponse> {
        querier.query_wasm_smart(
            self.address().to_string(),
            &QueryMsg::Price {
                denom: denom.to_string(),
            },
        )
    }
}
