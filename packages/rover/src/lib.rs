use cosmwasm_std::Uint128;

use crate::error::ContractError;

pub mod adapters;
pub mod coins;
pub mod error;
pub mod health;
pub mod msg;

pub type ContractResult<T> = Result<T, ContractError>;
pub type NftTokenId<'a> = &'a str;
pub type Denom<'a> = &'a str;
pub type Shares = Uint128;
