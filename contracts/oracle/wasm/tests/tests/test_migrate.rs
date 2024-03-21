// DOESN'T WORK BECAUSE OF THE SAME CONTRACT VERSION

// use std::collections::HashMap;

// use cosmwasm_std::{to_json_binary, CosmosMsg, Decimal, Empty, WasmMsg};
// use cw_it::{
//     osmosis_std::types::cosmwasm::wasm::v1::MsgMigrateContractResponse, test_tube::Runner,
//     traits::CwItRunner,
// };
// use mars_oracle_wasm::contract::CONTRACT_NAME;
// use mars_testing::{
//     test_runner::get_test_runner,
//     wasm_oracle::{get_contracts, get_wasm_oracle_contract, WasmOracleTestRobot},
// };
// use mars_types::oracle::{MigrateMsg, V2Updates};

// DOESN'T WORK BECAUSE OF THE SAME CONTRACT VERSION
// #[test]
// fn test_migrate_wasm_oracle() {
//     let owned_runner = get_test_runner();
//     let runner = owned_runner.as_ref();
//     let admin = &runner.init_default_account().unwrap();
//     let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, None);

//     let contract = get_wasm_oracle_contract(&runner);
//     let contract_map = HashMap::from([(CONTRACT_NAME.to_string(), contract)]);
//     let code_ids = cw_it::helpers::upload_wasm_files(&runner, admin, contract_map).unwrap();
//     let new_code_id = code_ids[CONTRACT_NAME];

//     runner
//         .execute_cosmos_msgs::<MsgMigrateContractResponse>(
//             &[CosmosMsg::Wasm(WasmMsg::Migrate {
//                 contract_addr: robot.mars_oracle_contract_addr,
//                 new_code_id,
//                 msg: to_json_binary(&MigrateMsg::V1_2_1ToV1_3_0(V2Updates {
//                     max_confidence: Decimal::percent(2),
//                     max_deviation: Decimal::percent(4),
//                 }))
//                 .unwrap(),
//             })],
//             admin,
//         )
//         .unwrap();
// }
