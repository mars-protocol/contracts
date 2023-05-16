use cosmwasm_schema::write_api;
use mars_oracle::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    WasmOracleCustomExecuteMsg, WasmOracleCustomInitParams,
};
use mars_oracle_wasm::WasmPriceSourceUnchecked;

fn main() {
    write_api! {
        instantiate: InstantiateMsg<WasmOracleCustomInitParams>,
        execute: ExecuteMsg<WasmPriceSourceUnchecked, WasmOracleCustomExecuteMsg>,
        query: QueryMsg,
    }
}
