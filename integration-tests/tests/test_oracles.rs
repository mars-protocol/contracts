use crate::helpers::osmosis::{assert_err, instantiate_contract};
use cosmwasm_std::{coin, Coin, Decimal, Isqrt, Uint128};
use mars_oracle_base::ContractError;
use mars_oracle_osmosis::msg::PriceSourceResponse;
use mars_oracle_osmosis::OsmosisPriceSource;
use mars_outpost::address_provider::InstantiateMsg as addr_instantiate;
use mars_outpost::oracle::{ExecuteMsg, InstantiateMsg, PriceResponse, QueryMsg};
use mars_outpost::red_bank::ExecuteMsg::Deposit;
use mars_outpost::red_bank::{CreateOrUpdateConfig, InstantiateMsg as red_bank_instantiate};
use mars_outpost::red_bank::{
    ExecuteMsg as execute_red_bank, InitOrUpdateAssetParams, InterestRateModel,
};
use osmosis_testing::{Account, Gamm, Module, OsmosisTestApp, Wasm};
use std::str::FromStr;

mod helpers;

const OSMOSIS_ORACLE_CONTRACT_NAME: &str = "mars-oracle-osmosis";
const OSMOSIS_RED_BANK_CONTRACT_NAME: &str = "mars-red-bank";
const OSMOSIS_ADDR_PROVIDER_CONTRACT_NAME: &str = "mars-address-provider";

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

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::SetPriceSource {
            denom: "umars_uatom_lp".to_string(),
            price_source: OsmosisPriceSource::XykLiquidityToken {
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
        &InstantiateMsg {
            owner: signer.address(),
            base_denom: "uosmo".to_string(),
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
        &ExecuteMsg::SetPriceSource {
            denom: "umars_uatom_lp".to_string(),
            price_source: OsmosisPriceSource::XykLiquidityToken {
                pool_id: pool_mars_atom,
            },
        },
        &[],
        &signer,
    )
    .unwrap();
    wasm.execute(
        &contract_addr,
        &ExecuteMsg::SetPriceSource {
            denom: "umars".to_string(),
            price_source: OsmosisPriceSource::Spot {
                pool_id: pool_mars_osmo,
            },
        },
        &[],
        &signer,
    )
    .unwrap();
    wasm.execute(
        &contract_addr,
        &ExecuteMsg::SetPriceSource {
            denom: "uatom".to_string(),
            price_source: OsmosisPriceSource::Spot {
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
            },
        )
        .unwrap();
    assert_eq!(res.price, lp_price);
}

// set osmo-atom liquidity pool, set oracle to SPOT, query price source, query price
#[test]
fn set_spot_price() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "uosmo"), coin(1_000_000_000_000, "uatom")])
        .unwrap();

    let oracle_addr = instantiate_contract(
        &wasm,
        &signer,
        OSMOSIS_ORACLE_CONTRACT_NAME,
        &InstantiateMsg {
            owner: signer.address(),
            base_denom: "uosmo".to_string(),
        },
    );

    let gamm = Gamm::new(&app);
    let pool_liquidity = vec![Coin::new(2_000_000, "uatom"), Coin::new(1_000_000, "uosmo")];
    let pool_id = gamm.create_basic_pool(&pool_liquidity, &signer).unwrap().data.pool_id;

    let osmo_atom_pool = gamm.query_pool(pool_id).unwrap().pool_assets;

    assert_eq!(
        pool_liquidity
            .into_iter()
            .map(|c| c.into())
            .collect::<Vec<osmosis_testing::osmosis_std::types::cosmos::base::v1beta1::Coin>>(),
        osmo_atom_pool
            .into_iter()
            .map(|a| a.token.unwrap())
            .collect::<Vec<osmosis_testing::osmosis_std::types::cosmos::base::v1beta1::Coin>>(),
    );

    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::SetPriceSource {
            denom: "uosmo".to_string(),
            price_source: OsmosisPriceSource::Spot {
                pool_id,
            },
        },
        &[],
        &signer,
    )
    .unwrap();

    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::SetPriceSource {
            denom: "uatom".to_string(),
            price_source: OsmosisPriceSource::Spot {
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
                denom: "uosmo".to_string(),
            },
        )
        .unwrap();

    assert_eq!(
        price_source.price_source,
        (OsmosisPriceSource::Spot {
            pool_id
        })
    );

    let price: PriceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::Price {
                denom: "uatom".to_string(),
            },
        )
        .unwrap();

    assert_eq!(price.price, Decimal::from_ratio(1u128, 2u128)); // 1 osmo = 2 atom

    assert_eq!(price.denom, "uatom".to_string());
}
// Set price source to spot for without creating a liquidity pool - should return an error
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
        &InstantiateMsg {
            owner: signer.address(),
            base_denom: "uosmo".to_string(),
        },
    );

    let pool_id: u64 = 1;

    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::SetPriceSource {
            denom: "uosmo".to_string(),
            price_source: OsmosisPriceSource::Spot {
                pool_id,
            },
        },
        &[],
        &signer,
    )
    .unwrap_err();
}

