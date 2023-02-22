use std::mem::take;

use anyhow::Result as AnyResult;
use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{BasicApp, Executor};
use mars_account_nft::msg::InstantiateMsg;

use crate::helpers::{mock_health_contract, mock_nft_contract, MockEnv, MAX_VALUE_FOR_BURN};

pub struct MockEnvBuilder {
    pub app: BasicApp,
    pub deployer: Addr,
    pub minter: Option<Addr>,
    pub health_contract: Option<Addr>,
    pub nft_contract: Option<Addr>,
    pub set_health_contract: bool,
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

    pub fn instantiate_with_health_contract(&mut self, bool: bool) -> &mut Self {
        self.set_health_contract = bool;
        self
    }

    pub fn set_minter(&mut self, minter: &str) -> &mut Self {
        self.minter = Some(Addr::unchecked(minter.to_string()));
        self
    }

    pub fn set_health_contract(&mut self, contract_addr: &str) -> &mut Self {
        self.health_contract = Some(Addr::unchecked(contract_addr.to_string()));
        self
    }

    fn get_health_contract(&mut self) -> Addr {
        if self.health_contract.is_none() {
            return self.deploy_health_contract();
        }
        self.health_contract.clone().unwrap()
    }

    fn deploy_health_contract(&mut self) -> Addr {
        let contract = mock_health_contract();
        let code_id = self.app.store_code(contract);

        let health_contract = self
            .app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &Empty {},
                &[],
                "mock-health-contract",
                None,
            )
            .unwrap();
        self.health_contract = Some(health_contract.clone());
        health_contract
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
        let health_contract = if self.set_health_contract {
            Some(self.get_health_contract().into())
        } else {
            None
        };

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
                    health_contract,
                },
                &[],
                "mock-account-nft",
                None,
            )
            .unwrap();
        self.nft_contract = Some(addr);
    }
}
