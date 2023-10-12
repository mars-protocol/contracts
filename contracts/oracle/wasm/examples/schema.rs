use cosmwasm_schema::write_api;
use mars_oracle_wasm::WasmPriceSourceUnchecked;
use mars_types::oracle::{
    ExecuteMsg, InstantiateMsg, QueryMsg, WasmOracleCustomExecuteMsg, WasmOracleCustomInitParams,
};

fn main() {
    write_api! {
        instantiate: InstantiateMsg<WasmOracleCustomInitParams>,
        execute: ExecuteMsg<WasmPriceSourceUnchecked, WasmOracleCustomExecuteMsg>,
        query: QueryMsg,
    }
}
