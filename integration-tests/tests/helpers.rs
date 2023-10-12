#![allow(dead_code)]

use anyhow::Result as AnyResult;
use cosmwasm_std::{Coin, Decimal, Fraction, Uint128};
use cw_multi_test::AppResponse;
use mars_types::{
    params::{AssetParams, CmSettings, LiquidationBonus, RedBankSettings},
    red_bank::{
        InitOrUpdateAssetParams, InterestRateModel, UserHealthStatus, UserPositionResponse,
    },
};
use osmosis_std::types::osmosis::{
    gamm::v1beta1::{MsgSwapExactAmountIn, MsgSwapExactAmountInResponse},
    poolmanager::v1beta1::SwapAmountInRoute,
};
use osmosis_test_tube::{Account, ExecuteResponse, OsmosisTestApp, Runner, SigningAccount};

pub fn default_asset_params(denom: &str) -> (InitOrUpdateAssetParams, AssetParams) {
    let market_params = InitOrUpdateAssetParams {
        reserve_factor: Some(Decimal::percent(20)),
        interest_rate_model: Some(InterestRateModel {
            optimal_utilization_rate: Decimal::percent(10),
            base: Decimal::percent(30),
            slope_1: Decimal::percent(25),
            slope_2: Decimal::percent(30),
        }),
    };
    let asset_params = AssetParams {
        denom: denom.to_string(),
        credit_manager: CmSettings {
            whitelisted: false,
            hls: None,
        },
        red_bank: RedBankSettings {
            deposit_enabled: true,
            borrow_enabled: true,
        },
        max_loan_to_value: Decimal::percent(60),
        liquidation_threshold: Decimal::percent(80),
        liquidation_bonus: LiquidationBonus {
            starting_lb: Decimal::percent(0u64),
            slope: Decimal::one(),
            min_lb: Decimal::percent(0u64),
            max_lb: Decimal::percent(5u64),
        },
        protocol_liquidation_fee: Decimal::percent(2u64),
        deposit_cap: Uint128::MAX,
    };
    (market_params, asset_params)
}

