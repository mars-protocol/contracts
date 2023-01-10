use std::mem::take;

use anyhow::Result as AnyResult;
use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{BasicApp, Executor};
use mars_account_nft::{
    config::ConfigUpdates,
    msg::{
        ExecuteMsg::{AcceptMinterRole, UpdateConfig},
        InstantiateMsg,
    },
};

use crate::helpers::{
    mock_credit_manager_contract, mock_nft_contract, MockEnv, MAX_VALUE_FOR_BURN,
};

pub struct MockEnvBuilder {
    pub app: BasicApp,
    pub minter: Option<Addr>,
    pub deployer: Addr,
    pub nft_contract: Option<Addr>,
}

impl MockEnvBuilder {
    pub fn build(&mut self) -> AnyResult<MockEnv> {
        Ok(MockEnv {
            minter: self.get_minter(),
            nft_contract: self.get_nft_contract(),
            deployer: self.deployer.clone(),
            app: take(&mut self.app),
        })
    }

    pub fn set_minter(&mut self, minter: &str) -> &mut Self {
        self.minter = Some(Addr::unchecked(minter.to_string()));
        self
    }

    fn get_minter(&mut self) -> Addr {
        self.minter.clone().unwrap_or_else(|| self.deployer.clone())
    }

    fn get_nft_contract(&mut self) -> Addr {
        if self.nft_contract.is_none() {
            self.deploy_nft_contract()
        }
        self.nft_contract.clone().unwrap()
    }

    fn deploy_nft_contract(&mut self) {
        let contract = mock_nft_contract();
        let code_id = self.app.store_code(contract);
        let minter = self.get_minter().into();

        let addr = self
            .app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &InstantiateMsg {
                    max_value_for_burn: MAX_VALUE_FOR_BURN,
                    name: "mock_nft".to_string(),
                    symbol: "MOCK".to_string(),
                    minter,
                },
                &[],
                "mock-account-nft",
                None,
            )
            .unwrap();
        self.nft_contract = Some(addr);
    }

    pub fn assign_minter_to_cm(&mut self) -> &mut Self {
        let contract = mock_credit_manager_contract();
        let code_id = self.app.store_code(contract);

        let cm_addr = self
            .app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &Empty {},
                &[],
                "mock-credit-manager",
                None,
            )
            .unwrap();

        let nft_contract = self.get_nft_contract();

        let minter = self.get_minter();

        // Propose new minter
        self.app
            .execute_contract(
                minter,
                nft_contract.clone(),
                &UpdateConfig {
                    updates: ConfigUpdates {
                        max_value_for_burn: None,
                        proposed_new_minter: Some(cm_addr.clone().into()),
                    },
                },
                &[],
            )
            .unwrap();

        // Accept new role
        self.app
            .execute_contract(cm_addr.clone(), nft_contract, &AcceptMinterRole {}, &[])
            .unwrap();

        self.minter = Some(cm_addr);
        self
    }
}
