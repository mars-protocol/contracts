use osmo_bindings::OsmosisMsg;

use crate::SwapInstructions;

pub type ExecuteMsg =
    mars_outpost::protocol_rewards_collector::ExecuteMsg<SwapInstructions, OsmosisMsg>;
