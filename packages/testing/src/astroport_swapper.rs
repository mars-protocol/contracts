use astroport_v5::router::SwapOperation;
use cosmwasm_std::{Coin, Uint128};
#[cfg(feature = "osmosis-test-tube")]
use cw_it::Artifact;
use cw_it::{
    astroport::{robot::AstroportTestRobot, utils::AstroportContracts},
    cw_multi_test::ContractWrapper,
    osmosis_std::types::cosmwasm::wasm::v1::MsgExecuteContractResponse,
    robot::TestRobot,
    test_tube::{Account, Module, RunnerExecuteResult, SigningAccount, Wasm},
    ContractMap, ContractType, TestRunner,
};
use mars_owner::OwnerResponse;
use mars_swapper_astroport::{config::AstroportConfig, route::AstroportRoute};
use mars_types::swapper::{
    EstimateExactInSwapResponse, RouteResponse, RoutesResponse, SwapperRoute,
};

use crate::wasm_oracle::{get_wasm_oracle_contract, WasmOracleTestRobot};

#[cfg(feature = "osmosis-test-tube")]
const CONTRACT_NAME: &str = "mars_swapper_astroport";

pub const ASTRO_ARTIFACTS_PATH: Option<&str> = Some("tests/astroport-artifacts");

const ARTIFACTS_PATH: &str = "../../../artifacts";
const APPEND_ARCH: bool = false;

#[cfg(feature = "osmosis-test-tube")]
fn get_swapper_wasm_path() -> String {
    wasm_path(ARTIFACTS_PATH, CONTRACT_NAME, APPEND_ARCH)
}

#[cfg(feature = "osmosis-test-tube")]
fn wasm_path(artifacts_path: &str, contract_name: &str, append_arch: bool) -> String {
    let contract_name = contract_name.replace("-", "_");
    if append_arch {
        format!("{}/{}-{}.wasm", artifacts_path, contract_name, std::env::consts::ARCH)
    } else {
        format!("{}/{}.wasm", artifacts_path, contract_name)
    }
}

fn get_local_swapper_contract(runner: &TestRunner) -> ContractType {
    match runner {
        #[cfg(feature = "osmosis-test-tube")]
        TestRunner::OsmosisTestApp(_) => {
            ContractType::Artifact(Artifact::Local(get_swapper_wasm_path()))
        }
        TestRunner::MultiTest(_) => {
            ContractType::MultiTestContract(Box::new(ContractWrapper::new(
                mars_swapper_astroport::contract::execute,
                mars_swapper_astroport::contract::instantiate,
                mars_swapper_astroport::contract::query,
            )))
        }
        _ => panic!("Unsupported test runner type"),
    }
}

pub struct AstroportSwapperRobot<'a> {
    pub runner: &'a TestRunner<'a>,
    /// The mars-swapper-astroport contract address
    pub swapper: String,
    /// The mars wasm oracle address
    pub oracle_robot: WasmOracleTestRobot<'a>,
}

impl<'a> TestRobot<'a, TestRunner<'a>> for AstroportSwapperRobot<'a> {
    fn runner(&self) -> &'a TestRunner<'a> {
        self.runner
    }
}

impl<'a> AstroportTestRobot<'a, TestRunner<'a>> for AstroportSwapperRobot<'a> {
    fn astroport_contracts(&self) -> &AstroportContracts {
        &self.oracle_robot.astroport_contracts
    }
}

impl<'a> AstroportSwapperRobot<'a> {
    /// Creates a new test robot with the given runner, contracts, and admin account.
    ///
    /// The contracts map must contain contracts for the following keys:
    /// - All contracts in the AstroportContracts struct
    /// - Mars swapper with key being the CARGO_PKG_NAME environment variable
    ///
    /// The contracts in the ContractMap must be compatible with the given TestRunner,
    /// else this function will panic.
    pub fn new(
        runner: &'a TestRunner,
        astroport_contracts: ContractMap,
        swapper_contract: ContractType,
        oracle_contract: ContractType,
        admin: &SigningAccount,
    ) -> Self {
        let mut contracts = astroport_contracts;
        contracts.insert("mars-oracle-wasm".to_string(), oracle_contract);
        let oracle_robot = WasmOracleTestRobot::new(runner, contracts, admin, Some("usd"));

        let swapper_code_id =
            cw_it::helpers::upload_wasm_file(runner, admin, swapper_contract).unwrap();

        let wasm = Wasm::new(runner);
        let swapper = wasm
            .instantiate(
                swapper_code_id,
                &mars_types::swapper::InstantiateMsg {
                    owner: admin.address(),
                },
                None,
                Some("swapper"),
                &[],
                admin,
            )
            .unwrap()
            .data
            .address;

        Self {
            runner,
            oracle_robot,
            swapper,
        }
    }