// set price source to spot and query the price of an asset that is not in the pool - should return an error
#[test]
fn incorrect_pool_for_spot_oracle() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "uosmo"), coin(1_000_000_000_000, "uatom")])
        .unwrap();

    let oracle_addr = instantiate_contract(
        &wasm,
        &signer,
        OSMOSIS_ORACLE_CONTRACT_NAME,
        &InstantiateMsg {
            owner: signer.address(),
            base_denom: "uosmo".to_string(),
        },
    );

    let gamm = Gamm::new(&app);
    let pool_liquidity = vec![Coin::new(2_000_000, "uatom"), Coin::new(1_000_000, "uosmo")];
    let pool_id = gamm.create_basic_pool(&pool_liquidity, &signer).unwrap().data.pool_id;

    let res = wasm
        .execute(
            &oracle_addr,
            &ExecuteMsg::SetPriceSource {
                denom: "umars".to_string(),
                price_source: OsmosisPriceSource::Spot {
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
fn test_different_prices() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "uosmo"), coin(1_000_000_000_000, "uatom")])
        .unwrap();

    let oracle_addr = instantiate_contract(
        &wasm,
        &signer,
        OSMOSIS_ORACLE_CONTRACT_NAME,
        &InstantiateMsg {
            owner: signer.address(),
            base_denom: "uosmo".to_string(),
        },
    );

    let gamm = Gamm::new(&app);
    let pool_liquidity = vec![Coin::new(98_000_000, "uatom"), Coin::new(1_764_000_000, "uosmo")];
    let pool_id = gamm.create_basic_pool(&pool_liquidity, &signer).unwrap().data.pool_id;

    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::SetPriceSource {
            denom: "uosmo".to_string(),
            price_source: OsmosisPriceSource::Spot {
                pool_id,
            },
        },
        &[],
        &signer,
    )
    .unwrap();

    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::SetPriceSource {
            denom: "uatom".to_string(),
            price_source: OsmosisPriceSource::Spot {
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
            },
        )
        .unwrap();

    assert_eq!(price.price, Decimal::from_ratio(1764u128, 98u128));

    let pool_liquidity = vec![Coin::new(13_000_000, "uatom"), Coin::new(78_000_000, "uosmo")];
    let pool_id = gamm.create_basic_pool(&pool_liquidity, &signer).unwrap().data.pool_id;

    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::SetPriceSource {
            denom: "uosmo".to_string(),
            price_source: OsmosisPriceSource::Spot {
                pool_id,
            },
        },
        &[],
        &signer,
    )
    .unwrap();

    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::SetPriceSource {
            denom: "uatom".to_string(),
            price_source: OsmosisPriceSource::Spot {
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
            },
        )
        .unwrap();

    assert_eq!(price.price, Decimal::from_ratio(78u128, 13u128));
}

