use std::fmt::{Debug, Display};

use cosmwasm_std::{CustomQuery, Decimal, Deps, Env};
use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Serialize};

use crate::ContractResult;

pub trait PriceSourceUnchecked<P, C>:
    Serialize + DeserializeOwned + Clone + Debug + PartialEq + JsonSchema
where
    P: PriceSourceChecked<C>,
    C: CustomQuery,
{
    /// Validate whether the price source is valid for a given denom
    fn validate(
        self,
        deps: &Deps<C>,
        denom: &str,
        base_denom: &str,
        price_sources: &Map<&str, P>,
    ) -> ContractResult<P>;
}
pub trait PriceSourceChecked<C>:
    Serialize + DeserializeOwned + Clone + Debug + Display + PartialEq + JsonSchema
where
    C: CustomQuery,
{
    /// Query the price of an asset based on the given price source
    ///
    /// Notable arguments:
    ///
    /// - `denom`: The coin whose price is to be queried.
    ///
    /// - `base_denom`: The coin in which the price is to be denominated in.
    ///   For example, if `denom` is uatom and `base_denom` is uosmo, the
    ///   function should return how many uosmo is per one uatom.
    ///
    /// - `price_sources`: A map that stores the price source for each coin.
    ///   This is necessary because for some coins, in order to calculate its
    ///   price, the prices of other coins are needed. An example is DEX LP
    ///   tokens, for which we need the price of each token in the pool.
    fn query_price(
        &self,
        deps: &Deps<C>,
        env: &Env,
        denom: &str,
        base_denom: &str,
        price_sources: &Map<&str, Self>,
    ) -> ContractResult<Decimal>;
}