    pub fn new_with_local(runner: &'a TestRunner, admin: &SigningAccount) -> Self {
        let astroport_contracts = cw_it::astroport::utils::get_local_contracts(
            runner,
            &Some(ARTIFACTS_PATH),
            APPEND_ARCH,
            &Some(std::env::consts::ARCH),
        );
        let swapper_contract = get_local_swapper_contract(runner);
        let oracle_contract = get_wasm_oracle_contract(runner);
        Self::new(runner, astroport_contracts, swapper_contract, oracle_contract, admin)
    }

    pub fn set_config(&self, config: AstroportConfig, signer: &SigningAccount) -> &Self {
        self.wasm()
            .execute(
                &self.swapper,
                &mars_types::swapper::ExecuteMsg::<AstroportRoute, AstroportConfig>::UpdateConfig {
                    config,
                },
                &[],
                signer,
            )
            .unwrap();
        self
    }

    pub fn set_route(
        &self,
        operations: Vec<SwapOperation>,
        denom_in: impl Into<String>,
        denom_out: impl Into<String>,
        signer: &SigningAccount,
    ) -> &Self {
        self.wasm()
            .execute(
                &self.swapper,
                &mars_types::swapper::ExecuteMsg::<AstroportRoute, AstroportConfig>::SetRoute {
                    route: AstroportRoute {
                        operations,
                        router: self.astroport_contracts().router.address.clone(),
                        factory: self.astroport_contracts().factory.address.clone(),
                        oracle: self.oracle_robot.mars_oracle_contract_addr.clone(),
                    },
                    denom_in: denom_in.into(),
                    denom_out: denom_out.into(),
                },
                &[],
                signer,
            )
            .unwrap();
        self
    }

    pub fn swap(
        &self,
        coin_in: Coin,
        denom_out: impl Into<String>,
        min_receive: Uint128,
        signer: &SigningAccount,
        route: SwapperRoute,
    ) -> &Self {
        println!("swapping {}", coin_in);
        self.swap_res(coin_in, denom_out, min_receive, signer, route).unwrap();
        self
    }

    pub fn swap_res(
        &self,
        coin_in: Coin,
        denom_out: impl Into<String>,
        min_receive: Uint128,
        signer: &SigningAccount,
        route: SwapperRoute,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        println!("sending {} to swapper contract", coin_in);
        self.wasm().execute(
            &self.swapper,
            &mars_types::swapper::ExecuteMsg::<AstroportRoute, AstroportConfig>::SwapExactIn {
                coin_in: coin_in.clone(),
                denom_out: denom_out.into(),
                min_receive,
                route: Some(route),
            },
            &[coin_in],
            signer,
        )
    }

    pub fn query_config(&self) -> AstroportConfig {
        self.wasm()
            .query::<_, AstroportConfig>(&self.swapper, &mars_types::swapper::QueryMsg::Config {})
            .unwrap()
    }

    pub fn query_estimate_exact_in_swap(
        &self,
        coin_in: &Coin,
        denom_out: impl Into<String>,
        route: SwapperRoute,
    ) -> Uint128 {
        self.wasm()
            .query::<_, EstimateExactInSwapResponse>(
                &self.swapper,
                &mars_types::swapper::QueryMsg::EstimateExactInSwap {
                    coin_in: coin_in.clone(),
                    denom_out: denom_out.into(),
                    route: Some(route),
                },
            )
            .unwrap()
            .amount
    }

    pub fn query_route(
        &self,
        denom_in: impl Into<String>,
        denom_out: impl Into<String>,
    ) -> AstroportRoute {
        self.wasm()
            .query::<_, RouteResponse<AstroportRoute>>(
                &self.swapper,
                &mars_types::swapper::QueryMsg::Route {
                    denom_in: denom_in.into(),
                    denom_out: denom_out.into(),
                },
            )
            .unwrap()
            .route
    }

    pub fn query_routes(
        &self,
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    ) -> RoutesResponse<AstroportRoute> {
        self.wasm()
            .query::<_, RoutesResponse<AstroportRoute>>(
                &self.swapper,
                &mars_types::swapper::QueryMsg::Routes {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_owner(&self) -> Option<String> {
        self.wasm()
            .query::<_, OwnerResponse>(&self.swapper, &mars_types::swapper::QueryMsg::Owner {})
            .unwrap()
            .owner
    }

    pub fn assert_route(
        &self,
        denom_in: impl Into<String>,
        denom_out: impl Into<String>,
        operations: Vec<SwapOperation>,
    ) -> &Self {
        let route = self.query_route(denom_in, denom_out);
        assert_eq!(route.operations, operations);
        assert_eq!(route.router, self.astroport_contracts().router.address);
        assert_eq!(route.factory, self.astroport_contracts().factory.address);
        assert_eq!(route.oracle, self.oracle_robot.mars_oracle_contract_addr);
        self
    }
}
