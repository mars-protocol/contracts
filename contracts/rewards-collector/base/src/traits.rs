use std::fmt::{Debug, Display};

use cosmwasm_std::{CosmosMsg, CustomMsg, CustomQuery, QuerierWrapper, Uint128};
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Serialize};

use crate::ContractResult;

pub trait Route<M, Q>:
    Serialize + DeserializeOwned + Clone + Debug + Display + PartialEq + JsonSchema
where
    M: CustomMsg,
    Q: CustomQuery,
{
    /// Determine whether the route is valid, given a pair of input and output denoms
    fn validate(
        &self,
        querier: &QuerierWrapper<Q>,
        denom_in: &str,
        denom_out: &str,
    ) -> ContractResult<()>;

    /// Build a message for executing the trade, given an input denom and amount
    fn build_swap_msg(
        &self,
        querier: &QuerierWrapper<Q>,
        denom_in: &str,
        amount: Uint128,
    ) -> ContractResult<CosmosMsg<M>>;
}
