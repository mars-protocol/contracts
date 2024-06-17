use std::{str::FromStr, time::SystemTime};

use cosmwasm_std::{coin, to_json_binary, Coin, Decimal, Empty, Isqrt, Uint128};
use helpers::osmosis::instantiate_stride_contract;
use mars_oracle_base::{redemption_rate::RedemptionRate, ContractError};
use mars_oracle_osmosis::{
    msg::PriceSourceResponse, DowntimeDetector, OsmosisPriceSourceChecked,
    OsmosisPriceSourceUnchecked, Twap, TwapKind,
};
use mars_types::{
    address_provider::{
        ExecuteMsg::SetAddress, InstantiateMsg as InstantiateAddr, MarsAddressType,
    },
    incentives::InstantiateMsg as InstantiateIncentives,
    oracle::{ExecuteMsg, InstantiateMsg, PriceResponse, QueryMsg},
    params::AssetParamsUpdate,
    red_bank::{
        CreateOrUpdateConfig, ExecuteMsg as ExecuteRedBank,
        ExecuteMsg::{Borrow, Deposit},
        InstantiateMsg as InstantiateRedBank,
    },
    rewards_collector::InstantiateMsg as InstantiateRewards,
};
use osmosis_std::types::osmosis::{
    downtimedetector::v1beta1::Downtime,
    gamm::poolmodels::stableswap::v1beta1::MsgCreateStableswapPool,
};
use osmosis_test_tube::{
    osmosis_std::types::osmosis::gamm::poolmodels::stableswap::v1beta1::PoolParams, Account, Gamm,
    Module, OsmosisTestApp, RunnerResult, SigningAccount, Wasm,
};

use crate::helpers::{
    default_asset_params,
    osmosis::{assert_err, instantiate_contract},
    swap, swap_to_create_twap_records,
};

mod helpers;

const OSMOSIS_ORACLE_CONTRACT_NAME: &str = "mars-oracle-osmosis";
const OSMOSIS_RED_BANK_CONTRACT_NAME: &str = "mars-red-bank";
const OSMOSIS_ADDR_PROVIDER_CONTRACT_NAME: &str = "mars-address-provider";
const OSMOSIS_REWARDS_CONTRACT_NAME: &str = "mars-rewards-collector-osmosis";
const OSMOSIS_INCENTIVES_CONTRACT_NAME: &str = "mars-incentives";
const OSMOSIS_PARAMS_CONTRACT_NAME: &str = "mars-params";

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
        &InstantiateMsg::<Empty> {
            owner: signer.address(),
            base_denom: "uosmo".to_string(),
            custom_init: None,
        },
    );

    let gamm = Gamm::new(&app);
    let pool_mars_atom = gamm
        .create_basic_pool(&[coin(31_500_000, "umars"), coin(1_500_000, "uatom")], &signer)
        .unwrap()
        .data
        .pool_id;

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<_, Empty>::SetPriceSource {
            denom: "umars_uatom_lp".to_string(),
            price_source: OsmosisPriceSourceUnchecked::XykLiquidityToken {
                pool_id: pool_mars_atom,
            },
        },
        &[],
        &signer,
    )
    .unwrap();

    // Should fail - missing price for umars and uatom
    wasm.query::<QueryMsg, PriceResponse>(
        &contract_addr,
        &QueryMsg::Price {
            denom: "umars_uatom_lp".to_string(),
            kind: None,
        },
    )
    .unwrap_err();
}

