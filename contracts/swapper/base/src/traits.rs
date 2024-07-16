use std::fmt::{Debug, Display};

use cosmwasm_std::{Api, Coin, CosmosMsg, CustomMsg, CustomQuery, Env, QuerierWrapper, Uint128};
use mars_types::swapper::{EstimateExactInSwapResponse, SwapperRoute};
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Serialize};

use crate::ContractResult;

pub trait Route<M, Q, C>:
    Serialize + DeserializeOwned + Clone + Debug + Display + PartialEq + JsonSchema
where
    M: CustomMsg,
    Q: CustomQuery,
    C: Config,
{
    fn from(route: SwapperRoute, config: Option<C>) -> ContractResult<Self>;

    /// Determine whether the route is valid, given a pair of input and output denoms
    fn validate(
        &self,
        querier: &QuerierWrapper<Q>,
        denom_in: &str,
        denom_out: &str,
    ) -> ContractResult<()>;

    /// Build a message for executing the trade, given an input denom and amount
    fn build_exact_in_swap_msg(
        &self,
        querier: &QuerierWrapper<Q>,
        env: &Env,
        coin_in: &Coin,
        min_receive: Uint128,
    ) -> ContractResult<CosmosMsg<M>>;

    /// Query to get the estimate result of a swap
    fn estimate_exact_in_swap(
        &self,
        querier: &QuerierWrapper<Q>,
        env: &Env,
        coin_in: &Coin,
    ) -> ContractResult<EstimateExactInSwapResponse>;
}

pub trait Config: Serialize + DeserializeOwned + Clone + Debug + PartialEq + JsonSchema {
    fn validate(&self, api: &dyn Api) -> ContractResult<()>;
}