pub fn default_asset_params_with(
    denom: &str,
    max_loan_to_value: Decimal,
    liquidation_threshold: Decimal,
    liquidation_bonus: LiquidationBonus,
) -> (InitOrUpdateAssetParams, AssetParams) {
    let market_params = InitOrUpdateAssetParams {
        reserve_factor: Some(Decimal::percent(20)),
        interest_rate_model: Some(InterestRateModel {
            optimal_utilization_rate: Decimal::percent(10),
            base: Decimal::percent(30),
            slope_1: Decimal::percent(25),
            slope_2: Decimal::percent(30),
        }),
    };
    let asset_params = AssetParams {
        denom: denom.to_string(),
        credit_manager: CmSettings {
            whitelisted: false,
            hls: None,
        },
        red_bank: RedBankSettings {
            deposit_enabled: true,
            borrow_enabled: true,
        },
        max_loan_to_value,
        liquidation_threshold,
        liquidation_bonus,
        protocol_liquidation_fee: Decimal::percent(2u64),
        deposit_cap: Uint128::MAX,
    };
    (market_params, asset_params)
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

pub fn liq_threshold_hf(position: &UserPositionResponse) -> Decimal {
    match position.health_status {
        UserHealthStatus::Borrowing {
            liq_threshold_hf,
            ..
        } if liq_threshold_hf < Decimal::one() => liq_threshold_hf,
        _ => panic!("User is not liquidatable"),
    }
}

pub fn calculate_max_debt_repayable(
    thf: Decimal,
    tlf: Decimal,
    collateral_liq_th: Decimal,
    debt_price: Decimal,
    position: &UserPositionResponse,
) -> Uint128 {
    let max_debt_repayable_numerator = (thf * position.total_collateralized_debt)
        - position.weighted_liquidation_threshold_collateral;
    let max_debt_repayable_denominator = thf - (collateral_liq_th * (Decimal::one() + tlf));

    let max_debt_repayable_value = max_debt_repayable_numerator.multiply_ratio(
        max_debt_repayable_denominator.denominator(),
        max_debt_repayable_denominator.numerator(),
    );

    max_debt_repayable_value.div_floor(debt_price)
}

pub mod osmosis {
    use std::fmt::Display;

    use osmosis_test_tube::{OsmosisTestApp, RunnerError, SigningAccount, Wasm};
    use serde::Serialize;

    pub fn wasm_file(contract_name: &str) -> String {
        let artifacts_dir =
            std::env::var("ARTIFACTS_DIR_PATH").unwrap_or_else(|_| "artifacts".to_string());
        let snaked_name = contract_name.replace('-', "_");
        let path = format!("../{artifacts_dir}/{snaked_name}.wasm");
        println!("Trying to read wasm file: {}", snaked_name);
        path
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
        println!("uploading {}", wasm_file(contract_name));
        let wasm_byte_code = std::fs::read(wasm_file(contract_name)).unwrap();
        let code_id = wasm.store_code(&wasm_byte_code, None, owner).unwrap().data.code_id;

        wasm.instantiate(code_id, msg, None, Some(contract_name), &[], owner).unwrap().data.address
    }

    pub fn instantiate_stride_contract<M>(
        wasm: &Wasm<OsmosisTestApp>,
        owner: &SigningAccount,
        msg: &M,
    ) -> String
    where
        M: ?Sized + Serialize,
    {
        let path =
            "../integration-tests/tests/files/stride-artifacts/151_stride_redemption_rate.wasm"
                .to_string();
        println!("Trying to read wasm file: {}", path);
        let wasm_byte_code = std::fs::read(path).unwrap();
        let code_id = wasm.store_code(&wasm_byte_code, None, owner).unwrap().data.code_id;

        wasm.instantiate(code_id, msg, None, Some("stride-rr"), &[], owner).unwrap().data.address
    }

    pub fn assert_err(actual: RunnerError, expected: impl Display) {
        match actual {
            RunnerError::ExecuteError {
                msg,
            } => assert!(msg.contains(&expected.to_string())),
            RunnerError::QueryError {
                msg,
            } => assert!(msg.contains(&expected.to_string())),
            _ => panic!("Unhandled error"),
        }
    }
}

/// Every execution creates new block and block timestamp will +5 secs from last block
/// (see https://github.com/osmosis-labs/osmosis-rust/issues/53#issuecomment-1311451418).
///
/// We need to swap n times to pass twap window size. Every swap moves block 5 sec so
/// n = window_size / 5 sec.
pub fn swap_to_create_twap_records(
    app: &OsmosisTestApp,
    signer: &SigningAccount,
    pool_id: u64,
    coin_in: Coin,
    denom_out: &str,
    window_size: u64,
) {
    let n = window_size / 5u64;
    swap_n_times(app, signer, pool_id, coin_in, denom_out, n);
}

pub fn swap_n_times(
    app: &OsmosisTestApp,
    signer: &SigningAccount,
    pool_id: u64,
    coin_in: Coin,
    denom_out: &str,
    n: u64,
) {
    for _ in 0..n {
        swap(app, signer, pool_id, coin_in.clone(), denom_out);
    }
}

pub fn swap(
    app: &OsmosisTestApp,
    signer: &SigningAccount,
    pool_id: u64,
    coin_in: Coin,
    denom_out: &str,
) -> ExecuteResponse<MsgSwapExactAmountInResponse> {
    app.execute::<_, MsgSwapExactAmountInResponse>(
        MsgSwapExactAmountIn {
            sender: signer.address(),
            routes: vec![SwapAmountInRoute {
                pool_id,
                token_out_denom: denom_out.to_string(),
            }],
            token_in: Some(coin_in.into()),
            token_out_min_amount: "1".to_string(),
        },
        MsgSwapExactAmountIn::TYPE_URL,
        signer,
    )
    .unwrap()
}

pub fn assert_red_bank_err(res: AnyResult<AppResponse>, err: mars_red_bank::error::ContractError) {
    match res {
        Ok(_) => panic!("Result was not an error"),
        Err(generic_err) => {
            let contract_err: mars_red_bank::error::ContractError = generic_err.downcast().unwrap();
            assert_eq!(contract_err, err);
        }
    }
}

pub fn assert_incentives_err(res: AnyResult<AppResponse>, err: mars_incentives::ContractError) {
    match res {
        Ok(_) => panic!("Result was not an error"),
        Err(generic_err) => {
            let contract_err: mars_incentives::ContractError = generic_err.downcast().unwrap();
            assert_eq!(contract_err, err);
        }
    }
}
