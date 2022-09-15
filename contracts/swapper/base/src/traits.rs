use std::fmt::{Debug, Display};

use cosmwasm_std::{
    Addr, Coin, CosmosMsg, CustomMsg, CustomQuery, Decimal, Deps, Env, QuerierWrapper,
};
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Serialize};

use rover::adapters::swap::EstimateExactInSwapResponse;

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
    fn build_exact_in_swap_msg(
        &self,
        querier: &QuerierWrapper<Q>,
        contract_addr: Addr,
        coin_in: &Coin,
        slippage: Decimal,
    ) -> ContractResult<CosmosMsg<M>>;

    /// Query to get the estimate result of a swap
    fn estimate_exact_in_swap(
        &self,
        deps: Deps<Q>,
        env: Env,
        coin_in: Coin,
    ) -> ContractResult<EstimateExactInSwapResponse>;
}
