use cosmwasm_std::Empty;
use mars_outpost::revenue_collector;

use crate::OsmosisRoute;

pub type ExecuteMsg = revenue_collector::ExecuteMsg<OsmosisRoute, Empty>;
pub type RouteResponse = revenue_collector::RouteResponse<OsmosisRoute>;
pub type RoutesResponse = revenue_collector::RoutesResponse<OsmosisRoute>;
