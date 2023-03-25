use cosmwasm_schema::write_api;
use mars_oracle::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use mars_oracle_osmosis::OsmosisPriceSource;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg<OsmosisPriceSource>,
        query: QueryMsg,
    }
}
