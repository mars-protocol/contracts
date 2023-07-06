use astroport::{factory::PairType, pair::StablePoolParams};
use cosmwasm_std::{to_binary, Binary, Decimal, Empty, Uint128};
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
use mars_red_bank_types::oracle::{
    InstantiateMsg, WasmOracleCustomExecuteMsg, WasmOracleCustomInitParams,
};

use crate::test_runner::get_test_runner;

// Base denom to use in tests
pub const BASE_DENOM: &str = "USD";

/// The path to the artifacts folder
pub const ARTIFACTS_PATH: &str = "../../../artifacts";
pub const APPEND_ARCH: bool = false;

/// The path to the artifacts folder
pub const ASTRO_ARTIFACTS_PATH: Option<&str> = Some("tests/astroport-artifacts");

pub const ORACLE_CONTRACT_NAME: &str = "mars-oracle-wasm";

pub struct WasmOracleTestRobot<'a> {
    runner: &'a TestRunner<'a>,
    pub astroport_contracts: AstroportContracts,
    pub mars_oracle_contract_addr: String,
}

impl<'a> WasmOracleTestRobot<'a> {
    pub fn new(
        runner: &'a TestRunner<'a>,
        contract_map: ContractMap,
        admin: &SigningAccount,
        base_denom: Option<&str>,
    ) -> Self {
        // Upload and instantiate contracts
        let (astroport_contracts, contract_addr) =
            Self::upload_and_init_contracts(runner, contract_map, admin, base_denom);

        Self {
            runner,
            astroport_contracts,
            mars_oracle_contract_addr: contract_addr,
        }
    }

    /// Uploads and instantiates all contracts needed for testing
    pub fn upload_and_init_contracts(
        runner: &'a TestRunner<'a>,
        contracts: ContractMap,
        admin: &SigningAccount,
        base_denom: Option<&str>,
    ) -> (AstroportContracts, String) {
        let admin_addr = admin.address();
        // Upload contracts
        let code_ids = cw_it::helpers::upload_wasm_files(runner, admin, contracts).unwrap();

        // Instantiate Astroport contracts
        let astroport_contracts =
            <WasmOracleTestRobot<'a> as AstroportTestRobot<TestRunner>>::instantiate_astroport_contracts(
                runner, admin, &code_ids,
            );

        // Instantiate Mars Oracle Wasm contract
        let code_id = code_ids[ORACLE_CONTRACT_NAME];
        let init_msg = InstantiateMsg::<WasmOracleCustomInitParams> {
            owner: admin_addr.clone(),
            base_denom: base_denom.unwrap_or(BASE_DENOM).to_string(),
            custom_init: Some(WasmOracleCustomInitParams {
                astroport_factory: astroport_contracts.factory.address.clone(),
            }),
        };
        let wasm = Wasm::new(runner);
        let init_res =
            wasm.instantiate(code_id, &init_msg, Some(&admin_addr), None, &[], admin).unwrap();

        let contract_addr = init_res.data.address;

        (astroport_contracts, contract_addr)
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
        let msg = mars_red_bank_types::oracle::msg::ExecuteMsg::<_, Empty>::SetPriceSource {
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
        let msg = mars_red_bank_types::oracle::msg::ExecuteMsg::<Empty>::RemovePriceSource {
            denom: denom.to_string(),
        };
        self.wasm().execute(&self.mars_oracle_contract_addr, &msg, &[], signer).unwrap();
        self
    }

