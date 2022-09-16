use account_nft::msg::{ExecuteMsg, QueryMsg};
use cosmwasm_schema::write_api;
use cw721_base::msg::InstantiateMsg;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
    }
}