#[test]
fn querying_xyk_lp_price_success() {
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
        &InstantiateMsg::<Empty> {
            owner: signer.address(),
            base_denom: "uosmo".to_string(),
            custom_init: None,
        },
    );

    let gamm = Gamm::new(&app);
    let pool_mars_atom = gamm
        .create_basic_pool(
            &[coin(31_500_000, "umars"), coin(1_500_000, "uatom")], // 1 atom = 21 mars
            &signer,
        )
        .unwrap()
        .data
        .pool_id;
    let pool_mars_osmo = gamm
        .create_basic_pool(
            &[coin(6_000_000, "umars"), coin(1_500_000, "uosmo")], // 1 mars = 0.25 osmo
            &signer,
        )
        .unwrap()
        .data
        .pool_id;
    let pool_atom_osmo = gamm
        .create_basic_pool(
            &[coin(1_000_000, "uatom"), coin(10_000_000, "uosmo")], // 1 atom = 10 osmo
            &signer,
        )
        .unwrap()
        .data
        .pool_id;

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<_, Empty>::SetPriceSource {
            denom: "umars_uatom_lp".to_string(),
            price_source: OsmosisPriceSourceUnchecked::XykLiquidityToken {
                pool_id: pool_mars_atom,
            },
        },
        &[],
        &signer,
    )
    .unwrap();
    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<_, Empty>::SetPriceSource {
            denom: "umars".to_string(),
            price_source: OsmosisPriceSourceUnchecked::Spot {
                pool_id: pool_mars_osmo,
            },
        },
        &[],
        &signer,
    )
    .unwrap();
    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<_, Empty>::SetPriceSource {
            denom: "uatom".to_string(),
            price_source: OsmosisPriceSourceUnchecked::Spot {
                pool_id: pool_atom_osmo,
            },
        },
        &[],
        &signer,
    )
    .unwrap();

    // Mars price: 0.25 osmo
    // Mars depth: 31_500_000
    // Atom price: 10 osmo
    // Atom depth: 1_500_000
    // pool value: 2 * sqrt((0.25 * 31_500_000) * (10 * 1_500_000))
    // LP token price: pool value / lp shares
    let mars_price = Decimal::from_ratio(1u128, 4u128);
    let atom_price = Decimal::from_ratio(10u128, 1u128);
    let mars_value = Uint128::from(31_500_000u128) * mars_price;
    let atom_value = Uint128::from(1_500_000u128) * atom_price;
    let pool_value = Uint128::from(2u8) * (mars_value * atom_value).isqrt();
    let pool = gamm.query_pool(pool_mars_atom).unwrap();
    let mars_atom_lp_shares = Uint128::from_str(&pool.total_shares.unwrap().amount).unwrap();
    let lp_price = Decimal::from_ratio(pool_value, mars_atom_lp_shares);
    let res: PriceResponse = wasm
        .query(
            &contract_addr,
            &QueryMsg::Price {
                denom: "umars_uatom_lp".to_string(),
                kind: None,
            },
        )
        .unwrap();
    assert_eq!(res.price, lp_price);
}

// set osmo-atom liquidity pool, set oracle to SPOT, query price source, query price
#[test]
fn query_spot_price() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "uosmo"), coin(1_000_000_000_000, "uatom")])
        .unwrap();

    let oracle_addr = instantiate_contract(
        &wasm,
        &signer,
        OSMOSIS_ORACLE_CONTRACT_NAME,
        &InstantiateMsg::<Empty> {
            owner: signer.address(),
            base_denom: "uosmo".to_string(),
            custom_init: None,
        },
    );

    let gamm = Gamm::new(&app);
    let pool_liquidity = vec![Coin::new(2_000_000, "uatom"), Coin::new(1_000_000, "uosmo")];
    let pool_id = gamm.create_basic_pool(&pool_liquidity, &signer).unwrap().data.pool_id;

    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::<_, Empty>::SetPriceSource {
            denom: "uatom".to_string(),
            price_source: OsmosisPriceSourceUnchecked::Spot {
                pool_id,
            },
        },
        &[],
        &signer,
    )
    .unwrap();

    let price_source: PriceSourceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::PriceSource {
                denom: "uatom".to_string(),
            },
        )
        .unwrap();
    assert_eq!(
        price_source.price_source,
        (OsmosisPriceSourceChecked::Spot {
            pool_id
        })
    );

    let price: PriceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::Price {
                denom: "uatom".to_string(),
                kind: None,
            },
        )
        .unwrap();
    assert_eq!(price.price, Decimal::from_ratio(1u128, 2u128)); // 1 osmo = 2 atom
    assert_eq!(price.denom, "uatom".to_string());
}

// set price source to spot for without creating a liquidity pool - should return an error
#[test]
fn set_spot_without_pools() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "uosmo"), coin(1_000_000_000_000, "uatom")])
        .unwrap();

    let oracle_addr = instantiate_contract(
        &wasm,
        &signer,
        OSMOSIS_ORACLE_CONTRACT_NAME,
        &InstantiateMsg::<Empty> {
            owner: signer.address(),
            base_denom: "uosmo".to_string(),
            custom_init: None,
        },
    );

    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::<_, Empty>::SetPriceSource {
            denom: "uatom".to_string(),
            price_source: OsmosisPriceSourceUnchecked::Spot {
                pool_id: 1u64,
            },
        },
        &[],
        &signer,
    )
    .unwrap_err();

    // returns a generic error:
    // ExecuteError { msg: "failed to execute message; message index: 0: Generic error: Querier contract error: codespace: undefined, code: 1: execute wasm contract failed" }
}

