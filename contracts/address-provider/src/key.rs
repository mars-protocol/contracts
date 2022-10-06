use std::convert::TryFrom;
use std::str::FromStr;

use cosmwasm_std::{StdError, StdResult};
use cw_storage_plus::{Key, KeyDeserialize, Prefixer, PrimaryKey};

use mars_outpost::address_provider::{MarsContract, MarsGov};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarsAddressKey(pub Vec<u8>);

impl From<MarsContract> for MarsAddressKey {
    fn from(contract: MarsContract) -> Self {
        Self(contract.to_string().into_bytes())
    }
}

impl From<MarsGov> for MarsAddressKey {
    fn from(gov: MarsGov) -> Self {
        Self(gov.to_string().into_bytes())
    }
}

impl TryFrom<MarsAddressKey> for MarsContract {
    type Error = StdError;

    fn try_from(key: MarsAddressKey) -> Result<Self, Self::Error> {
        let s = String::from_utf8(key.0)?;
        MarsContract::from_str(&s)
    }
}

impl TryFrom<MarsAddressKey> for MarsGov {
    type Error = StdError;

    fn try_from(key: MarsAddressKey) -> Result<Self, Self::Error> {
        let s = String::from_utf8(key.0)?;
        MarsGov::from_str(&s)
    }
}

impl<'a> PrimaryKey<'a> for MarsAddressKey {
    type Prefix = ();
    type SubPrefix = ();
    type Suffix = Self;
    type SuperSuffix = Self;

    fn key(&self) -> Vec<Key> {
        vec![Key::Ref(&self.0)]
    }
}

impl<'a> Prefixer<'a> for MarsAddressKey {
    fn prefix(&self) -> Vec<Key> {
        vec![Key::Ref(&self.0)]
    }
}

impl KeyDeserialize for MarsAddressKey {
    type Output = Self;

    #[inline(always)]
    fn from_vec(value: Vec<u8>) -> StdResult<Self::Output> {
        Ok(Self(value))
    }
}
