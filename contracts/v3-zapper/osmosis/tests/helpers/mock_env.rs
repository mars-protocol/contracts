use std::{mem::take, str::FromStr};

use anyhow::Result as AnyResult;
use cosmwasm_std::{coin, Coin};
use mars_owner::{OwnerResponse, OwnerUpdate};
use mars_v3_zapper_base::msg::{CallbackMsg, ExecuteMsg, InstantiateMsg, QueryMsg};
use osmosis_std::types::osmosis::{
    concentratedliquidity::v1beta1::{
        MsgCreateConcentratedPool, PositionWithUnderlyingAssetBreakdown, QueryUserPositionsRequest,
    },
    tokenfactory::v1beta1::{MsgCreateDenom, MsgMint},
};
use osmosis_test_tube::{
    cosmrs::proto::{
        cosmos::bank::v1beta1::QueryBalanceRequest, cosmwasm::wasm::v1::MsgExecuteContractResponse,
    },
    Account, Bank, ConcentratedLiquidity, Module, OsmosisTestApp, RunnerExecuteResult,
    SigningAccount, TokenFactory, Wasm,
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");

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

    pub fn create_pool(&mut self, subdenom0: &str, subdenom1: &str) -> (String, String, u64) {
        let cl = ConcentratedLiquidity::new(&self.app);
        let token_factory = TokenFactory::new(&self.app);

        let denom0 = token_factory
            .create_denom(
                MsgCreateDenom {
                    sender: self.owner.address(),
                    subdenom: subdenom0.to_string(),
                },
                &self.owner,
            )
            .unwrap()
            .data
            .new_token_denom;

        let denom1 = token_factory
            .create_denom(
                MsgCreateDenom {
                    sender: self.owner.address(),
                    subdenom: subdenom1.to_string(),
                },
                &self.owner,
            )
            .unwrap()
            .data
            .new_token_denom;

        token_factory
            .mint(
                MsgMint {
                    sender: self.owner.address(),
                    amount: Some(Coin::new(100_000_000_000, &denom0).into()),
                    mint_to_address: self.owner.address(),
                },
                &self.owner,
            )
            .unwrap();

        token_factory
            .mint(
                MsgMint {
                    sender: self.owner.address(),
                    amount: Some(Coin::new(100_000_000_000, &denom1).into()),
                    mint_to_address: self.owner.address(),
                },
                &self.owner,
            )
            .unwrap();

        let pool_id = cl
            .create_concentrated_pool(
                MsgCreateConcentratedPool {
                    sender: self.owner.address(),
                    denom0: denom0.clone(),
                    denom1: denom1.clone(),
                    tick_spacing: 1,
                    exponent_at_price_one: "-4".to_string(),
                    swap_fee: "0".to_string(),
                },
                &self.owner,
            )
            .unwrap()
            .data
            .pool_id;

        (denom0, denom1, pool_id)
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

    pub fn query_positions(&self, pool_id: u64) -> Vec<PositionWithUnderlyingAssetBreakdown> {
        let cl = ConcentratedLiquidity::new(&self.app);
        let res = cl
            .query_user_positions(&QueryUserPositionsRequest {
                address: self.zapper.clone(),
                pool_id,
            })
            .unwrap();
        res.positions
    }
}

impl MockEnvBuilder {
    pub fn build(&mut self) -> AnyResult<MockEnv> {
        let owner = self.app.init_account(&[coin(1_000_000_000_000, "uosmo")]).unwrap();
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
