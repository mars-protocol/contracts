use cosmwasm_schema::write_api;
use mars_swapper_astroport::{config::AstroportConfig, route::AstroportRoute};
use mars_types::swapper::{ExecuteMsg, InstantiateMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg<AstroportRoute, AstroportConfig>,
        query: QueryMsg,
    }
}
