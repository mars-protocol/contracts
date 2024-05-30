use std::str::FromStr;

use astroport::{
    factory::PairType,
    pair::{QueryMsg, StablePoolParams},
    pair_concentrated::{ConcentratedPoolParams, QueryMsg as ConcentratedPairQueryMsg},
};
use cosmwasm_std::{to_json_binary, Binary, Decimal, Decimal256, Empty, Uint128};
#[cfg(feature = "osmosis-test-tube")]
use cw_it::Artifact;
use cw_it::{
    astroport::{
        robot::AstroportTestRobot,
        utils::{native_asset, native_info, AstroportContracts},
    },
    robot::TestRobot,
    test_tube::{Account, Module, SigningAccount, Wasm},
    traits::CwItRunner,
    ContractMap, ContractType, TestRunner,
};
use mars_oracle_wasm::WasmPriceSourceUnchecked;
use mars_owner::OwnerUpdate;
use mars_types::oracle::{InstantiateMsg, WasmOracleCustomExecuteMsg, WasmOracleCustomInitParams};

use crate::test_runner::get_test_runner;

// Base denom to use in tests
pub const BASE_DENOM: &str = "USD";

/// The path to the artifacts folder
pub const ARTIFACTS_PATH: &str = "../../../artifacts";
pub const APPEND_ARCH: bool = false;

/// The path to the artifacts folder
pub const ASTRO_ARTIFACTS_PATH: Option<&str> = Some("tests/astroport-artifacts");

pub const ORACLE_CONTRACT_NAME: &str = "mars-oracle-wasm";

pub const STRIDE_ARTIFACTS_PATH: &str = "tests/stride-artifacts";
pub const STRIDE_ICA_ORACLE_CONTRACT_NAME: &str = "ica_oracle";
pub const STRIDE_TRANSFER_CHANNEL_ID: &str = "channel-8";

pub struct WasmOracleTestRobot<'a> {
    runner: &'a TestRunner<'a>,
    pub astroport_contracts: AstroportContracts,
    pub mars_oracle_contract_addr: String,
    pub stride_contract_addr: Option<String>,
}

impl<'a> WasmOracleTestRobot<'a> {
    pub fn new(
        runner: &'a TestRunner<'a>,
        contract_map: ContractMap,
        admin: &SigningAccount,
        base_denom: Option<&str>,
    ) -> Self {
        // Upload and instantiate contracts
        let (astroport_contracts, oracle_contract_addr, stride_contract_addr) =
            Self::upload_and_init_contracts(runner, contract_map, admin, base_denom);

        Self {
            runner,
            astroport_contracts,
            mars_oracle_contract_addr: oracle_contract_addr,
            stride_contract_addr,
        }
    }

    /// Uploads and instantiates all contracts needed for testing
    pub fn upload_and_init_contracts(
        runner: &'a TestRunner<'a>,
        contracts: ContractMap,
        admin: &SigningAccount,
        base_denom: Option<&str>,
    ) -> (AstroportContracts, String, Option<String>) {
        let admin_addr = admin.address();
        // Upload contracts
        let code_ids = cw_it::helpers::upload_wasm_files(runner, admin, contracts).unwrap();

        // Instantiate Astroport contracts
        let astroport_contracts =
            <WasmOracleTestRobot<'a> as AstroportTestRobot<TestRunner>>::instantiate_astroport_contracts(
                runner, admin, &code_ids,
            );

        let wasm = Wasm::new(runner);

        // Instantiate Mars Oracle Wasm contract
        let code_id = code_ids[ORACLE_CONTRACT_NAME];
        let oracle_init_msg = InstantiateMsg::<WasmOracleCustomInitParams> {
            owner: admin_addr.clone(),
            base_denom: base_denom.unwrap_or(BASE_DENOM).to_string(),
            custom_init: Some(WasmOracleCustomInitParams {
                astroport_factory: astroport_contracts.factory.address.clone(),
            }),
        };
        let oracle_init_res = wasm
            .instantiate(code_id, &oracle_init_msg, Some(&admin_addr), None, &[], admin)
            .unwrap();
        let oracle_contract_addr = oracle_init_res.data.address;

