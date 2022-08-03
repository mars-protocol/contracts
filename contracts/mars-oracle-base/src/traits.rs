use std::fmt::{Debug, Display};

use cosmwasm_std::{CustomQuery, Decimal, QuerierWrapper, StdResult};
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Serialize};

use crate::ContractResult;

pub trait PriceSource<C>:
    Serialize + DeserializeOwned + Clone + Debug + Display + PartialEq + JsonSchema
where
    C: CustomQuery,
{
    /// Validate whether the price source is valid for a given denom
    fn validate(&self, querier: &QuerierWrapper<C>, denom: impl Into<String>)
        -> ContractResult<()>;

    /// Query the price of an asset based on the given price source
    fn query_price(
        &self,
        querier: &QuerierWrapper<C>,
        denom: impl Into<String>,
    ) -> StdResult<Decimal>;
}
