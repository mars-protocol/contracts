use cosmwasm_std::Empty;
use mars_outpost::rewards_collector;

use crate::OsmosisRoute;

pub type ExecuteMsg = rewards_collector::ExecuteMsg<OsmosisRoute, Empty>;
pub type RouteResponse = rewards_collector::RouteResponse<OsmosisRoute>;
pub type RoutesResponse = rewards_collector::RoutesResponse<OsmosisRoute>;