        // Instantiate Stride ICA oracle contract
        let stride_contract_addr =
            if let Some(&code_id) = code_ids.get(STRIDE_ICA_ORACLE_CONTRACT_NAME) {
                let stride_init_msg = ica_oracle::msg::InstantiateMsg {
                    admin_address: admin_addr.clone(),
                    transfer_channel_id: Some(STRIDE_TRANSFER_CHANNEL_ID.to_string()),
                };
                let stride_init_res = wasm
                    .instantiate(code_id, &stride_init_msg, Some(&admin_addr), None, &[], admin)
                    .unwrap();
                Some(stride_init_res.data.address)
            } else {
                None
            };

        (astroport_contracts, oracle_contract_addr, stride_contract_addr)
    }

    pub fn increase_time(&self, seconds: u64) -> &Self {
        self.runner.increase_time(seconds).unwrap();
        self
    }

    pub fn create_default_astro_pair(&self, signer: &SigningAccount) -> (String, String) {
        let initial_liq: [u128; 2] = [10000000000000000000000u128, 1000000000000000000000u128];
        self.create_astroport_pair(
            PairType::Xyk {},
            &[native_info("uatom"), native_info("uosmo")],
            None,
            signer,
            Some(&initial_liq),
            None,
        )
    }

    // ====== Price source methods ======

    pub fn set_price_source(
        &self,
        denom: &str,
        price_source: WasmPriceSourceUnchecked,
        signer: &SigningAccount,
    ) -> &Self {
        let msg = mars_types::oracle::ExecuteMsg::<_, Empty>::SetPriceSource {
            denom: denom.to_string(),
            price_source,
        };
        self.wasm().execute(&self.mars_oracle_contract_addr, &msg, &[], signer).unwrap();
        self
    }

    pub fn set_price_sources(
        &self,
        price_sources: Vec<(&str, WasmPriceSourceUnchecked)>,
        signer: &SigningAccount,
    ) -> &Self {
        for (denom, price_source) in price_sources {
            self.set_price_source(denom, price_source, signer);
        }
        self
    }

    pub fn remove_price_source(&self, signer: &SigningAccount, denom: &str) -> &Self {
        let msg = mars_types::oracle::ExecuteMsg::<Empty>::RemovePriceSource {
            denom: denom.to_string(),
        };
        self.wasm().execute(&self.mars_oracle_contract_addr, &msg, &[], signer).unwrap();
        self
    }

    pub fn query_price_sources(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Vec<mars_types::oracle::PriceSourceResponse<WasmPriceSourceUnchecked>> {
        let msg = &mars_types::oracle::QueryMsg::PriceSources {
            start_after,
            limit,
        };
        self.wasm().query(&self.mars_oracle_contract_addr, &msg).unwrap()
    }

    pub fn query_price_source(
        &self,
        denom: &str,
    ) -> mars_types::oracle::PriceSourceResponse<WasmPriceSourceUnchecked> {
        let msg = &mars_types::oracle::QueryMsg::PriceSource {
            denom: denom.to_string(),
        };
        self.wasm().query(&self.mars_oracle_contract_addr, &msg).unwrap()
    }

    pub fn query_price(&self, denom: &str) -> mars_types::oracle::PriceResponse {
        let msg = &mars_types::oracle::QueryMsg::Price {
            denom: denom.to_string(),
            kind: None,
        };
        self.wasm().query(&self.mars_oracle_contract_addr, &msg).unwrap()
    }

    pub fn query_prices(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Vec<mars_types::oracle::PriceResponse> {
        let msg = &mars_types::oracle::QueryMsg::Prices {
            start_after,
            limit,
            kind: None,
        };
        self.wasm().query(&self.mars_oracle_contract_addr, &msg).unwrap()
    }

    /// Queries the oracle price and asserts that it is equal to the expected price
    pub fn assert_price(&self, denom: &str, expected_price: Decimal) -> &Self {
        let price = self.query_price(denom);
        assert_eq!(price.price, expected_price);
        assert_eq!(price.denom, denom);
        self
    }

    /// Queries the oracle price and asserts that it is almost equal to the expected price, within
    /// the given tolerance in percent.
    pub fn assert_price_almost_equal(
        &self,
        denom: &str,
        expected_price: Decimal,
        tolerance: Decimal,
    ) -> &Self {
        let price = self.query_price(denom);
        assert_almost_equal(price.price, expected_price, tolerance);
        assert_eq!(price.denom, denom);
        self
    }

    pub fn assert_prices_almost_equal(
        &self,
        price0: Decimal,
        price1: Decimal,
        tolerance: Decimal,
    ) -> &Self {
        assert_almost_equal(price0, price1, tolerance);
        self
    }

    pub fn assert_price_source(
        &self,
        denom: &str,
        expected_price_source: WasmPriceSourceUnchecked,
    ) -> &Self {
        let price_sources = self.query_price_sources(None, None);
        let price_source =
            price_sources.iter().find(|ps| ps.denom == denom).expect("Price source not found");
        assert_eq!(price_source.price_source, expected_price_source);
        self
    }

    pub fn assert_price_source_not_exists(&self, denom: &str) -> &Self {
        let price_sources = self.query_price_sources(None, None);
        let price_source = price_sources.iter().find(|ps| ps.denom == denom);
        assert!(price_source.is_none());
        self
    }

    pub fn record_twap_snapshots(&self, denoms: &[&str], signer: &SigningAccount) -> &Self {
        let msg = &mars_types::oracle::ExecuteMsg::<Empty, WasmOracleCustomExecuteMsg>::Custom(
            WasmOracleCustomExecuteMsg::RecordTwapSnapshots {
                denoms: denoms.iter().map(|d| d.to_string()).collect(),
            },
        );
        self.wasm().execute(&self.mars_oracle_contract_addr, &msg, &[], signer).unwrap();
        self
    }
    pub fn query_price_via_simulation(&self, pair_addr: &str, denom: &str) -> Decimal {
        let decimals = self.query_native_coin_registry(denom).unwrap();
        let one: Uint128 = Uint128::from(10u128.pow(decimals as u32));
        let denominator = one * Uint128::from(10u128);

        let return_amount = self
            .query_simulate_swap(pair_addr, native_asset(denom, denominator), None)
            .return_amount;

        Decimal::from_ratio(return_amount, denominator)
    }

    // =====  Owner update methods ======

    pub fn owner_update(&self, update_msg: OwnerUpdate, signer: &SigningAccount) -> &Self {
        let msg = &mars_types::oracle::ExecuteMsg::<Empty>::UpdateOwner(update_msg);
        self.wasm().execute(&self.mars_oracle_contract_addr, &msg, &[], signer).unwrap();
        self
    }

    pub fn query_config(&self) -> mars_types::oracle::ConfigResponse {
        let msg = &mars_types::oracle::QueryMsg::Config {};
        self.wasm().query(&self.mars_oracle_contract_addr, &msg).unwrap()
    }

    pub fn assert_owner(&self, expected_owner: impl Into<String>) -> &Self {
        let config = self.query_config();
        assert_eq!(config.owner, Some(expected_owner.into()));
        self
    }

    pub fn assert_proposed_new_owner(&self, expected_proposed_owner: impl Into<String>) -> &Self {
        let config = self.query_config();
        assert_eq!(config.proposed_new_owner, Some(expected_proposed_owner.into()));
        self
    }

    // ====== Stride methods ======

    pub fn set_redemption_rate_metric(
        &self,
        denom: &str,
        value: Decimal,
        update_time: u64,
        block_height: u64,
        signer: &SigningAccount,
    ) -> &Self {
        let stride_contract_addr =
            self.stride_contract_addr.clone().expect("Stride ica oracle contract not found");

        let ica_attr = ica_oracle::state::RedemptionRateAttributes {
            sttoken_denom: denom.to_string(),
        };
        let binary_attr = to_json_binary(&ica_attr).expect("Failed to serialize Stride attributes");

        let msg = ica_oracle::msg::ExecuteMsg::PostMetric {
            key: format!("{}_redemption_rate", denom),
            value: value.to_string(),
            metric_type: ica_oracle::state::MetricType::RedemptionRate,
            update_time,
            block_height,
            attributes: Some(binary_attr),
        };
        self.wasm().execute(&stride_contract_addr, &msg, &[], signer).unwrap();
        self
    }

    pub fn query_redemption_rate(&self, denom: &str) -> ica_oracle::msg::RedemptionRateResponse {
        let stride_contract_addr =
            self.stride_contract_addr.clone().expect("Stride ica oracle contract not found");

        let msg = &ica_oracle::msg::QueryMsg::RedemptionRate {
            denom: denom.to_string(),
            params: None,
        };
        self.wasm().query(&stride_contract_addr, &msg).unwrap()
    }

    pub fn assert_redemption_rate(&self, denom: &str, expected_value: Decimal) -> &Self {
        let rr = self.query_redemption_rate(denom);
        assert_eq!(rr.redemption_rate, expected_value);
        self
    }

    pub fn query_pcl_curve_invariant(&self, pair_addr: &str) -> Decimal256 {
        self.wasm().query(pair_addr, &ConcentratedPairQueryMsg::ComputeD {}).unwrap()
    }

    pub fn query_ss_curve_invariant(&self, pair_addr: &str) -> Uint128 {
        self.wasm().query(pair_addr, &QueryMsg::QueryComputeD {}).unwrap()
    }

    pub fn query_astroport_config(&self, pair_addr: &str) -> astroport::pair::ConfigResponse {
        self.wasm().query(pair_addr, &ConcentratedPairQueryMsg::Config {}).unwrap()
    }
}

impl<'a> TestRobot<'a, TestRunner<'a>> for WasmOracleTestRobot<'a> {
    fn runner(&self) -> &'a TestRunner<'a> {
        self.runner
    }
}
impl<'a> AstroportTestRobot<'a, TestRunner<'a>> for WasmOracleTestRobot<'a> {
    fn astroport_contracts(&self) -> &AstroportContracts {
        &self.astroport_contracts
    }
}

/// Creates a test robot, initializes accounts, and uploads and instantiates contracts
pub fn setup_test<'a>(
    runner: &'a TestRunner<'a>,
    contract_map: ContractMap,
    admin: &SigningAccount,
    base_denom: Option<&str>,
) -> WasmOracleTestRobot<'a> {
    let robot = WasmOracleTestRobot::new(runner, contract_map, admin, base_denom);
    robot
}