// set price source to spot and query the price of an asset that is not in the pool - should return an error
#[test]
fn incorrect_pool_for_spot() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "uosmo"), coin(1_000_000_000_000, "uatom")])
        .unwrap();

    let oracle_addr = instantiate_contract(
        &wasm,
        &signer,
        OSMOSIS_ORACLE_CONTRACT_NAME,
        &InstantiateMsg::<Empty> {
            owner: signer.address(),
            base_denom: "uosmo".to_string(),
            custom_init: None,
        },
    );

    let gamm = Gamm::new(&app);
    let pool_liquidity = vec![Coin::new(2_000_000, "uatom"), Coin::new(1_000_000, "uosmo")];
    let pool_id = gamm.create_basic_pool(&pool_liquidity, &signer).unwrap().data.pool_id;

    let res = wasm
        .execute(
            &oracle_addr,
            &ExecuteMsg::<_, Empty>::SetPriceSource {
                denom: "umars".to_string(),
                price_source: OsmosisPriceSourceUnchecked::Spot {
                    pool_id,
                },
            },
            &[],
            &signer,
        )
        .unwrap_err();

    assert_err(
        res,
        ContractError::InvalidPriceSource {
            reason: "pool 1 does not contain umars".to_string(),
        },
    )
}

// change pool liquidity and assert accurate price change on asset
#[test]
fn update_spot_with_different_pool() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "uosmo"), coin(1_000_000_000_000, "uatom")])
        .unwrap();

    let oracle_addr = instantiate_contract(
        &wasm,
        &signer,
        OSMOSIS_ORACLE_CONTRACT_NAME,
        &InstantiateMsg::<Empty> {
            owner: signer.address(),
            base_denom: "uosmo".to_string(),
            custom_init: None,
        },
    );

    let gamm = Gamm::new(&app);
    let pool_liquidity = vec![Coin::new(98_000_000, "uatom"), Coin::new(1_764_000_000, "uosmo")];
    let pool_id = gamm.create_basic_pool(&pool_liquidity, &signer).unwrap().data.pool_id;

    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::<_, Empty>::SetPriceSource {
            denom: "uatom".to_string(),
            price_source: OsmosisPriceSourceUnchecked::Spot {
                pool_id,
            },
        },
        &[],
        &signer,
    )
    .unwrap();

    let price: PriceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::Price {
                denom: "uatom".to_string(),
                kind: None,
            },
        )
        .unwrap();
    assert_eq!(price.price, Decimal::from_ratio(1764u128, 98u128));

    let pool_liquidity = vec![Coin::new(13_000_000, "uatom"), Coin::new(78_000_000, "uosmo")];
    let pool_id = gamm.create_basic_pool(&pool_liquidity, &signer).unwrap().data.pool_id;

    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::<_, Empty>::SetPriceSource {
            denom: "uatom".to_string(),
            price_source: OsmosisPriceSourceUnchecked::Spot {
                pool_id,
            },
        },
        &[],
        &signer,
    )
    .unwrap();

    let price: PriceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::Price {
                denom: "uatom".to_string(),
                kind: None,
            },
        )
        .unwrap();
    assert_eq!(price.price, Decimal::from_ratio(78u128, 13u128));
}

// test a swap executed that changes the liquidity pool size and test how it corresponds to the price.
#[test]
fn query_spot_price_after_lp_change() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "uosmo"), coin(1_000_000_000_000, "uatom")])
        .unwrap();

    let oracle_addr = instantiate_contract(
        &wasm,
        &signer,
        OSMOSIS_ORACLE_CONTRACT_NAME,
        &InstantiateMsg::<Empty> {
            owner: signer.address(),
            base_denom: "uosmo".to_string(),
            custom_init: None,
        },
    );

    let gamm = Gamm::new(&app);
    let pool_liquidity = vec![Coin::new(2_000, "uatom"), Coin::new(1_000, "uosmo")];
    let pool_id = gamm.create_basic_pool(&pool_liquidity, &signer).unwrap().data.pool_id;

    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::<_, Empty>::SetPriceSource {
            denom: "uatom".to_string(),
            price_source: OsmosisPriceSourceUnchecked::Spot {
                pool_id,
            },
        },
        &[],
        &signer,
    )
    .unwrap();

    let price: PriceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::Price {
                denom: "uatom".to_string(),
                kind: None,
            },
        )
        .unwrap();
    assert_eq!(price.price, Decimal::from_ratio(1u128, 2u128));

    swap(&app, &signer, pool_id, coin(100u128, "uosmo"), "uatom");

    let price2: PriceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::Price {
                denom: "uatom".to_string(),
                kind: None,
            },
        )
        .unwrap();
    assert!(price.price < price2.price);
}

