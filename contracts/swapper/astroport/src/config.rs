use cosmwasm_schema::cw_serde;
use mars_swapper_base::Config;

#[cw_serde]
pub struct AstroportConfig {
    /// The astroport router contract address
    pub router: String,
    /// The astroport factory contract address
    pub factory: String,
    /// The mars wasm oracle contract address
    pub oracle: String,
}

impl Config for AstroportConfig {}
