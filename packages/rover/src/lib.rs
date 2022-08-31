use cosmwasm_std::Uint128;

pub mod adapters;
pub mod coins;
pub mod error;
pub mod msg;

pub type Denom<'a> = &'a str;
pub type NftTokenId<'a> = &'a str;
pub type Shares = Uint128;
