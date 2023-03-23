use cosmwasm_schema::write_api;
use mars_swapper::msgs::{ExecuteMsg, InstantiateMsg, QueryMsg};
use mars_swapper_osmosis::route::OsmosisRoute;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg<OsmosisRoute>,
        query: QueryMsg,
    }
}
