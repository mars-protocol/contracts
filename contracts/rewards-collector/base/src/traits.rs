use std::fmt::{Debug, Display};

use cosmwasm_std::{
    Coin, CosmosMsg, CustomMsg, CustomQuery, Decimal, Empty, Env, IbcMsg, IbcTimeout,
    QuerierWrapper, Uint128,
};
use mars_types::rewards_collector::Config;
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
        env: &Env,
        querier: &QuerierWrapper<Q>,
        denom_in: &str,
        amount: Uint128,
        slippage_tolerance: Decimal,
    ) -> ContractResult<CosmosMsg<M>>;
}

pub trait IbcTransferMsg<M: CustomMsg> {
    fn ibc_transfer_msg(
        env: Env,
        to_address: String,
        amount: Coin,
        cfg: Config,
    ) -> ContractResult<CosmosMsg<M>>;
}

impl IbcTransferMsg<Empty> for Empty {
    fn ibc_transfer_msg(
        env: Env,
        to_address: String,
        amount: Coin,
        cfg: Config,
    ) -> ContractResult<CosmosMsg<Empty>> {
        Ok(CosmosMsg::Ibc(IbcMsg::Transfer {
            channel_id: cfg.channel_id,
            to_address,
            amount,
            timeout: IbcTimeout::with_timestamp(env.block.time.plus_seconds(cfg.timeout_seconds)),
        }))
    }
}
