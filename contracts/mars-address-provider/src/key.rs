use std::convert::TryFrom;
use std::str::FromStr;

use cosmwasm_std::{StdError, StdResult};
use cw_storage_plus::{Key, KeyDeserialize, Prefixer, PrimaryKey};

use mars_outpost::address_provider::MarsContract;

#[derive(Clone, Debug, PartialEq)]
pub struct MarsContractKey(pub Vec<u8>);

impl From<MarsContract> for MarsContractKey {
    fn from(contract: MarsContract) -> Self {
        Self(contract.to_string().into_bytes())
    }
}

impl TryFrom<MarsContractKey> for MarsContract {
    type Error = StdError;

    fn try_from(key: MarsContractKey) -> Result<Self, Self::Error> {
        let s = String::from_utf8(key.0)?;
        MarsContract::from_str(&s)
    }
}

impl<'a> PrimaryKey<'a> for MarsContractKey {
    type Prefix = ();
    type SubPrefix = ();
    type Suffix = Self;
    type SuperSuffix = Self;

    fn key(&self) -> Vec<Key> {
        vec![Key::Ref(&self.0)]
    }
}

impl<'a> Prefixer<'a> for MarsContractKey {
    fn prefix(&self) -> Vec<Key> {
        vec![Key::Ref(&self.0)]
    }
}

impl KeyDeserialize for MarsContractKey {
    type Output = Self;

    #[inline(always)]
    fn from_vec(value: Vec<u8>) -> StdResult<Self::Output> {
        Ok(Self(value))
    }
}