#[test]
fn query_geometric_twap_price_with_downtime_detector() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "uosmo"), coin(1_000_000_000_000, "uatom")])
        .unwrap();

    let oracle_addr = instantiate_contract(
        &wasm,
        &signer,
        OSMOSIS_ORACLE_CONTRACT_NAME,
        &InstantiateMsg::<Empty> {
            owner: signer.address(),
            base_denom: "uosmo".to_string(),
            custom_init: None,
        },
    );

    let gamm = Gamm::new(&app);
    let pool_liquidity = vec![Coin::new(2_000_000_000, "uatom"), Coin::new(1_000_000_000, "uosmo")];
    let pool_id = gamm.create_basic_pool(&pool_liquidity, &signer).unwrap().data.pool_id;

    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::<_, Empty>::SetPriceSource {
            denom: "uatom".to_string(),
            price_source: OsmosisPriceSourceUnchecked::GeometricTwap {
                pool_id,
                window_size: 10, // 10 seconds = 2 swaps when each swap increases block time by 5 seconds
                downtime_detector: Some(DowntimeDetector {
                    downtime: Downtime::Duration2m,
                    recovery: 60u64,
                }),
            },
        },
        &[],
        &signer,
    )
    .unwrap();
    let price_source: PriceSourceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::PriceSource {
                denom: "uatom".to_string(),
            },
        )
        .unwrap();
    assert_eq!(
        price_source.price_source,
        (OsmosisPriceSourceChecked::GeometricTwap {
            pool_id,
            window_size: 10,
            downtime_detector: Some(DowntimeDetector {
                downtime: Downtime::Duration2m,
                recovery: 60u64
            }),
        })
    );

    // chain has just started
    let res: RunnerResult<PriceResponse> = wasm.query(
        &oracle_addr,
        &QueryMsg::Price {
            denom: "uatom".to_string(),
            kind: None,
        },
    );
    assert_err(res.unwrap_err(), "chain is recovering from downtime");

    // window_size > recovery (60 sec)
    swap_to_create_twap_records(&app, &signer, pool_id, coin(1u128, "uosmo"), "uatom", 100);

    // chain recovered
    let _res: PriceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::Price {
                denom: "uatom".to_string(),
                kind: None,
            },
        )
        .unwrap();
}

// assert oracle was correctly set to Arithmetic TWAP and assert prices are queried correctly
#[test]
fn query_arithmetic_twap_price() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "uosmo"), coin(1_000_000_000_000, "uatom")])
        .unwrap();

    let oracle_addr = instantiate_contract(
        &wasm,
        &signer,
        OSMOSIS_ORACLE_CONTRACT_NAME,
        &InstantiateMsg::<Empty> {
            owner: signer.address(),
            base_denom: "uosmo".to_string(),
            custom_init: None,
        },
    );

    let gamm = Gamm::new(&app);
    let pool_liquidity = vec![Coin::new(2_000_000_000, "uatom"), Coin::new(1_000_000_000, "uosmo")];
    let pool_id = gamm.create_basic_pool(&pool_liquidity, &signer).unwrap().data.pool_id;

    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::<_, Empty>::SetPriceSource {
            denom: "uatom".to_string(),
            price_source: OsmosisPriceSourceUnchecked::ArithmeticTwap {
                pool_id,
                window_size: 10, // 10 seconds = 2 swaps when each swap increases block time by 5 seconds
                downtime_detector: None,
            },
        },
        &[],
        &signer,
    )
    .unwrap();

    swap_to_create_twap_records(&app, &signer, pool_id, coin(1u128, "uosmo"), "uatom", 10);

    let price_source: PriceSourceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::PriceSource {
                denom: "uatom".to_string(),
            },
        )
        .unwrap();
    assert_eq!(
        price_source.price_source,
        (OsmosisPriceSourceChecked::ArithmeticTwap {
            pool_id,
            window_size: 10,
            downtime_detector: None
        })
    );

    // since swaps were small, the prices should be the same within a 1% tolerance
    let tolerance = Decimal::percent(1);

    let price: PriceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::Price {
                denom: "uatom".to_string(),
                kind: None,
            },
        )
        .unwrap();
    // calculate spot price
    let spot_price = Decimal::from_ratio(1u128, 2u128);
    assert!((price.price - spot_price) < tolerance);

    swap_to_create_twap_records(&app, &signer, pool_id, coin(1u128, "uosmo"), "uatom", 10);

    let price2: PriceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::Price {
                denom: "uatom".to_string(),
                kind: None,
            },
        )
        .unwrap();
    assert!(price2.price - price.price < tolerance);
}

