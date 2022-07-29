use osmo_bindings::OsmosisMsg;

use crate::SwapInstruction;

pub type ExecuteMsg =
    mars_outpost::protocol_rewards_collector::ExecuteMsg<SwapInstruction, OsmosisMsg>;