pub fn get_wasm_oracle_contract(runner: &TestRunner) -> ContractType {
    match runner {
        #[cfg(feature = "osmosis-test-tube")]
        TestRunner::OsmosisTestApp(_) => {
            let contract_name = ORACLE_CONTRACT_NAME.replace("-", "_");
            let oracle_wasm_path = if APPEND_ARCH {
                format!("{}/{}-{}.wasm", ARTIFACTS_PATH, contract_name, std::env::consts::ARCH)
            } else {
                format!("{}/{}.wasm", ARTIFACTS_PATH, contract_name)
            };
            ContractType::Artifact(Artifact::Local(oracle_wasm_path))
        }
        TestRunner::MultiTest(_) => ContractType::MultiTestContract(Box::new(
            cw_it::cw_multi_test::ContractWrapper::new(
                mars_oracle_wasm::contract::entry::execute,
                mars_oracle_wasm::contract::entry::instantiate,
                mars_oracle_wasm::contract::entry::query,
            )
            .with_migrate(mars_oracle_wasm::contract::entry::migrate),
        )),
        _ => panic!("Unsupported test runner type"),
    }
}

pub fn get_stride_ica_oracle_contract(runner: &TestRunner) -> ContractType {
    match runner {
        #[cfg(feature = "osmosis-test-tube")]
        TestRunner::OsmosisTestApp(_) => {
            let oracle_wasm_path =
                format!("{}/{}.wasm", STRIDE_ARTIFACTS_PATH, STRIDE_ICA_ORACLE_CONTRACT_NAME);
            ContractType::Artifact(Artifact::Local(oracle_wasm_path))
        }
        TestRunner::MultiTest(_) => {
            ContractType::MultiTestContract(Box::new(cw_it::cw_multi_test::ContractWrapper::new(
                ica_oracle::contract::execute,
                ica_oracle::contract::instantiate,
                ica_oracle::contract::query,
            )))
        }
        _ => panic!("Unsupported test runner type"),
    }
}

