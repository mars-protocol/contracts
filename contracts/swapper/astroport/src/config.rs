use cosmwasm_schema::cw_serde;
use cosmwasm_std::Api;
use mars_swapper_base::{Config, ContractResult};

#[cw_serde]
pub struct AstroportConfig {
    /// The astroport router contract address
    pub router: String,
    /// The astroport factory contract address
    pub factory: String,
    /// The mars wasm oracle contract address
    pub oracle: String,
}

impl Config for AstroportConfig {
    fn validate(&self, api: &dyn Api) -> ContractResult<()> {
        api.addr_validate(&self.router)?;
        api.addr_validate(&self.factory)?;
        api.addr_validate(&self.oracle)?;

        Ok(())
    }
}
