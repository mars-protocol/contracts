use osmo_bindings::{OsmosisMsg, Step};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::ContractResult;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SwapInstructions(Vec<Step>);

impl SwapInstructions {
    pub fn validate(&self) -> ContractResult<()> {
        // TODO
        Ok(())
    }

    pub fn steps(&self) -> &[Step] {
        &self.0
    }
}

pub type ExecuteMsg =
    mars_outpost::protocol_rewards_collector::ExecuteMsg<SwapInstructions, OsmosisMsg>;
