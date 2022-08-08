use mars_outpost::rewards_collector;

use osmo_bindings::OsmosisMsg;

use crate::OsmosisRoute;

pub type ExecuteMsg = rewards_collector::ExecuteMsg<OsmosisRoute, OsmosisMsg>;
pub type RouteResponse = rewards_collector::RouteResponse<OsmosisRoute>;
pub type RoutesResponse = rewards_collector::RoutesResponse<OsmosisRoute>;
