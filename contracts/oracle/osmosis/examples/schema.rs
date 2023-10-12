use cosmwasm_schema::write_api;
use mars_oracle_osmosis::OsmosisPriceSourceUnchecked;
use mars_types::oracle::{ExecuteMsg, InstantiateMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg<OsmosisPriceSourceUnchecked>,
        query: QueryMsg,
    }
}
