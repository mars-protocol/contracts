use cosmwasm_std::{to_binary, Empty, WasmMsg};
use cw_it::{
    astroport::{robot::AstroportTestRobot, utils::AstroportContracts},
    multi_test::MultiTestRunner,
    osmosis_test_tube::{Account, Module, OsmosisTestApp, SigningAccount, Wasm},
    robot::TestRobot,
    Artifact, ContractMap, ContractType, TestRunner,
};
use mars_oracle::{InstantiateMsg, WasmOracleCustomInitParams};
use mars_oracle_wasm::WasmPriceSourceUnchecked;
use mars_owner::OwnerUpdate;
use serde::Serialize;

// Base denom to use in tests
pub const BASE_DENOM: &str = "USD";

/// The path to the artifacts folder
pub const ARTIFACTS_PATH: &str = "artifacts/";
pub const APPEND_ARCH: bool = true;

/// The path to the artifacts folder
pub const ASTRO_ARTIFACTS_PATH: Option<&str> = Some("tests/astroport-artifacts");

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");

const TEST_RUNNER: Option<&str> = option_env!("TEST_RUNNER");

/// Default test runner to use if TEST_RUNNER env var is not set
const DEFAULT_TEST_RUNNER: &str = "multi-test";

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
    ) -> Self {
        // Upload and instantiate contracts
        let (astroport_contracts, contract_addr) =
            Self::upload_and_init_contracts(runner, contract_map, admin);

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
        let code_id = code_ids[CONTRACT_NAME];
        let init_msg = InstantiateMsg::<WasmOracleCustomInitParams> {
            owner: admin_addr.clone(),
            base_denom: BASE_DENOM.to_string(),
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

    // ====== Price source methods ======

    pub fn set_price_source(
        &self,
        contract_addr: &str,
        admin: &SigningAccount,
        denom: &str,
        price_source: WasmPriceSourceUnchecked,
    ) -> &Self {
        let msg = mars_oracle::msg::ExecuteMsg::SetPriceSource {
            denom: denom.to_string(),
            price_source,
        };
        self.wasm().execute(contract_addr, &msg, &[], admin).unwrap();
        self
    }

    pub fn remove_price_source(&self, signer: &SigningAccount, denom: &str) -> &Self {
        let msg = mars_oracle::msg::ExecuteMsg::<Empty>::RemovePriceSource {
            denom: denom.to_string(),
        };
        self.wasm().execute(&self.mars_oracle_contract_addr, &msg, &[], signer).unwrap();
        self
    }

    pub fn query_price_sources(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Vec<mars_oracle::PriceSourceResponse<WasmPriceSourceUnchecked>> {
        let msg = &mars_oracle::msg::QueryMsg::PriceSources {
            start_after,
            limit,
        };
        self.wasm().query(&self.mars_oracle_contract_addr, &msg).unwrap()
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

    // =====  Owner update methods ======

    pub fn owner_update(&self, update_msg: OwnerUpdate, signer: &SigningAccount) -> &Self {
        let msg = &mars_oracle::msg::ExecuteMsg::<Empty>::UpdateOwner(update_msg);
        self.wasm().execute(&self.mars_oracle_contract_addr, &msg, &[], signer).unwrap();
        self
    }

    pub fn query_config(&self) -> mars_oracle::ConfigResponse {
        let msg = &mars_oracle::msg::QueryMsg::Config {};
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
impl<'a> AstroportTestRobot<'a, TestRunner<'a>> for WasmOracleTestRobot<'a> {}

/// Creates an OsmosisTestApp TestRunner
pub fn get_test_runner<'a>() -> TestRunner<'a> {
    match TEST_RUNNER.unwrap_or(DEFAULT_TEST_RUNNER) {
        "osmosis-test-tube" => {
            let app = OsmosisTestApp::new();
            TestRunner::OsmosisTestApp(app)
        }
        "multi-test" => TestRunner::MultiTest(MultiTestRunner::new("osmo")),
        _ => panic!("Unsupported test runner type"),
    }
}

/// Creates a test robot, initializes accounts, and uploads and instantiates contracts
pub fn setup_test<'a>(
    runner: &'a TestRunner<'a>,
    contract_map: ContractMap,
    admin: &SigningAccount,
) -> WasmOracleTestRobot<'a> {
    let robot = WasmOracleTestRobot::new(runner, contract_map, admin);
    robot
}

pub fn get_contracts(runner: &TestRunner) -> ContractMap {
    let mut contracts =
        cw_it::astroport::utils::get_local_contracts(runner, &ASTRO_ARTIFACTS_PATH, false, &None);

    let contract = match runner {
        TestRunner::OsmosisTestApp(_) => {
            let oracle_wasm_path = if APPEND_ARCH {
                format!(
                    "{}/{}-{}.wasm",
                    ARTIFACTS_PATH,
                    CONTRACT_NAME.replace('-', "_"),
                    std::env::consts::ARCH
                )
            } else {
                format!("{}/{}.wasm", ARTIFACTS_PATH, CONTRACT_NAME)
            };
            ContractType::Artifact(Artifact::Local(oracle_wasm_path))
        }
        TestRunner::MultiTest(_) => {
            ContractType::MultiTestContract(Box::new(cw_it::cw_multi_test::ContractWrapper::new(
                mars_oracle_wasm::contract::entry::execute,
                mars_oracle_wasm::contract::entry::instantiate,
                mars_oracle_wasm::contract::entry::query,
            )))
        }
        _ => panic!("Unsupported test runner type"),
    };

    contracts.insert(CONTRACT_NAME.to_string(), contract);

    contracts
}
