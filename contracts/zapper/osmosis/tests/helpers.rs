use std::{fmt::Display, str::FromStr};

use mars_zapper_base::InstantiateMsg;
use osmosis_testing::{
    cosmrs::proto::cosmos::bank::v1beta1::QueryBalanceRequest, Bank, OsmosisTestApp, RunnerError,
    SigningAccount, Wasm,
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");

pub fn wasm_file() -> String {
    let artifacts_dir =
        std::env::var("ARTIFACTS_DIR_PATH").unwrap_or_else(|_| "artifacts".to_string());
    let snaked_name = CONTRACT_NAME.replace('-', "_");
    format!("../../../{artifacts_dir}/{snaked_name}.wasm")
}

pub fn instantiate_contract(wasm: &Wasm<OsmosisTestApp>, owner: &SigningAccount) -> String {
    println!("WASM name: {}", wasm_file());
    let wasm_byte_code = std::fs::read(wasm_file()).unwrap();
    let code_id = wasm.store_code(&wasm_byte_code, None, owner).unwrap().data.code_id;

    wasm.instantiate(code_id, &InstantiateMsg {}, None, Some("zapper-osmosis-contract"), &[], owner)
        .unwrap()
        .data
        .address
}

pub fn query_balance(bank: &Bank<OsmosisTestApp>, addr: &str, denom: &str) -> u128 {
    bank.query_balance(&QueryBalanceRequest {
        address: addr.to_string(),
        denom: denom.to_string(),
    })
    .unwrap()
    .balance
    .map(|c| u128::from_str(&c.amount).unwrap())
    .unwrap_or(0)
}

pub fn assert_err(actual: RunnerError, expected: impl Display) {
    match actual {
        RunnerError::ExecuteError {
            msg,
        } => {
            println!("ExecuteError, msg: {msg}");
            assert!(msg.contains(&format!("{expected}")))
        }
        RunnerError::QueryError {
            msg,
        } => {
            println!("QueryError, msg: {msg}");
            assert!(msg.contains(&format!("{expected}")))
        }
        _ => panic!("Unhandled error"),
    }
}
