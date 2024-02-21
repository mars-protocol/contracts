use cosmwasm_schema::cw_serde;
use cosmwasm_std::Api;
use mars_swapper_base::{Config, ContractResult};

#[cw_serde]
pub struct OsmosisConfig {}

impl Config for OsmosisConfig {
    fn validate(&self, _api: &dyn Api) -> ContractResult<()> {
        Ok(())
    }
}