// assert oracle was correctly set to Geometric TWAP and assert prices are queried correctly
#[test]
fn query_geometric_twap_price() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "uosmo"), coin(1_000_000_000_000, "uatom")])
        .unwrap();

    let oracle_addr = instantiate_contract(
        &wasm,
        &signer,
        OSMOSIS_ORACLE_CONTRACT_NAME,
        &InstantiateMsg::<Empty> {
            owner: signer.address(),
            base_denom: "uosmo".to_string(),
            custom_init: None,
        },
    );

    let gamm = Gamm::new(&app);
    let pool_liquidity = vec![Coin::new(4_000_000_000, "uatom"), Coin::new(1_000_000_000, "uosmo")];
    let pool_id = gamm.create_basic_pool(&pool_liquidity, &signer).unwrap().data.pool_id;

    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::<_, Empty>::SetPriceSource {
            denom: "uatom".to_string(),
            price_source: OsmosisPriceSourceUnchecked::GeometricTwap {
                pool_id,
                window_size: 10, // 10 seconds = 2 swaps when each swap increases block time by 5 seconds
                downtime_detector: None,
            },
        },
        &[],
        &signer,
    )
    .unwrap();

    swap_to_create_twap_records(&app, &signer, pool_id, coin(1u128, "uosmo"), "uatom", 10);

    let price_source: PriceSourceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::PriceSource {
                denom: "uatom".to_string(),
            },
        )
        .unwrap();
    assert_eq!(
        price_source.price_source,
        (OsmosisPriceSourceChecked::GeometricTwap {
            pool_id,
            window_size: 10,
            downtime_detector: None
        })
    );

    // since swaps were small, the prices should be the same within a 1% tolerance
    let tolerance = Decimal::percent(1);

    let price: PriceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::Price {
                denom: "uatom".to_string(),
                kind: None,
            },
        )
        .unwrap();
    // calculate spot price
    let spot_price = Decimal::from_ratio(1u128, 4u128);
    assert!((spot_price - price.price) < tolerance);

    swap_to_create_twap_records(&app, &signer, pool_id, coin(1u128, "uosmo"), "uatom", 10);

    let price2: PriceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::Price {
                denom: "uatom".to_string(),
                kind: None,
            },
        )
        .unwrap();
    assert!(price2.price - price.price < tolerance);
}

// compare SPOT and TWAP prices
#[test]
fn compare_spot_and_twap_price() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "uosmo"), coin(1_000_000_000_000, "uatom")])
        .unwrap();

    let oracle_addr = instantiate_contract(
        &wasm,
        &signer,
        OSMOSIS_ORACLE_CONTRACT_NAME,
        &InstantiateMsg::<Empty> {
            owner: signer.address(),
            base_denom: "uosmo".to_string(),
            custom_init: None,
        },
    );

    let gamm = Gamm::new(&app);
    let pool_liquidity = vec![Coin::new(2_000_000_000, "uatom"), Coin::new(1_000_000_000, "uosmo")];
    let pool_id = gamm.create_basic_pool(&pool_liquidity, &signer).unwrap().data.pool_id;

    // do more swaps than window_size
    swap_to_create_twap_records(&app, &signer, pool_id, coin(1u128, "uosmo"), "uatom", 300);

    // set spot price source
    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::<_, Empty>::SetPriceSource {
            denom: "uatom".to_string(),
            price_source: OsmosisPriceSourceUnchecked::Spot {
                pool_id,
            },
        },
        &[],
        &signer,
    )
    .unwrap();
    let price_source: PriceSourceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::PriceSource {
                denom: "uatom".to_string(),
            },
        )
        .unwrap();
    assert_eq!(
        price_source.price_source,
        OsmosisPriceSourceChecked::Spot {
            pool_id,
        }
    );
    let spot_price: PriceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::Price {
                denom: "uatom".to_string(),
                kind: None,
            },
        )
        .unwrap();

    // override price source to arithmetic TWAP
    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::<_, Empty>::SetPriceSource {
            denom: "uatom".to_string(),
            price_source: OsmosisPriceSourceUnchecked::ArithmeticTwap {
                pool_id,
                window_size: 10, // 10 seconds = 2 swaps when each swap increases block time by 5 seconds
                downtime_detector: None,
            },
        },
        &[],
        &signer,
    )
    .unwrap();
    let price_source: PriceSourceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::PriceSource {
                denom: "uatom".to_string(),
            },
        )
        .unwrap();
    assert_eq!(
        price_source.price_source,
        OsmosisPriceSourceChecked::ArithmeticTwap {
            pool_id,
            window_size: 10,
            downtime_detector: None
        }
    );
    let arithmetic_twap_price: PriceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::Price {
                denom: "uatom".to_string(),
                kind: None,
            },
        )
        .unwrap();

    // override price source to geometric TWAP
    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::<_, Empty>::SetPriceSource {
            denom: "uatom".to_string(),
            price_source: OsmosisPriceSourceUnchecked::GeometricTwap {
                pool_id,
                window_size: 10, // 10 seconds = 2 swaps when each swap increases block time by 5 seconds
                downtime_detector: None,
            },
        },
        &[],
        &signer,
    )
    .unwrap();
    let price_source: PriceSourceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::PriceSource {
                denom: "uatom".to_string(),
            },
        )
        .unwrap();
    assert_eq!(
        price_source.price_source,
        OsmosisPriceSourceChecked::GeometricTwap {
            pool_id,
            window_size: 10,
            downtime_detector: None
        }
    );
    let geometric_twap_price: PriceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::Price {
                denom: "uatom".to_string(),
                kind: None,
            },
        )
        .unwrap();

    let tolerance = Decimal::percent(1);
    assert!(spot_price.price.abs_diff(arithmetic_twap_price.price) < tolerance);
    assert!(spot_price.price.abs_diff(geometric_twap_price.price) < tolerance);
}

