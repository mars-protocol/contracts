use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub struct NftConfigBase<T> {
    pub max_value_for_burn: Uint128,
    pub proposed_new_minter: Option<T>,
    pub health_contract_addr: Option<T>,
}

pub type NftConfig = NftConfigBase<Addr>;
pub type UncheckedNftConfig = NftConfigBase<String>;

impl From<NftConfig> for UncheckedNftConfig {
    fn from(config: NftConfig) -> Self {
        Self {
            max_value_for_burn: config.max_value_for_burn,
            proposed_new_minter: config.proposed_new_minter.map(Into::into),
            health_contract_addr: config.health_contract_addr.map(Into::into),
        }
    }
}

#[cw_serde]
pub struct NftConfigUpdates {
    pub max_value_for_burn: Option<Uint128>,
    pub proposed_new_minter: Option<String>,
    pub health_contract_addr: Option<String>,
}
