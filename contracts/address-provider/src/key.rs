use std::{convert::TryFrom, str::FromStr};

use cosmwasm_std::{StdError, StdResult};
use cw_storage_plus::{Key, KeyDeserialize, Prefixer, PrimaryKey};
use mars_types::address_provider::MarsAddressType;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarsAddressTypeKey(pub Vec<u8>);

impl From<MarsAddressType> for MarsAddressTypeKey {
    fn from(address_type: MarsAddressType) -> Self {
        Self(address_type.to_string().into_bytes())
    }
}

impl TryFrom<MarsAddressTypeKey> for MarsAddressType {
    type Error = StdError;

    fn try_from(key: MarsAddressTypeKey) -> Result<Self, Self::Error> {
        let s = String::from_utf8(key.0)?;
        MarsAddressType::from_str(&s)
    }
}

impl<'a> PrimaryKey<'a> for MarsAddressTypeKey {
    type Prefix = ();
    type SubPrefix = ();
    type Suffix = Self;
    type SuperSuffix = Self;

    fn key(&self) -> Vec<Key> {
        vec![Key::Ref(&self.0)]
    }
}

impl<'a> Prefixer<'a> for MarsAddressTypeKey {
    fn prefix(&self) -> Vec<Key> {
        vec![Key::Ref(&self.0)]
    }
}

impl KeyDeserialize for MarsAddressTypeKey {
    type Output = Self;

    #[inline(always)]
    fn from_vec(value: Vec<u8>) -> StdResult<Self::Output> {
        Ok(Self(value))
    }
}
