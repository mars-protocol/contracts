use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub struct ConfigBase<T> {
    pub max_value_for_burn: Uint128,
    pub proposed_new_minter: Option<T>,
}

pub type Config = ConfigBase<Addr>;
pub type UncheckedConfig = ConfigBase<String>;

impl From<Config> for UncheckedConfig {
    fn from(config: Config) -> Self {
        Self {
            max_value_for_burn: config.max_value_for_burn,
            proposed_new_minter: config.proposed_new_minter.map(Into::into),
        }
    }
}

#[cw_serde]
pub struct ConfigUpdates {
    pub max_value_for_burn: Option<Uint128>,
    pub proposed_new_minter: Option<String>,
}
