use cosmwasm_schema::write_api;
use mars_outpost::red_bank::{ExecuteMsg, QueryMsg};
use mock_red_bank::msg::InstantiateMsg;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
    }
}
