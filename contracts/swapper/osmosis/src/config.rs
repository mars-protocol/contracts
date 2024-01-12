use cosmwasm_schema::cw_serde;
use mars_swapper_base::Config;

#[cw_serde]
pub struct OsmosisConfig {}

impl Config for OsmosisConfig {}