/// Returns a HashMap of contracts to be used in the tests
pub fn get_contracts(runner: &TestRunner) -> ContractMap {
    // Get Astroport contracts
    let mut contracts =
        cw_it::astroport::utils::get_local_contracts(runner, &ASTRO_ARTIFACTS_PATH, false, &None);

    // Get Stride contract
    let stride_contract = get_stride_ica_oracle_contract(runner);
    contracts.insert(STRIDE_ICA_ORACLE_CONTRACT_NAME.to_string(), stride_contract);

    // Get Oracle contract
    let contract = get_wasm_oracle_contract(runner);
    contracts.insert(ORACLE_CONTRACT_NAME.to_string(), contract);

    contracts
}

/// Returns some default pair initialization params for the given pair type
pub fn astro_init_params(pair_type: &PairType) -> Option<Binary> {
    match pair_type {
        PairType::Xyk {} => None,
        PairType::Stable {} => Some(
            to_json_binary(&StablePoolParams {
                amp: 10,
                owner: None,
            })
            .unwrap(),
        ),
        PairType::Custom(custom) if custom == "concentrated" => Some(
            // {"amp":"500","gamma":"0.000001","mid_fee":"0.003","out_fee":"0.0045","fee_gamma":"0.01","repeg_profit_threshold":"0.00000001","min_price_scale_delta":"0.0000055","price_scale":"1.198144288063828944","ma_half_time":600,"track_asset_balances":false}
            to_json_binary(&ConcentratedPoolParams {
                amp: Decimal::from_atomics(500u128, 0).unwrap(),
                gamma: Decimal::from_atomics(1u128, 6).unwrap(),
                mid_fee: Decimal::from_atomics(3u128, 3).unwrap(),
                out_fee: Decimal::from_atomics(45u128, 4).unwrap(),
                fee_gamma: Decimal::from_atomics(1u128, 2).unwrap(),
                repeg_profit_threshold: Decimal::from_atomics(1u128, 8).unwrap(),
                min_price_scale_delta: Decimal::from_atomics(55u128, 7).unwrap(),
                price_scale: Decimal::from_str("1.198144288063828944").unwrap(),
                ma_half_time: 600u64,
                track_asset_balances: Some(false),
            })
            .unwrap(),
        ),
        _ => panic!("Unsupported pair type"),
    }
}

