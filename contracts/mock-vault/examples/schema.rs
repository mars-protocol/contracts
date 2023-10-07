use cosmwasm_schema::write_api;
use mars_mock_vault::msg::InstantiateMsg;
use mars_rover::adapters::vault::{ExecuteMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
    }
}
