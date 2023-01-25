use mars_red_bank_types::rewards_collector;

use crate::OsmosisRoute;

pub type ExecuteMsg = rewards_collector::ExecuteMsg<OsmosisRoute>;
pub type RouteResponse = rewards_collector::RouteResponse<OsmosisRoute>;
pub type RoutesResponse = rewards_collector::RoutesResponse<OsmosisRoute>;