pub const fn fixed_source(price: Decimal) -> WasmPriceSourceUnchecked {
    WasmPriceSourceUnchecked::Fixed {
        price,
    }
}

/// Asserts that the difference between two Decimal values is less than `tolerance` percent
fn assert_almost_equal(a: Decimal, b: Decimal, tolerance: Decimal) {
    let diff = a.abs_diff(b);
    if a == Decimal::zero() {
        assert!(diff < tolerance);
    } else {
        assert!((diff / a) < tolerance);
    }
}

/// Tests that Astroport Spot Price Source validation and querying works as expected
pub fn validate_and_query_astroport_spot_price_source(
    pair_type: PairType,
    pair_denoms: &[&str; 2],
    base_denom: &str,
    other_asset_price: Option<Decimal>,
    initial_liq: &[u128; 2],
    register_second_price: bool,
    decimals: &[u8; 2],
) {
    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let admin = &runner.init_default_account().unwrap();
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, Some(base_denom));

    let (primary_denom, other_denom, primary_decimals) = if pair_denoms[0] == base_denom {
        (pair_denoms[1], pair_denoms[0], decimals[1])
    } else {
        (pair_denoms[0], pair_denoms[1], decimals[0])
    };

    let (pair_address, _lp_token_addr) = robot.create_astroport_pair(
        pair_type.clone(),
        &[native_info(primary_denom), native_info(other_denom)],
        astro_init_params(&pair_type),
        admin,
        Some(initial_liq),
        Some(decimals),
    );

    let price_source = WasmPriceSourceUnchecked::AstroportSpot {
        pair_address: pair_address.clone(),
    };
    let other_asset_price_source = if register_second_price {
        vec![(other_denom, fixed_source(other_asset_price.unwrap()))]
    } else {
        vec![]
    };

    // Oracle uses a swap simulation rather than just dividing the reserves, because we need to support non-XYK pools
    let one = Uint128::new(10_u128.pow(primary_decimals.into()));
    let sim_res = robot.query_simulate_swap(&pair_address, native_asset(primary_denom, one), None);
    let mut expected_price = Decimal::from_ratio(sim_res.return_amount, one);
    if let Some(other_asset_price) = other_asset_price {
        expected_price *= other_asset_price
    }

    // Set price sources and assert that the price is as expected
    robot
        .set_price_sources(other_asset_price_source, admin)
        .set_price_source(primary_denom, price_source.clone(), admin)
        .assert_price_source(primary_denom, price_source)
        .assert_price_almost_equal(primary_denom, expected_price, Decimal::percent(1));
}

