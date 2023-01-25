use cosmwasm_schema::write_api;
use mars_oracle_osmosis::OsmosisPriceSource;
use mars_red_bank_types::oracle::{ExecuteMsg, InstantiateMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg<OsmosisPriceSource>,
        query: QueryMsg,
    }
}