// assert oracle was correctly set to LSD and assert prices are queried correctly
#[test]
fn query_lsd_price() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let ibc_stuosmo = format!(
        "ibc/{}",
        "d176154b0c63d1f9c6dcfb4f70349ebf2e2b5a87a05902f57a6ae92b863e9aec" // hash for: transfer/channel-326/stuosmo
            .to_ascii_uppercase()
    );

    let signer = app
        .init_account(&[
            coin(100_000_000_000_000, "uosmo"),
            coin(100_000_000_000_000, &ibc_stuosmo),
        ])
        .unwrap();

    let oracle_addr = instantiate_contract(
        &wasm,
        &signer,
        OSMOSIS_ORACLE_CONTRACT_NAME,
        &InstantiateMsg::<Empty> {
            owner: signer.address(),
            base_denom: "uosmo".to_string(),
            custom_init: None,
        },
    );

    let stride_addr = instantiate_stride_contract(
        &wasm,
        &signer,
        &ica_oracle::msg::InstantiateMsg {
            admin_address: signer.address(),
            transfer_channel_id: Some("channel-326".to_string()),
        },
    );

    let gamm = Gamm::new(&app);
    let pool_id = gamm
        .create_stable_swap_pool(
            MsgCreateStableswapPool {
                sender: signer.address(),
                pool_params: Some(PoolParams {
                    swap_fee: "10000000000000000".to_string(),
                    exit_fee: "0".to_string(),
                }),
                initial_pool_liquidity: vec![
                    osmosis_std::types::cosmos::base::v1beta1::Coin {
                        denom: ibc_stuosmo.to_string(),
                        amount: "3800671945286".to_string(),
                    },
                    osmosis_std::types::cosmos::base::v1beta1::Coin {
                        denom: "uosmo".to_string(),
                        amount: "3261943288901".to_string(),
                    },
                ],
                scaling_factors: vec![100000, 115680],
                future_pool_governor: "".to_string(),
                scaling_factor_controller: signer.address(),
            },
            &signer,
        )
        .unwrap()
        .data
        .pool_id;

    // setup uosmo price
    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::<_, Empty>::SetPriceSource {
            denom: "uosmo".to_string(),
            price_source: OsmosisPriceSourceUnchecked::Fixed {
                price: Decimal::one(),
            },
        },
        &[],
        &signer,
    )
    .unwrap();

    // setup Geomertic TWAP price source in order to test TWAP price for StableSwap pool
    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::<_, Empty>::SetPriceSource {
            denom: ibc_stuosmo.to_string(),
            price_source: OsmosisPriceSourceUnchecked::GeometricTwap {
                pool_id,
                window_size: 10, // 10 seconds = 2 swaps when each swap increases block time by 5 seconds
                downtime_detector: None,
            },
        },
        &[],
        &signer,
    )
    .unwrap();

    swap_to_create_twap_records(&app, &signer, pool_id, coin(10u128, "uosmo"), &ibc_stuosmo, 10);

    let price: PriceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::Price {
                denom: ibc_stuosmo.to_string(),
                kind: None,
            },
        )
        .unwrap();
    let ibc_stuosmo_twap_price = price.price;

    // setup LSD price source with StableSwap pool and redemption rate contract
    let max_staleness = 3600u64;
    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::<_, Empty>::SetPriceSource {
            denom: ibc_stuosmo.to_string(),
            price_source: OsmosisPriceSourceUnchecked::Lsd {
                transitive_denom: "uosmo".to_string(),
                twap: Twap {
                    pool_id,
                    window_size: 10,
                    downtime_detector: None,
                    kind: TwapKind::ArithmeticTwap {},
                },
                redemption_rate: RedemptionRate {
                    contract_addr: stride_addr.clone(),
                    max_staleness,
                },
            },
        },
        &[],
        &signer,
    )
    .unwrap();

    let rr_attr = ica_oracle::state::RedemptionRateAttributes {
        sttoken_denom: "stuosmo".to_string(),
    };
    let rr_attr_bin = to_json_binary(&rr_attr).unwrap();
    let rr_value = Decimal::from_str("1.123").unwrap();
    let now_sec = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
    wasm.execute(
        &stride_addr,
        &ica_oracle::msg::ExecuteMsg::PostMetric {
            key: "ustosmo_redemption_rate".to_string(),
            value: rr_value.to_string(),
            metric_type: ica_oracle::state::MetricType::RedemptionRate,
            update_time: now_sec - 10,
            block_height: 0,
            attributes: Some(rr_attr_bin),
        },
        &[],
        &signer,
    )
    .unwrap();

    let res: ica_oracle::msg::RedemptionRateResponse = wasm
        .query(
            &stride_addr,
            &ica_oracle::msg::QueryMsg::RedemptionRate {
                denom: ibc_stuosmo.clone(),
                params: None,
            },
        )
        .unwrap();
    assert_eq!(res.redemption_rate, rr_value);

    let price: PriceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::Price {
                denom: ibc_stuosmo,
                kind: None,
            },
        )
        .unwrap();
    assert!(ibc_stuosmo_twap_price > rr_value);
    // twap price > rr then we take min(twap, rr)
    assert_eq!(price.price, rr_value);
}