/// Tests that Astroport TWAP Price Source validation and querying works as expected
pub fn validate_and_query_astroport_twap_price_source(
    pair_type: PairType,
    pair_denoms: &[&str; 2],
    base_denom: &str,
    other_asset_price: Option<Decimal>,
    register_second_price: bool,
    tolerance: u64,
    window_size: u64,
    initial_liq: &[u128; 2],
    decimals: &[u8; 2],
) {
    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let admin = &runner.init_default_account().unwrap();
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, Some(base_denom));

    let (primary_denom, other_denom) = if pair_denoms[0] == base_denom {
        (pair_denoms[1], pair_denoms[0])
    } else {
        (pair_denoms[0], pair_denoms[1])
    };

    let (pair_address, _lp_token_addr) = robot.create_astroport_pair(
        pair_type.clone(),
        &[native_info(primary_denom), native_info(other_denom)],
        astro_init_params(&pair_type),
        admin,
        Some(initial_liq),
        Some(decimals),
    );
    let initial_price = robot.query_price_via_simulation(&pair_address, primary_denom);

    let price_source = WasmPriceSourceUnchecked::AstroportTwap {
        pair_address: pair_address.clone(),
        tolerance,
        window_size,
    };
    let other_asset_price_source = if register_second_price {
        vec![(other_denom, fixed_source(other_asset_price.unwrap()))]
    } else {
        vec![]
    };

    println!("Swap amount: {}", initial_liq[1] / 1000000);

    let price_after_swap = robot
        .set_price_sources(other_asset_price_source, admin)
        .set_price_source(primary_denom, price_source.clone(), admin)
        .assert_price_source(primary_denom, price_source)
        .record_twap_snapshots(&[primary_denom], admin)
        .increase_time(window_size + tolerance)
        .swap_on_astroport_pair(
            &pair_address,
            native_asset(other_denom, initial_liq[1] / 1000000),
            None,
            None,
            Some(Decimal::from_ratio(1u128, 2u128)),
            admin,
        )
        .query_price_via_simulation(&pair_address, primary_denom);

    let price_precision: Uint128 = Uint128::from(10_u128.pow(8));
    let mut expected_price = Decimal::from_ratio(
        (initial_price + price_after_swap) * Decimal::from_ratio(1u128, 2u128) * price_precision,
        price_precision,
    );
    if let Some(other_asset_price) = other_asset_price {
        expected_price *= other_asset_price
    }

    robot
        .record_twap_snapshots(&[primary_denom], admin)
        .increase_time(window_size + tolerance)
        .assert_price_almost_equal(primary_denom, expected_price, Decimal::percent(1));
}
