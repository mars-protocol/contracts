use cosmwasm_schema::write_api;
use cosmwasm_std::Empty;
use mars_types::red_bank::{ExecuteMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: Empty,
        query: QueryMsg,
        execute: ExecuteMsg,
    }
}
