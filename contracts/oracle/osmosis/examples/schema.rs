use cosmwasm_schema::write_api;
use mars_oracle::{ExecuteMsg, InstantiateMsg, QueryMsg};
use mars_oracle_osmosis::OsmosisPriceSourceUnchecked;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg<OsmosisPriceSourceUnchecked>,
        query: QueryMsg,
    }
}
