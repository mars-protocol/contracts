use cosmwasm_schema::write_api;
use mock_vault::msg::InstantiateMsg;
use rover::msg::vault::{ExecuteMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
    }
}
