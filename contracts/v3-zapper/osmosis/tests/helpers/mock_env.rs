use std::{mem::take, str::FromStr};

use anyhow::Result as AnyResult;
use cosmwasm_std::coin;
use mars_owner::{OwnerResponse, OwnerUpdate};
use mars_v3_zapper_base::msg::{CallbackMsg, ExecuteMsg, InstantiateMsg, QueryMsg};
use osmosis_std::types::{
    cosmos::bank::v1beta1::QueryBalanceRequest,
    cosmwasm::wasm::v1::MsgExecuteContractResponse,
    osmosis::{
        concentratedliquidity,
        concentratedliquidity::v1beta1::{
            CreateConcentratedLiquidityPoolsProposal, FullPositionBreakdown, PoolRecord,
            PoolsRequest, UserPositionsRequest,
        },
    },
};
use osmosis_test_tube::{
    cosmrs::proto::prost::Message, Account, Bank, ConcentratedLiquidity, GovWithAppAccess, Module,
    OsmosisTestApp, RunnerExecuteResult, SigningAccount, Wasm,
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const DEFAULT_STARTING_BALANCE: u128 = 1_000_000_000_000;

pub const ATOM: &str = "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2";
pub const DAI: &str = "ibc/0CD3A0285E1341859B5E86B6AB7682F023D03E97607CCC1DC95706411D866DF7";

pub struct MockEnv {
    pub app: OsmosisTestApp,
    pub owner: SigningAccount,
    pub zapper: String,
}

pub struct MockEnvBuilder {
    pub app: OsmosisTestApp,
}

#[allow(clippy::new_ret_no_self)]
impl MockEnv {
    pub fn new() -> MockEnvBuilder {
        MockEnvBuilder {
            app: OsmosisTestApp::new(),
        }
    }

    //--------------------------------------------------------------------------------------------------
    // Execute Msgs
    //--------------------------------------------------------------------------------------------------
    pub fn update_owner(
        &mut self,
        update: OwnerUpdate,
        alt_signer: Option<&SigningAccount>,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        let wasm = Wasm::new(&self.app);
        wasm.execute(
            &self.zapper,
            &ExecuteMsg::UpdateOwner(update),
            &[coin(5000, "uosmo")],
            alt_signer.unwrap_or(&self.owner),
        )
    }

    pub fn create_pool(&mut self, denom0: &str, denom1: &str) -> u64 {
        let cl = ConcentratedLiquidity::new(&self.app);
        let gov = GovWithAppAccess::new(&self.app);

        gov.propose_and_execute(
            CreateConcentratedLiquidityPoolsProposal::TYPE_URL.to_string(),
            CreateConcentratedLiquidityPoolsProposal {
                title: String::from("test"),
                description: String::from("test"),
                pool_records: vec![PoolRecord {
                    denom0: denom0.to_string(),
                    denom1: denom1.to_string(),
                    tick_spacing: 1,
                    exponent_at_price_one: "-4".to_string(),
                    spread_factor: "500000000000000".to_string(),
                }],
            },
            self.owner.address(),
            false,
            &self.owner,
        )
        .unwrap();

        let pools = cl
            .query_pools(&PoolsRequest {
                pagination: None,
            })
            .unwrap()
            .pools;

        concentratedliquidity::v1beta1::Pool::decode(pools.last().unwrap().value.as_slice())
            .unwrap()
            .id
    }

    pub fn callback(
        &mut self,
        msg: CallbackMsg,
        alt_signer: Option<&SigningAccount>,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        let wasm = Wasm::new(&self.app);
        wasm.execute(
            &self.zapper,
            &ExecuteMsg::Callback(msg),
            &[coin(5000, "uosmo")],
            alt_signer.unwrap_or(&self.owner),
        )
    }

    //--------------------------------------------------------------------------------------------------
    // Queries
    //--------------------------------------------------------------------------------------------------
    pub fn query_ownership(&self) -> OwnerResponse {
        let wasm = Wasm::new(&self.app);
        wasm.query(&self.zapper, &QueryMsg::Owner {}).unwrap()
    }

    pub fn query_balance(&self, address: &str, denom: &str) -> u128 {
        let bank = Bank::new(&self.app);
        let str_balance = bank
            .query_balance(&QueryBalanceRequest {
                address: address.to_string(),
                denom: denom.to_string(),
            })
            .unwrap()
            .balance
            .unwrap()
            .amount;
        u128::from_str(&str_balance).unwrap()
    }

    pub fn query_positions(&self, pool_id: u64) -> Vec<FullPositionBreakdown> {
        let cl = ConcentratedLiquidity::new(&self.app);
        let res = cl
            .query_user_positions(&UserPositionsRequest {
                address: self.zapper.clone(),
                pool_id,
                pagination: None,
            })
            .unwrap();
        res.positions
    }
}

impl MockEnvBuilder {
    pub fn build(&mut self) -> AnyResult<MockEnv> {
        let owner = self
            .app
            .init_account(&[
                coin(DEFAULT_STARTING_BALANCE, "uosmo"),
                coin(DEFAULT_STARTING_BALANCE, ATOM),
                coin(DEFAULT_STARTING_BALANCE, DAI),
            ])
            .unwrap();
        let zapper = self.instantiate_contract(&owner);

        Ok(MockEnv {
            app: take(&mut self.app),
            owner,
            zapper,
        })
    }

    pub fn wasm_file(&self) -> Vec<u8> {
        let artifacts_dir =
            std::env::var("ARTIFACTS_DIR_PATH").unwrap_or_else(|_| "artifacts".to_string());
        let snaked_name = CONTRACT_NAME.replace('-', "_");
        let relative_dir = format!("../../../{artifacts_dir}");

        let wasm_file_path = format!("{relative_dir}/{snaked_name}.wasm");

        match std::fs::read(wasm_file_path.clone()) {
            Ok(bytes) => {
                println!("{wasm_file_path}");
                bytes
            }
            // Retry if in arch64 environment
            Err(_) => {
                let alt_file_path = format!("{relative_dir}/{snaked_name}-aarch64.wasm");
                println!("{}", alt_file_path);
                std::fs::read(alt_file_path).unwrap()
            }
        }
    }

    pub fn instantiate_contract(&mut self, owner: &SigningAccount) -> String {
        let wasm = Wasm::new(&self.app);
        let wasm_byte_code = self.wasm_file();
        let code_id = wasm.store_code(&wasm_byte_code, None, owner).unwrap().data.code_id;

        wasm.instantiate(
            code_id,
            &InstantiateMsg {
                owner: owner.address(),
            },
            None,
            Some("v3-zapper-osmosis-contract"),
            &[],
            owner,
        )
        .unwrap()
        .data
        .address
    }
}
