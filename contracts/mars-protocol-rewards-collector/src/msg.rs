use osmo_bindings::OsmosisMsg;

use crate::Route;

pub type ExecuteMsg = mars_outpost::protocol_rewards_collector::ExecuteMsg<Route, OsmosisMsg>;
