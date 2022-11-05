#![allow(dead_code)]

mod test_oracles;

use std::fmt::Display;
use std::str::FromStr;

use osmosis_testing::cosmrs::proto::cosmos::bank::v1beta1::QueryBalanceRequest;
use osmosis_testing::{Account, Bank, OsmosisTestApp, RunnerError, SigningAccount, Wasm};

use cosmwasm_std::Decimal;
use mars_outpost::oracle::InstantiateMsg;
use mars_outpost::red_bank::{
    InitOrUpdateAssetParams, InterestRateModel, UserHealthStatus, UserPositionResponse,
};

//cw-multi-test helpers
pub fn default_asset_params() -> InitOrUpdateAssetParams {
    InitOrUpdateAssetParams {
        initial_borrow_rate: Some(Decimal::percent(10)),
        reserve_factor: Some(Decimal::percent(20)),
        max_loan_to_value: Some(Decimal::percent(60)),
        liquidation_threshold: Some(Decimal::percent(80)),
        liquidation_bonus: Some(Decimal::percent(10)),
        interest_rate_model: Some(InterestRateModel {
            optimal_utilization_rate: Decimal::percent(10),
            base: Decimal::percent(30),
            slope_1: Decimal::percent(25),
            slope_2: Decimal::percent(30),
        }),
        deposit_enabled: Some(true),
        borrow_enabled: Some(true),
        deposit_cap: None,
    }
}

pub fn default_asset_params_with(
    max_loan_to_value: Decimal,
    liquidation_threshold: Decimal,
    liquidation_bonus: Decimal,
) -> InitOrUpdateAssetParams {
    InitOrUpdateAssetParams {
        initial_borrow_rate: Some(Decimal::percent(10)),
        reserve_factor: Some(Decimal::percent(20)),
        max_loan_to_value: Some(max_loan_to_value),
        liquidation_threshold: Some(liquidation_threshold),
        liquidation_bonus: Some(liquidation_bonus),
        interest_rate_model: Some(InterestRateModel {
            optimal_utilization_rate: Decimal::percent(10),
            base: Decimal::percent(30),
            slope_1: Decimal::percent(25),
            slope_2: Decimal::percent(30),
        }),
        deposit_enabled: Some(true),
        borrow_enabled: Some(true),
        deposit_cap: None,
    }
}

pub fn is_user_liquidatable(position: &UserPositionResponse) -> bool {
    match position.health_status {
        UserHealthStatus::NotBorrowing => false,
        UserHealthStatus::Borrowing {
            liq_threshold_hf,
            ..
        } => liq_threshold_hf < Decimal::one(),
    }
}

pub mod osmosis {
    use osmosis_testing::cosmrs::proto::cosmos::bank::v1beta1::QueryBalanceRequest;
    use osmosis_testing::{Bank, OsmosisTestApp, RunnerError, SigningAccount, Wasm};
    use serde::Serialize;
    use std::fmt::Display;
    use std::str::FromStr;

    pub fn wasm_file(contract_name: &str) -> String {
        let artifacts_dir =
            std::env::var("ARTIFACTS_DIR_PATH").unwrap_or_else(|_| "artifacts".to_string());
        let snaked_name = contract_name.replace('-', "_");
        format!("../{}/{}.wasm", artifacts_dir, snaked_name)
    }

    pub fn instantiate_contract<M>(
        wasm: &Wasm<OsmosisTestApp>,
        owner: &SigningAccount,
        contract_name: &str,
        msg: &M,
    ) -> String
    where
        M: ?Sized + Serialize,
    {
        let wasm_byte_code = std::fs::read(wasm_file(contract_name)).unwrap();
        let code_id = wasm.store_code(&wasm_byte_code, None, owner).unwrap().data.code_id;

        wasm.instantiate(code_id, msg, None, Some(contract_name), &[], owner).unwrap().data.address
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
                assert!(msg.contains(&format!("{}", expected)))
            }
            RunnerError::QueryError {
                msg,
            } => {
                assert!(msg.contains(&format!("{}", expected)))
            }
            _ => panic!("Unhandled error"),
        }
    }
}
