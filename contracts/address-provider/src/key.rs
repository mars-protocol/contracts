use std::convert::TryFrom;
use std::str::FromStr;

use cosmwasm_std::{StdError, StdResult};
use cw_storage_plus::{Key, KeyDeserialize, Prefixer, PrimaryKey};

use mars_outpost::address_provider::{MarsLocal, MarsRemote};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarsAddressKey(pub Vec<u8>);

impl From<MarsLocal> for MarsAddressKey {
    fn from(local: MarsLocal) -> Self {
        Self(local.to_string().into_bytes())
    }
}

impl From<MarsRemote> for MarsAddressKey {
    fn from(remote: MarsRemote) -> Self {
        Self(remote.to_string().into_bytes())
    }
}

impl TryFrom<MarsAddressKey> for MarsLocal {
    type Error = StdError;

    fn try_from(key: MarsAddressKey) -> Result<Self, Self::Error> {
        let s = String::from_utf8(key.0)?;
        MarsLocal::from_str(&s)
    }
}

impl TryFrom<MarsAddressKey> for MarsRemote {
    type Error = StdError;

    fn try_from(key: MarsAddressKey) -> Result<Self, Self::Error> {
        let s = String::from_utf8(key.0)?;
        MarsRemote::from_str(&s)
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
