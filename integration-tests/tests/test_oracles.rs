use crate::helpers::osmosis::instantiate_contract;
use cosmwasm_std::{coin, Decimal, Isqrt, Uint128};
use mars_oracle_osmosis::OsmosisPriceSource;
use mars_outpost::oracle::{ExecuteMsg, InstantiateMsg, PriceResponse, QueryMsg};
use osmosis_testing::{Account, Gamm, Module, OsmosisTestApp, Wasm};
use std::str::FromStr;

mod helpers;

const OSMOSIS_ORACLE_CONTRACT_NAME: &str = "mars-oracle-osmosis";

#[test]
fn querying_xyk_lp_price_if_no_price_for_tokens() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[
            coin(1_000_000_000_000, "uosmo"),
            coin(1_000_000_000_000, "umars"),
            coin(1_000_000_000_000, "uatom"),
        ])
        .unwrap();

    let contract_addr = instantiate_contract(
        &wasm,
        &signer,
        OSMOSIS_ORACLE_CONTRACT_NAME,
        &InstantiateMsg {
            owner: signer.address(),
            base_denom: "uosmo".to_string(),
        },
    );

    let gamm = Gamm::new(&app);
    let pool_mars_atom = gamm
        .create_basic_pool(&[coin(31_500_000, "umars"), coin(1_500_000, "uatom")], &signer)
        .unwrap()
        .data
        .pool_id;

use osmosis_std::types::osmosis::gamm::v1beta1::{Pool, QueryPoolResponse, QuerySpotPriceResponse};
use osmosis_std::types::osmosis::twap::v1beta1::ArithmeticTwapToNowResponse;

use mars_outpost::oracle::InstantiateMsg;

#[test]
fn spot_test() {
    let app = OsmosisTestApp::new();

    let accs = app
        .init_accounts(
            &[Coin::new(1_000_000_000_000, "uatom"), Coin::new(1_000_000_000_000, "uosmo")],
            2,
        )
        .unwrap();

    let user_a = &accs[0];
    let user_b = &accs[1];
}
