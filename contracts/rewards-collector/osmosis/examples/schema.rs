use cosmwasm_schema::write_api;
use mars_rewards_collector_osmosis::OsmosisRoute;
use mars_types::rewards_collector::{ExecuteMsg, InstantiateMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg<OsmosisRoute>,
        query: QueryMsg,
    }
}
