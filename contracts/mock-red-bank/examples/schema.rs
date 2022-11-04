use cosmwasm_schema::write_api;
use mars_mock_red_bank::msg::InstantiateMsg;
use mars_outpost::red_bank::{ExecuteMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
    }
}
