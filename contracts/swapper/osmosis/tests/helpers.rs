use std::{fmt::Display, str::FromStr};

use cosmwasm_std::{Coin, Decimal, Uint128};
use mars_rover::adapters::swap::InstantiateMsg;
use osmosis_std::types::osmosis::gamm::v1beta1::{
    MsgSwapExactAmountIn, MsgSwapExactAmountInResponse, SwapAmountInRoute,
};
use osmosis_testing::{
    cosmrs::proto::cosmos::bank::v1beta1::QueryBalanceRequest, Account, Bank, ExecuteResponse,
    Gamm, OsmosisTestApp, Runner, RunnerError, SigningAccount, Wasm,
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");

pub fn wasm_file() -> String {
    let artifacts_dir =
        std::env::var("ARTIFACTS_DIR_PATH").unwrap_or_else(|_| "artifacts".to_string());
    let snaked_name = CONTRACT_NAME.replace('-', "_");
    format!("../../../{artifacts_dir}/{snaked_name}.wasm")
}

pub fn instantiate_contract(wasm: &Wasm<OsmosisTestApp>, owner: &SigningAccount) -> String {
    let wasm_byte_code = std::fs::read(wasm_file()).unwrap();
    let code_id = wasm.store_code(&wasm_byte_code, None, owner).unwrap().data.code_id;

    wasm.instantiate(
        code_id,
        &InstantiateMsg {
            owner: owner.address(),
        },
        None,
        Some("swapper-osmosis-contract"),
        &[],
        owner,
    )
    .unwrap()
    .data
    .address
}

/// Every execution creates new block and block timestamp will +5 secs from last block
/// (see https://github.com/osmosis-labs/osmosis-rust/issues/53#issuecomment-1311451418).
///
/// We need to swap n times to pass TWAP_WINDOW_SIZE_SECONDS (10 min). Every swap moves block 5 sec so
/// n = TWAP_WINDOW_SIZE_SECONDS / 5 sec = 600 sec / 5 sec = 120.
/// We need to swap at least 120 times to create historical index for TWAP.
pub fn swap_to_create_twap_records(
    app: &OsmosisTestApp,
    signer: &SigningAccount,
    pool_id: u64,
    coin_in: Coin,
    denom_out: &str,
) {
    swap_n_times(app, signer, pool_id, coin_in, denom_out, 120u64);
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

fn swap(
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

/// Query price for 1 denom from pool_id (quoted in second denom from the pool).
///
/// Example:
/// pool consists of: 250 uosmo and 100 uatom
/// query price for uatom so 1 uatom = 2.5 uosmo
pub fn query_price_from_pool(gamm: &Gamm<OsmosisTestApp>, pool_id: u64, denom: &str) -> Decimal {
    let pool_assets = &gamm.query_pool(pool_id).unwrap().pool_assets;
    let coin_1 = pool_assets[0].token.as_ref().unwrap();
    let coin_2 = &pool_assets[1].token.as_ref().unwrap();
    let coin_1_amt = Uint128::from_str(&coin_1.amount).unwrap();
    let coin_2_amt = Uint128::from_str(&coin_2.amount).unwrap();

    if coin_1.denom == denom {
        Decimal::from_ratio(coin_2_amt, coin_1_amt)
    } else if coin_2.denom == denom {
        Decimal::from_ratio(coin_1_amt, coin_2_amt)
    } else {
        panic!("{denom} not found in the pool {pool_id}")
    }
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
            assert!(msg.contains(&format!("{expected}")))
        }
        RunnerError::QueryError {
            msg,
        } => {
            assert!(msg.contains(&format!("{expected}")))
        }
        _ => panic!("Unhandled error"),
    }
}
