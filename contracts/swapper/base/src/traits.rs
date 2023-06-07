use std::fmt::{Debug, Display};

use cosmwasm_std::{Coin, CosmosMsg, CustomMsg, CustomQuery, Decimal, Env, QuerierWrapper};
use mars_red_bank_types::swapper::EstimateExactInSwapResponse;
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
    fn build_exact_in_swap_msg(
        &self,
        querier: &QuerierWrapper<Q>,
        env: &Env,
        coin_in: &Coin,
        slippage: Decimal,
    ) -> ContractResult<CosmosMsg<M>>;

    /// Query to get the estimate result of a swap
    fn estimate_exact_in_swap(
        &self,
        querier: &QuerierWrapper<Q>,
        env: &Env,
        coin_in: &Coin,
    ) -> ContractResult<EstimateExactInSwapResponse>;
}
