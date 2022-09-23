use cosmwasm_schema::write_api;
use cosmwasm_std::Empty;
use mars_outpost::rewards_collector::{ExecuteMsg, InstantiateMsg, QueryMsg};
use mars_rewards_collector_osmosis::OsmosisRoute;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg<OsmosisRoute, Empty>,
        query: QueryMsg,
    }
}
