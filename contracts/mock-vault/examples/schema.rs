use cosmos_vault_standard::msg::{ExecuteMsg, QueryMsg};
use cosmwasm_schema::write_api;
use mars_mock_vault::msg::InstantiateMsg;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
    }
}
