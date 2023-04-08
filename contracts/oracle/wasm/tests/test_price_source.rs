use cw_it::{
    astroport::{robot::AstroportTestRobot, utils::AstroportContracts},
    multi_test::MultiTestRunner,
    osmosis_test_tube::{Account, Module, OsmosisTestApp, SigningAccount, Wasm},
    robot::TestRobot,
    Artifact, ContractMap, ContractType, TestRunner,
};
use mars_oracle::{InstantiateMsg, WasmOracleCustomInitParams};
use mars_oracle_wasm::WasmPriceSourceUnchecked;
use test_case::test_case;

// Base denom to use in tests
pub const BASE_DENOM: &str = "USD";

pub const ARTIFACTS_PATH: &str = "artifacts/";
pub const APPEND_ARCH: bool = true;

struct TestingRobot<'a> {
    runner: &'a TestRunner<'a>,
    astroport_contracts: AstroportContracts,
    mars_oracle_contract_addr: String,
    accs: Vec<SigningAccount>,
}
impl<'a> TestingRobot<'a> {
    fn new(runner: &'a TestRunner<'a>, contract_map: ContractMap) -> Self {
        // Initialize accounts
        let accs = runner.init_accounts();
        let admin = &accs[0];

        // Upload and instantiate contracts
        let (astroport_contracts, contract_addr) =
            Self::upload_and_init_contracts(runner, contract_map, admin);

        Self {
            runner,
            astroport_contracts,
            mars_oracle_contract_addr: contract_addr,
            accs,
        }
    }

    /// Uploads and instantiates all contracts needed for testing
    fn upload_and_init_contracts(
        runner: &'a TestRunner<'a>,
        contracts: ContractMap,
        admin: &SigningAccount,
    ) -> (AstroportContracts, String) {
        let admin_addr = admin.address().to_string();
        // Upload contracts
        let code_ids = cw_it::helpers::upload_wasm_files(runner, admin, contracts).unwrap();

        // Instantiate Astroport contracts
        let astroport_contracts =
            <TestingRobot<'a> as AstroportTestRobot<TestRunner>>::instantiate_astroport_contracts(
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

    fn set_price_source(
        &self,
        contract_addr: &str,
        admin: &SigningAccount,
        denom: &str,
        price_source: WasmPriceSourceUnchecked,
    ) {
        let msg = mars_oracle::msg::ExecuteMsg::SetPriceSource {
            denom: denom.to_string(),
            price_source,
        };
        self.wasm().execute(contract_addr, &msg, &[], admin).unwrap();
    }
}
impl<'a> TestRobot<'a, TestRunner<'a>> for TestingRobot<'a> {
    fn runner(&self) -> &'a TestRunner<'a> {
        &self.runner
    }
}
impl<'a> AstroportTestRobot<'a, TestRunner<'a>> for TestingRobot<'a> {}

/// The path to the artifacts folder
pub const ASTRO_ARTIFACTS_PATH: Option<&str> = Some("tests/astroport-artifacts");

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");

const TEST_RUNNER_TYPE: Option<&str> = option_env!("TEST_RUNNER_TYPE");

/// Creates an OsmosisTestApp TestRunner
fn get_test_runner<'a>() -> TestRunner<'a> {
    match TEST_RUNNER_TYPE.unwrap_or("multi-test") {
        "osmosis-test-tube" => {
            let app = OsmosisTestApp::new();
            TestRunner::OsmosisTestApp(app)
        }
        "multi-test" => TestRunner::MultiTest(MultiTestRunner::new("osmo")),
        _ => panic!("Unsupported test runner type"),
    }
}

/// Creates a test robot, initializes accounts, and uploads and instantiates contracts
fn setup_test<'a>(runner: &'a TestRunner<'a>, contract_map: ContractMap) -> TestingRobot<'a> {
    let robot = TestingRobot::new(runner, contract_map);
    robot
}

fn get_contracts(runner: &TestRunner) -> ContractMap {
    let mut contracts =
        cw_it::astroport::utils::get_local_contracts(runner, &ASTRO_ARTIFACTS_PATH, false, &None);

    let contract = match runner {
        TestRunner::OsmosisTestApp(_) => {
            let oracle_wasm_path = if APPEND_ARCH {
                format!(
                    "{}/{}-{}.wasm",
                    ARTIFACTS_PATH,
                    CONTRACT_NAME.replace("-", "_"),
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

#[test]
fn test_contract_initialization() {
    let runner = get_test_runner();
    let contract_map = get_contracts(&runner);
    setup_test(&runner, contract_map);
}

#[test_case(get_test_runner(), "uusd", WasmPriceSourceUnchecked::Fixed { price: cosmwasm_std::Decimal::one()})]
fn test_set_price_source(runner: TestRunner, denom: &str, price_source: WasmPriceSourceUnchecked) {
    let contract_map = get_contracts(&runner);
    let robot = setup_test(&runner, contract_map);
    let admin = &robot.accs[0];

    // Execute SetPriceSource
    robot.set_price_source(&robot.mars_oracle_contract_addr, admin, denom, price_source);
}
