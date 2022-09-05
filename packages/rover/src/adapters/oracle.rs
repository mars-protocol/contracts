use cosmwasm_std::{Addr, Api, Coin, Decimal, QuerierWrapper, StdResult};
use mars_outpost::oracle::PriceResponse;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::ops::Add;

use crate::error::ContractResult;
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

    pub fn query_total_value(
        &self,
        querier: &QuerierWrapper,
        coins: &[Coin],
    ) -> ContractResult<Decimal> {
        Ok(coins
            .iter()
            .map(|coin| {
                let res = self.query_price(querier, &coin.denom)?;
                let asset_amount_dec = Decimal::from_atomics(coin.amount, 0)?;
                Ok(res.price.checked_mul(asset_amount_dec)?)
            })
            .collect::<ContractResult<Vec<_>>>()?
            .iter()
            .fold(Decimal::zero(), |total_value, amount| {
                total_value.add(amount)
            }))
    }
}
