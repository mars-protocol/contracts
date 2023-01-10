use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Coin, QuerierWrapper, StdResult, Uint128};

use mars_math::FractionMath;
use mars_outpost::oracle::{PriceResponse, QueryMsg};

use crate::error::ContractResult;

#[cw_serde]
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

    pub fn query_value(&self, querier: &QuerierWrapper, coin: &Coin) -> ContractResult<Uint128> {
        self.query_total_value(querier, &[coin.clone()])
    }

    pub fn query_total_value(
        &self,
        querier: &QuerierWrapper,
        coins: &[Coin],
    ) -> ContractResult<Uint128> {
        Ok(coins
            .iter()
            .map(|coin| {
                let res = self.query_price(querier, &coin.denom)?;
                Ok(coin.amount.checked_mul_floor(res.price)?)
            })
            .collect::<ContractResult<Vec<_>>>()?
            .iter()
            .sum())
    }
}
