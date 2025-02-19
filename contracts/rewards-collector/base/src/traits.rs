use std::fmt::{Debug, Display};

use cosmwasm_std::{
    BankMsg, Coin, CosmosMsg, CustomMsg, CustomQuery, Decimal, Empty, Env, QuerierWrapper, Uint128,
};
use mars_types::rewards_collector::{Config, TransferType};
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Serialize};

use crate::{ContractError, ContractResult};

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

pub trait TransferMsg<M: CustomMsg> {
    fn transfer_msg(
        env: &Env,
        to_address: &str,
        amount: Coin,
        cfg: &Config,
        transfer_type: &TransferType,
    ) -> ContractResult<CosmosMsg<M>>;
}

impl TransferMsg<Empty> for Empty {
    fn transfer_msg(
        _: &Env,
        to_address: &str,
        amount: Coin,
        _: &Config,
        transfer_type: &TransferType,
    ) -> ContractResult<CosmosMsg<Empty>> {
        // By default, we only support bank transfers
        match transfer_type {
            TransferType::Bank => Ok(CosmosMsg::Bank(BankMsg::Send {
                to_address: to_address.to_string(),
                amount: vec![amount],
            })),
            TransferType::Ibc => Err(ContractError::UnsupportedTransferType {
                transfer_type: transfer_type.to_string(),
            }),
        }
    }
}