    pub fn query_price_sources(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Vec<mars_red_bank_types::oracle::PriceSourceResponse<WasmPriceSourceUnchecked>> {
        let msg = &mars_red_bank_types::oracle::msg::QueryMsg::PriceSources {
            start_after,
            limit,
        };
        self.wasm().query(&self.mars_oracle_contract_addr, &msg).unwrap()
    }

    pub fn query_price_source(
        &self,
        denom: &str,
    ) -> mars_red_bank_types::oracle::PriceSourceResponse<WasmPriceSourceUnchecked> {
        let msg = &mars_red_bank_types::oracle::msg::QueryMsg::PriceSource {
            denom: denom.to_string(),
        };
        self.wasm().query(&self.mars_oracle_contract_addr, &msg).unwrap()
    }

    pub fn query_price(&self, denom: &str) -> mars_red_bank_types::oracle::PriceResponse {
        let msg = &mars_red_bank_types::oracle::msg::QueryMsg::Price {
            denom: denom.to_string(),
        };
        self.wasm().query(&self.mars_oracle_contract_addr, &msg).unwrap()
    }

    pub fn query_prices(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Vec<mars_red_bank_types::oracle::PriceResponse> {
        let msg = &mars_red_bank_types::oracle::msg::QueryMsg::Prices {
            start_after,
            limit,
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
        let msg = &mars_red_bank_types::oracle::msg::ExecuteMsg::<Empty, WasmOracleCustomExecuteMsg>::Custom(
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
        let msg = &mars_red_bank_types::oracle::msg::ExecuteMsg::<Empty>::UpdateOwner(update_msg);
        self.wasm().execute(&self.mars_oracle_contract_addr, &msg, &[], signer).unwrap();
        self
    }

    pub fn query_config(&self) -> mars_red_bank_types::oracle::ConfigResponse {
        let msg = &mars_red_bank_types::oracle::msg::QueryMsg::Config {};
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

/// Returns a HashMap of contracts to be used in the tests
pub fn get_contracts(runner: &TestRunner) -> ContractMap {
    // Get Astroport contracts
    let mut contracts =
        cw_it::astroport::utils::get_local_contracts(runner, &ASTRO_ARTIFACTS_PATH, false, &None);

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
            to_binary(&StablePoolParams {
                amp: 10,
                owner: None,
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
    route_prices: &[(&str, Decimal)],
    initial_liq: &[u128; 2],
    register_routes: bool,
) {
    let runner = get_test_runner();
    let admin = &runner.init_default_account().unwrap();
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, Some(base_denom));

    let (pair_address, _lp_token_addr) = robot.create_astroport_pair(
        pair_type.clone(),
        &[native_info(pair_denoms[0]), native_info(pair_denoms[1])],
        astro_init_params(&pair_type),
        admin,
        Some(initial_liq),
        Some(&[6, 6]), //TODO: make this configurable
    );

    let price_source = WasmPriceSourceUnchecked::AstroportSpot {
        pair_address: pair_address.clone(),
        route_assets: route_prices.iter().map(|&(s, _)| s.to_string()).collect(),
    };
    let route_price_sources: Vec<_> = if register_routes {
        route_prices.iter().map(|&(s, p)| (s, fixed_source(p))).collect()
    } else {
        vec![]
    };

    // Oracle uses a swap simulation rather than just dividing the reserves, because we need to support non-XYK pools
    let sim_res =
        robot.query_simulate_swap(&pair_address, native_asset(pair_denoms[0], 1000000u128), None);
    let expected_price = route_prices
        .iter()
        .fold(Decimal::from_ratio(sim_res.return_amount, 1000000u128), |acc, &(_, p)| acc * p);

    // Execute SetPriceSource
    robot
        .add_denom_precision_to_coin_registry(pair_denoms[0], 6, admin)
        .add_denom_precision_to_coin_registry(pair_denoms[1], 6, admin)
        .add_denom_precision_to_coin_registry(base_denom, 6, admin)
        .set_price_sources(route_price_sources, admin)
        .set_price_source(pair_denoms[0], price_source.clone(), admin)
        .assert_price_source(pair_denoms[0], price_source)
        .assert_price_almost_equal(pair_denoms[0], expected_price, Decimal::permille(1));
}

/// Tests that Astroport TWAP Price Source validation and querying works as expected
pub fn validate_and_query_astroport_twap_price_source(
    pair_type: PairType,
    pair_denoms: &[&str; 2],
    base_denom: &str,
    route_prices: &[(&str, Decimal)],
    tolerance: u64,
    window_size: u64,
    initial_liq: &[u128; 2],
) {
    let runner = get_test_runner();
    let admin = &runner.init_default_account().unwrap();
    let robot = WasmOracleTestRobot::new(&runner, get_contracts(&runner), admin, Some(base_denom));

    let (pair_address, _lp_token_addr) = robot.create_astroport_pair(
        pair_type.clone(),
        &[native_info(pair_denoms[0]), native_info(pair_denoms[1])],
        astro_init_params(&pair_type),
        admin,
        Some(initial_liq),
        Some(&[6, 6]), //TODO: make this configurable
    );
    let initial_price = robot
        .add_denom_precision_to_coin_registry(pair_denoms[0], 6, admin)
        .add_denom_precision_to_coin_registry(pair_denoms[1], 6, admin)
        .add_denom_precision_to_coin_registry(base_denom, 6, admin)
        .query_price_via_simulation(&pair_address, pair_denoms[0]);

    let price_source = WasmPriceSourceUnchecked::AstroportTwap {
        pair_address: pair_address.clone(),
        route_assets: route_prices.iter().map(|&(s, _)| s.to_string()).collect(),
        tolerance,
        window_size,
    };
    let route_price_sources: Vec<_> =
        route_prices.iter().map(|&(s, p)| (s, fixed_source(p))).collect();

    println!("Swap amount: {}", initial_liq[1] / 1000000);

    let price_after_swap = robot
        .set_price_sources(route_price_sources, admin)
        .set_price_source(pair_denoms[0], price_source.clone(), admin)
        .assert_price_source(pair_denoms[0], price_source)
        .record_twap_snapshots(&[pair_denoms[0]], admin)
        .increase_time(window_size + tolerance)
        .swap_on_astroport_pair(
            &pair_address,
            native_asset(pair_denoms[1], initial_liq[1] / 1000000),
            None,
            None,
            Some(Decimal::from_ratio(1u128, 2u128)),
            admin,
        )
        .query_price_via_simulation(&pair_address, pair_denoms[0]);

    let price_precision: Uint128 = Uint128::from(10_u128.pow(8));
    let expected_price = Decimal::from_ratio(
        (initial_price + price_after_swap) * Decimal::from_ratio(1u128, 2u128) * price_precision,
        price_precision,
    );
    let expected_price = route_prices.iter().fold(expected_price, |acc, &(_, p)| acc * p);

    robot
        .record_twap_snapshots(&[pair_denoms[0]], admin)
        .increase_time(window_size + tolerance)
        .assert_price_almost_equal(pair_denoms[0], expected_price, Decimal::percent(1));
}
