use cosmwasm_schema::write_api;
use mars_rover::adapters::swap::{ExecuteMsg, InstantiateMsg, QueryMsg};
use mars_swapper_osmosis::route::OsmosisRoute;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg<OsmosisRoute>,
    }
}
