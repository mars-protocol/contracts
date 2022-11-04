use cosmwasm_schema::write_api;
use cw721_base::msg::InstantiateMsg;
use mars_account_nft::msg::{ExecuteMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
    }
}
