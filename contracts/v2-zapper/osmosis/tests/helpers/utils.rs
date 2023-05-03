use std::{fmt::Display, str::FromStr};

use mars_v2_zapper_base::InstantiateMsg;
use osmosis_test_tube::{
    cosmrs::proto::cosmos::bank::v1beta1::QueryBalanceRequest, Bank, OsmosisTestApp, RunnerError,
    SigningAccount, Wasm,
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");

pub fn wasm_file() -> Vec<u8> {
    let artifacts_dir =
        std::env::var("ARTIFACTS_DIR_PATH").unwrap_or_else(|_| "artifacts".to_string());
    let snaked_name = CONTRACT_NAME.replace('-', "_");
    let relative_dir = format!("../../../{artifacts_dir}");

    let wasm_file_path = format!("{relative_dir}/{snaked_name}.wasm");

    match std::fs::read(wasm_file_path.clone()) {
        Ok(bytes) => {
            println!("{wasm_file_path}");
            bytes
        }
        // Retry if in arch64 environment
        Err(_) => {
            let alt_file_path = format!("{relative_dir}/{snaked_name}-aarch64.wasm");
            println!("{}", alt_file_path);
            std::fs::read(alt_file_path).unwrap()
        }
    }
}

pub fn instantiate_contract(wasm: &Wasm<OsmosisTestApp>, owner: &SigningAccount) -> String {
    let code_id = wasm.store_code(&wasm_file(), None, owner).unwrap().data.code_id;

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