// execute borrow action in red bank with an asset not in the oracle - should fail when attempting to query oracle
#[test]
fn redbank_should_fail_if_no_price() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let account = app
        .init_accounts(
            &[Coin::new(1_000_000_000_000, "uosmo"), Coin::new(1_000_000_000_000, "uatom")],
            2,
        )
        .unwrap();

    let signer = &account[0];
    let depositor = &account[1];

    let (oracle_addr, red_bank_addr) = setup_redbank(&wasm, signer);

    let gamm = Gamm::new(&app);
    let pool_liquidity = vec![Coin::new(2_000_000, "uatom"), Coin::new(1_000_000, "uosmo")];
    let pool_id = gamm.create_basic_pool(&pool_liquidity, signer).unwrap().data.pool_id;

    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::<_, Empty>::SetPriceSource {
            denom: "uatom".to_string(),
            price_source: OsmosisPriceSourceUnchecked::Spot {
                pool_id,
            },
        },
        &[],
        signer,
    )
    .unwrap();

    wasm.execute(
        &red_bank_addr,
        &Deposit {
            account_id: None,
            on_behalf_of: None,
        },
        &[coin(1_000_000, "uatom")],
        depositor,
    )
    .unwrap();

    // execute msg should fail since it is attempting to query an asset from the oracle contract that doesn't have an LP pool set up
    wasm.execute(
        &red_bank_addr,
        &Borrow {
            denom: "umars".to_string(),
            amount: Uint128::new(10_000),
            recipient: None,
        },
        &[],
        depositor,
    )
    .unwrap_err();

    // returns a generic error:
    // ExecuteError { msg: "failed to execute message; message index: 0: Generic error: Querier contract error: codespace: undefined, code: 1: execute wasm contract failed" }
}

// execute borrow action in red bank to confirm the redbank is properly querying oracle
#[test]
fn redbank_quering_oracle_successfully() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let account = app
        .init_accounts(
            &[Coin::new(1_000_000_000_000, "uosmo"), Coin::new(1_000_000_000_000, "uatom")],
            2,
        )
        .unwrap();

    let signer = &account[0];
    let depositor = &account[1];

    let (oracle_addr, red_bank_addr) = setup_redbank(&wasm, signer);

    let gamm = Gamm::new(&app);
    let pool_liquidity = vec![Coin::new(2_000_000, "uatom"), Coin::new(1_000_000, "uosmo")];
    let pool_id = gamm.create_basic_pool(&pool_liquidity, signer).unwrap().data.pool_id;

    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::<_, Empty>::SetPriceSource {
            denom: "uatom".to_string(),
            price_source: OsmosisPriceSourceUnchecked::Spot {
                pool_id,
            },
        },
        &[],
        signer,
    )
    .unwrap();

    wasm.execute(
        &red_bank_addr,
        &Deposit {
            account_id: None,
            on_behalf_of: None,
        },
        &[coin(1_000_000, "uatom")],
        depositor,
    )
    .unwrap();

    wasm.execute(
        &red_bank_addr,
        &Borrow {
            denom: "uatom".to_string(),
            amount: Uint128::new(10_000),
            recipient: None,
        },
        &[],
        depositor,
    )
    .unwrap();
}

