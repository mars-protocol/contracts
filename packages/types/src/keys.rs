use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, StdError, StdResult};
use cw_storage_plus::{Key, KeyDeserialize, Prefixer, PrimaryKey};

#[cw_serde]
pub struct UserId {
    pub addr: Addr,
    pub acc_id: String,
}

impl UserId {
    pub fn credit_manager(addr: Addr, acc_id: String) -> Self {
        Self {
            addr,
            acc_id,
        }
    }

    pub fn red_bank(addr: Addr) -> Self {
        Self {
            addr,
            acc_id: "".to_string(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct UserIdKey(pub Vec<u8>);

impl TryFrom<UserIdKey> for UserId {
    type Error = StdError;

    fn try_from(key: UserIdKey) -> Result<Self, Self::Error> {
        let user_id: Self = serde_json_wasm::from_slice(&key.0)
            .map_err(|_| StdError::generic_err("Failed to deserialize UserId from JSON string"))?;
        Ok(user_id)
    }
}

impl TryFrom<UserId> for UserIdKey {
    type Error = StdError;

    fn try_from(user_id: UserId) -> Result<Self, Self::Error> {
        let bytes = serde_json_wasm::to_vec(&user_id)
            .map_err(|_| StdError::generic_err("Failed to serialize UserId to JSON string"))?;
        Ok(Self(bytes))
    }
}

impl<'a> PrimaryKey<'a> for &UserIdKey {
    type Prefix = ();
    type SubPrefix = ();
    type Suffix = Self;
    type SuperSuffix = Self;

    fn key(&self) -> Vec<Key> {
        vec![Key::Ref(&self.0)]
    }
}

impl<'a> Prefixer<'a> for &UserIdKey {
    fn prefix(&self) -> Vec<Key> {
        vec![Key::Ref(&self.0)]
    }
}

impl KeyDeserialize for &UserIdKey {
    type Output = UserIdKey;

    #[inline(always)]
    fn from_vec(value: Vec<u8>) -> StdResult<Self::Output> {
        Ok(UserIdKey(value))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn from_user_to_user_key() {
        let user_before =
            UserId::credit_manager(Addr::unchecked("random_addr"), "1234".to_string());
        let user_key: UserIdKey = user_before.clone().try_into().unwrap();
        let user_after: UserId = user_key.try_into().unwrap();
        assert_eq!(user_before, user_after);

        let user_before = UserId::red_bank(Addr::unchecked("random_addr"));
        let user_key: UserIdKey = user_before.clone().try_into().unwrap();
        let user_after: UserId = user_key.try_into().unwrap();
        assert_eq!(user_before, user_after);
    }

    #[test]
    fn composite_keys() {
        let user = UserId::credit_manager(Addr::unchecked("random_addr"), "1234".to_string());
        let user_key: UserIdKey = user.try_into().unwrap();

        let k: (&UserIdKey, &str, &str) = (&user_key, "uosmo", "ujake");

        let path = k.key();
        assert_eq!(3, path.len());

        let user_key_bytes: &[u8] = &user_key.0;
        assert_eq!(path, vec![user_key_bytes, b"uosmo", b"ujake"]);

        // ensure prefix also works
        let dir = k.0.prefix();
        assert_eq!(1, dir.len());
        assert_eq!(dir, vec![user_key_bytes]);
    }

    #[test]
    fn nested_composite_keys() {
        let user = UserId::red_bank(Addr::unchecked("random_addr"));
        let user_key: UserIdKey = user.try_into().unwrap();

        let k: ((&UserIdKey, &str), &str) = ((&user_key, "uosmo"), "ujake");

        let path = k.key();
        assert_eq!(3, path.len());

        let user_key_bytes: &[u8] = &user_key.0;
        assert_eq!(path, vec![user_key_bytes, b"uosmo", b"ujake"]);

        // ensure prefix also works
        let dir = k.0.prefix();
        assert_eq!(2, dir.len());
        assert_eq!(dir, vec![user_key_bytes, b"uosmo"]);
    }
}
