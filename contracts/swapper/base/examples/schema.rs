use cosmwasm_schema::write_api;
use cosmwasm_std::Empty;
use mars_swapper::msgs::{ExecuteMsg, InstantiateMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg<Empty>,
    }
}