// helper function for redbank setup
fn setup_redbank(wasm: &Wasm<OsmosisTestApp>, signer: &SigningAccount) -> (String, String) {
    let oracle_addr = instantiate_contract(
        wasm,
        signer,
        OSMOSIS_ORACLE_CONTRACT_NAME,
        &InstantiateMsg::<Empty> {
            owner: signer.address(),
            base_denom: "uosmo".to_string(),
            custom_init: None,
        },
    );

    let addr_provider_addr = instantiate_contract(
        wasm,
        signer,
        OSMOSIS_ADDR_PROVIDER_CONTRACT_NAME,
        &InstantiateAddr {
            owner: signer.address(),
            prefix: "osmo".to_string(),
        },
    );

    let red_bank_addr = instantiate_contract(
        wasm,
        signer,
        OSMOSIS_RED_BANK_CONTRACT_NAME,
        &InstantiateRedBank {
            owner: signer.address(),
            config: CreateOrUpdateConfig {
                address_provider: Some(addr_provider_addr.clone()),
            },
        },
    );

    let incentives_addr = instantiate_contract(
        wasm,
        signer,
        OSMOSIS_INCENTIVES_CONTRACT_NAME,
        &InstantiateIncentives {
            owner: signer.address(),
            address_provider: addr_provider_addr.clone(),
            epoch_duration: 604800, // 1 week in seconds
            max_whitelisted_denoms: 10,
        },
    );

    let rewards_addr = instantiate_contract(
        wasm,
        signer,
        OSMOSIS_REWARDS_CONTRACT_NAME,
        &InstantiateRewards {
            owner: (signer.address()),
            address_provider: addr_provider_addr.clone(),
            safety_tax_rate: Decimal::percent(25),
            safety_fund_denom: "uosmo".to_string(),
            fee_collector_denom: "uosmo".to_string(),
            channel_id: "channel-1".to_string(),
            timeout_seconds: 60,
            slippage_tolerance: Decimal::new(Uint128::from(1u128)),
            neutron_ibc_config: None,
        },
    );

    let params_addr = instantiate_contract(
        wasm,
        signer,
        OSMOSIS_PARAMS_CONTRACT_NAME,
        &mars_types::params::InstantiateMsg {
            owner: (signer.address()),
            address_provider: addr_provider_addr.clone(),
            target_health_factor: Decimal::from_str("1.05").unwrap(),
        },
    );

    wasm.execute(
        &addr_provider_addr,
        &SetAddress {
            address_type: MarsAddressType::RedBank,
            address: red_bank_addr.clone(),
        },
        &[],
        signer,
    )
    .unwrap();

    wasm.execute(
        &addr_provider_addr,
        &SetAddress {
            address_type: MarsAddressType::Incentives,
            address: incentives_addr,
        },
        &[],
        signer,
    )
    .unwrap();

    wasm.execute(
        &addr_provider_addr,
        &SetAddress {
            address_type: MarsAddressType::Oracle,
            address: oracle_addr.clone(),
        },
        &[],
        signer,
    )
    .unwrap();

    wasm.execute(
        &addr_provider_addr,
        &SetAddress {
            address_type: MarsAddressType::RewardsCollector,
            address: rewards_addr,
        },
        &[],
        signer,
    )
    .unwrap();

    wasm.execute(
        &addr_provider_addr,
        &SetAddress {
            address_type: MarsAddressType::Params,
            address: params_addr.clone(),
        },
        &[],
        signer,
    )
    .unwrap();

    // We can simulate credit manager contract balance with own params address (used by params contract for deposit caps logic)
    wasm.execute(
        &addr_provider_addr,
        &SetAddress {
            address_type: MarsAddressType::CreditManager,
            address: params_addr.clone(),
        },
        &[],
        signer,
    )
    .unwrap();

    // We can simulate Astroport incentives contract deposits with own params address (we don't check deposit caps for Astroport incentives contract so it's safe to use params address here)
    wasm.execute(
        &addr_provider_addr,
        &SetAddress {
            address_type: MarsAddressType::AstroportIncentives,
            address: params_addr.clone(),
        },
        &[],
        signer,
    )
    .unwrap();

    let (market_params, asset_params) = default_asset_params("uosmo");

    wasm.execute(
        &red_bank_addr,
        &ExecuteRedBank::InitAsset {
            denom: "uosmo".to_string(),
            params: market_params,
        },
        &[],
        signer,
    )
    .unwrap();
    wasm.execute(
        &params_addr,
        &mars_types::params::ExecuteMsg::UpdateAssetParams(AssetParamsUpdate::AddOrUpdate {
            params: asset_params.into(),
        }),
        &[],
        signer,
    )
    .unwrap();

    let (market_params, asset_params) = default_asset_params("uatom");

    wasm.execute(
        &red_bank_addr,
        &ExecuteRedBank::InitAsset {
            denom: "uatom".to_string(),
            params: market_params,
        },
        &[],
        signer,
    )
    .unwrap();

    wasm.execute(
        &params_addr,
        &mars_types::params::ExecuteMsg::UpdateAssetParams(AssetParamsUpdate::AddOrUpdate {
            params: asset_params.into(),
        }),
        &[],
        signer,
    )
    .unwrap();

    (oracle_addr, red_bank_addr)
}
