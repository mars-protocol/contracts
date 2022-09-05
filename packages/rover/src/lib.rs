use cosmwasm_std::{Addr, Uint128};

pub mod adapters;
pub mod coins;
pub mod error;
pub mod extensions;
pub mod msg;

pub type Denom<'a> = &'a str;
pub type NftTokenId<'a> = &'a str;
pub type Shares = Uint128;
pub type VaultAddr = Addr;
