use osmo_bindings::OsmosisMsg;

use crate::OsmosisRoute;

pub type ExecuteMsg = mars_outpost::rewards_collector::ExecuteMsg<OsmosisRoute, OsmosisMsg>;
