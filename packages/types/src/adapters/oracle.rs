use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    Addr, Api, CheckedMultiplyFractionError, Coin, QuerierWrapper, StdError, StdResult, Uint128,
};

use crate::oracle::{ActionKind, PriceResponse, QueryMsg};

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
    pub fn query_price(
        &self,
        querier: &QuerierWrapper,
        denom: &str,
        pricing: ActionKind,
    ) -> StdResult<PriceResponse> {
        querier.query_wasm_smart(
            self.address().to_string(),
            &QueryMsg::Price {
                denom: denom.to_string(),
                kind: Some(pricing),
            },
        )
    }

    pub fn query_value(
        &self,
        querier: &QuerierWrapper,
        coin: &Coin,
        action: ActionKind,
    ) -> Result<Uint128, OracleError> {
        self.query_total_value(querier, &[coin.clone()], action)
    }

    pub fn query_total_value(
        &self,
        querier: &QuerierWrapper,
        coins: &[Coin],
        action: ActionKind,
    ) -> Result<Uint128, OracleError> {
        Ok(coins
            .iter()
            .map(|coin| {
                let res = self.query_price(querier, &coin.denom, action.clone())?;
                Ok(coin.amount.checked_mul_floor(res.price)?)
            })
            .collect::<Result<Vec<_>, OracleError>>()?
            .iter()
            .sum())
    }
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum OracleError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    CheckedMultiplyFraction(#[from] CheckedMultiplyFractionError),
}
