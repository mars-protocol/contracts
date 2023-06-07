use cosmwasm_schema::write_api;
use mars_red_bank_types::swapper::{ExecuteMsg, InstantiateMsg, QueryMsg};
use mars_swapper_astroport::route::AstroportRoute;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg<AstroportRoute>,
        query: QueryMsg,
    }
}
