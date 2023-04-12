use cosmwasm_schema::write_api;
use mars_oracle::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    WasmOracleCustomInitParams,
};
use mars_oracle_wasm::WasmPriceSourceUnchecked;

fn main() {
    write_api! {
        instantiate: InstantiateMsg<WasmOracleCustomInitParams>,
        execute: ExecuteMsg<WasmPriceSourceUnchecked>,
        query: QueryMsg,
    }
}
