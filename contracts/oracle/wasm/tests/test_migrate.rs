mod helpers;
use std::collections::HashMap;

use cosmwasm_std::{to_binary, CosmosMsg, Empty, WasmMsg};
use cw_it::test_tube::{
    osmosis_std::types::cosmwasm::wasm::v1::MsgMigrateContractResponse, Runner,
};
pub use helpers::*;
use mars_oracle_wasm::contract::CONTRACT_NAME;

#[test]
fn test_migrate_wasm_oracle() {
    let runner = get_test_runner();
    let admin = &runner.init_accounts()[0];
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, None);

    let contract = get_wasm_oracle_contract(&runner);
    let contract_map = HashMap::from([(CONTRACT_NAME.to_string(), contract)]);
    let code_ids = cw_it::helpers::upload_wasm_files(&runner, admin, contract_map).unwrap();
    let new_code_id = code_ids[CONTRACT_NAME];

    runner
        .execute_cosmos_msgs::<MsgMigrateContractResponse>(
            &[CosmosMsg::Wasm(WasmMsg::Migrate {
                contract_addr: robot.mars_oracle_contract_addr,
                new_code_id,
                msg: to_binary(&Empty {}).unwrap(),
            })],
            admin,
        )
        .unwrap();
}
