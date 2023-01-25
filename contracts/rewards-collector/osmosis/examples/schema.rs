use cosmwasm_schema::write_api;
use mars_red_bank_types::rewards_collector::{ExecuteMsg, InstantiateMsg, QueryMsg};
use mars_rewards_collector_osmosis::OsmosisRoute;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg<OsmosisRoute>,
        query: QueryMsg,
    }
}