//assert oracle was correctly set to TWAP and assert prices are queried correctly
#[test]
#[ignore] // FIXME: TWAP doesn't work on osmosis-testing - fix in progress
fn set_twap_price() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let signer = app
        .init_account(&[coin(1_000_000_000_000, "uosmo"), coin(1_000_000_000_000, "uatom")])
        .unwrap();

    let oracle_addr = instantiate_contract(
        &wasm,
        &signer,
        OSMOSIS_ORACLE_CONTRACT_NAME,
        &InstantiateMsg {
            owner: signer.address(),
            base_denom: "uosmo".to_string(),
        },
    );

    let gamm = Gamm::new(&app);
    let pool_liquidity = vec![Coin::new(1_000_000, "uatom"), Coin::new(1_000_000, "uosmo")];
    let pool_id = gamm.create_basic_pool(&pool_liquidity, &signer).unwrap().data.pool_id;

    let osmo_atom_pool = gamm.query_pool(pool_id).unwrap().pool_assets;

    assert_eq!(
        pool_liquidity
            .into_iter()
            .map(|c| c.into())
            .collect::<Vec<osmosis_testing::osmosis_std::types::cosmos::base::v1beta1::Coin>>(),
        osmo_atom_pool
            .into_iter()
            .map(|a| a.token.unwrap())
            .collect::<Vec<osmosis_testing::osmosis_std::types::cosmos::base::v1beta1::Coin>>(),
    );

    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::SetPriceSource {
            denom: "uosmo".to_string(),
            price_source: OsmosisPriceSource::Twap {
                pool_id,
                window_size: 1800, //30min
            },
        },
        &[],
        &signer,
    )
    .unwrap();

    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::SetPriceSource {
            denom: "uatom".to_string(),
            price_source: OsmosisPriceSource::Twap {
                pool_id,
                window_size: 1800, //30min
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
                denom: "uosmo".to_string(),
            },
        )
        .unwrap();

    assert_eq!(
        price_source.price_source,
        (OsmosisPriceSource::Twap {
            pool_id,
            window_size: 1800,
        })
    );

    let price: PriceResponse = wasm
        .query(
            &oracle_addr,
            &QueryMsg::Price {
                denom: "uatom".to_string(),
            },
        )
        .unwrap();

    assert_eq!(price.price, Decimal::from_ratio(1u128, 2u128)); // 1 osmo = 2 atom

    assert_eq!(price.denom, "uatom".to_string());
}
// execute borrow action in red bank with an asset not in the oracle - should fail when attempting to query oracle
#[test]
fn test_oracle_with_redbank() {
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

    let oracle_addr = instantiate_contract(
        &wasm,
        &signer,
        OSMOSIS_ORACLE_CONTRACT_NAME,
        &InstantiateMsg {
            owner: signer.address(),
            base_denom: "uosmo".to_string(),
        },
    );

    let addr_provider_addr = instantiate_contract(
        &wasm,
        &signer,
        OSMOSIS_ADDR_PROVIDER_CONTRACT_NAME,
        &addr_instantiate {
            owner: signer.address(),
            prefix: "osmo".to_string(),
        },
    );

    let red_bank_addr = instantiate_contract(
        &wasm,
        &signer,
        OSMOSIS_RED_BANK_CONTRACT_NAME,
        &red_bank_instantiate {
            config: CreateOrUpdateConfig {
                owner: Some(signer.address()),
                address_provider: Some(addr_provider_addr),
                close_factor: Some(Decimal::percent(10)),
            },
        },
    );

    wasm.execute(
        &red_bank_addr,
        &execute_red_bank::InitAsset {
            denom: "uosmo".to_string(),
            params: InitOrUpdateAssetParams {
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
            },
        },
        &[],
        &signer,
    )
    .unwrap();

    wasm.execute(
        &red_bank_addr,
        &execute_red_bank::InitAsset {
            denom: "uatom".to_string(),
            params: InitOrUpdateAssetParams {
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
            },
        },
        &[],
        &signer,
    )
    .unwrap();

    let gamm = Gamm::new(&app);
    let pool_liquidity = vec![Coin::new(2_000_000, "uatom"), Coin::new(1_000_000, "uosmo")];
    let pool_id = gamm.create_basic_pool(&pool_liquidity, &signer).unwrap().data.pool_id;

    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::SetPriceSource {
            denom: "uosmo".to_string(),
            price_source: OsmosisPriceSource::Spot {
                pool_id,
            },
        },
        &[],
        &signer,
    )
    .unwrap();

    wasm.execute(
        &oracle_addr,
        &ExecuteMsg::SetPriceSource {
            denom: "uatom".to_string(),
            price_source: OsmosisPriceSource::Spot {
                pool_id,
            },
        },
        &[],
        &signer,
    )
    .unwrap();

    wasm.execute(
        &red_bank_addr,
        &Deposit {
            on_behalf_of: None,
        },
        &[coin(1_000_000_000_000_000, "uosmo")],
        &depositor,
    )
    .unwrap_err();

    //failure due to insufficient funds
}
