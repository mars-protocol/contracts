use std::mem::take;

use anyhow::Result as AnyResult;
use cosmwasm_std::{coin, Addr, Decimal};
use cw_multi_test::{BasicApp, Executor};
use cw_utils::Duration;
use mars_mock_credit_manager::msg::{
    ExecuteMsg::SetVaultConfig, InstantiateMsg as CmMockInstantiateMsg,
};
use mars_mock_oracle::msg::InstantiateMsg as OracleInstantiateMsg;
use mars_mock_red_bank::msg::InstantiateMsg as RedBankInstantiateMsg;
use mars_mock_vault::msg::InstantiateMsg as VaultInstantiateMsg;
use mars_owner::OwnerResponse;
use mars_rover::{
    adapters::{oracle::OracleUnchecked, vault::VaultConfig},
    msg::query::ConfigResponse,
};
use mars_rover_health_types::{ExecuteMsg::UpdateConfig, InstantiateMsg};

use crate::helpers::{
    mock_credit_manager_contract, mock_health_contract, mock_oracle_contract,
    mock_red_bank_contract, mock_vault_contract, MockEnv,
};

pub struct MockEnvBuilder {
    pub app: BasicApp,
    pub deployer: Addr,
    pub health_contract: Option<Addr>,
    pub cm_contract: Option<Addr>,
    pub vault_contract: Option<Addr>,
    pub oracle: Option<Addr>,
    pub red_bank: Option<Addr>,
    pub set_cm_config: bool,
}

impl MockEnvBuilder {
    pub fn build(&mut self) -> AnyResult<MockEnv> {
        if self.set_cm_config {
            self.add_cm_to_config();
        }

        Ok(MockEnv {
            deployer: self.deployer.clone(),
            health_contract: self.get_health_contract(),
            vault_contract: self.get_vault_contract(),
            oracle: self.get_oracle(),
            red_bank: self.get_red_bank(),
            cm_contract: self.get_cm_contract(),
            app: take(&mut self.app),
        })
    }

    pub fn skip_cm_config(&mut self) -> &mut Self {
        self.set_cm_config = false;
        self
    }

    fn add_cm_to_config(&mut self) {
        let health_contract = self.get_health_contract();
        let cm_contract = self.get_cm_contract();

        self.app
            .execute_contract(
                self.deployer.clone(),
                health_contract,
                &UpdateConfig {
                    credit_manager: cm_contract.to_string(),
                },
                &[],
            )
            .unwrap();
    }

    fn get_oracle(&mut self) -> Addr {
        if self.oracle.is_none() {
            self.deploy_oracle()
        }
        self.oracle.clone().unwrap()
    }

    fn deploy_oracle(&mut self) {
        let contract = mock_oracle_contract();
        let code_id = self.app.store_code(contract);

        let addr = self
            .app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &OracleInstantiateMsg {
                    prices: vec![],
                },
                &[],
                "mock-oracle",
                None,
            )
            .unwrap();
        self.oracle = Some(addr);
    }

    fn get_red_bank(&mut self) -> Addr {
        if self.red_bank.is_none() {
            self.deploy_red_bank()
        }
        self.red_bank.clone().unwrap()
    }

    fn deploy_red_bank(&mut self) {
        let contract = mock_red_bank_contract();
        let code_id = self.app.store_code(contract);

        let addr = self
            .app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &RedBankInstantiateMsg {
                    coins: vec![],
                },
                &[],
                "mock-red-bank",
                None,
            )
            .unwrap();
        self.red_bank = Some(addr);
    }

    fn get_cm_contract(&mut self) -> Addr {
        if self.cm_contract.is_none() {
            self.deploy_cm_contract()
        }
        self.cm_contract.clone().unwrap()
    }

    fn deploy_cm_contract(&mut self) {
        let contract = mock_credit_manager_contract();
        let code_id = self.app.store_code(contract);
        let red_bank = self.get_red_bank().to_string();
        let oracle = self.get_oracle().to_string();

        let cm_addr = self
            .app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &CmMockInstantiateMsg {
                    config: ConfigResponse {
                        ownership: OwnerResponse {
                            owner: Some(self.deployer.to_string()),
                            proposed: None,
                            emergency_owner: None,
                            initialized: true,
                            abolished: false,
                        },
                        red_bank,
                        oracle,
                        account_nft: None,
                        max_close_factor: Default::default(),
                        max_unlocking_positions: Default::default(),
                        swapper: "n/a".to_string(),
                        zapper: "n/a".to_string(),
                        health_contract: "n/a".to_string(),
                    },
                },
                &[],
                "mock-credit-manager-contract",
                Some(self.deployer.clone().into()),
            )
            .unwrap();
        self.cm_contract = Some(cm_addr.clone());

        // Set mock vault with a starting config
        let vault = self.get_vault_contract().to_string();
        self.app
            .execute_contract(
                self.deployer.clone(),
                cm_addr,
                &SetVaultConfig {
                    address: vault,
                    config: VaultConfig {
                        deposit_cap: coin(10000000u128, "uusdc"),
                        max_ltv: Decimal::from_atomics(4u128, 1).unwrap(),
                        liquidation_threshold: Decimal::from_atomics(44u128, 2).unwrap(),
                        whitelisted: true,
                    },
                },
                &[],
            )
            .unwrap();
    }

    fn get_vault_contract(&mut self) -> Addr {
        if self.vault_contract.is_none() {
            self.deploy_vault_contract()
        }
        self.vault_contract.clone().unwrap()
    }

    fn deploy_vault_contract(&mut self) {
        let contract = mock_vault_contract();
        let code_id = self.app.store_code(contract);

        let addr = self
            .app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &VaultInstantiateMsg {
                    vault_token_denom: "vault_token_xyz".to_string(),
                    lockup: Some(Duration::Height(100)),
                    base_token_denom: "base_token_abc".to_string(),
                    oracle: OracleUnchecked::new("oracle_123".to_string()),
                },
                &[],
                "mock-vault",
                None,
            )
            .unwrap();
        self.vault_contract = Some(addr);
    }

    fn get_health_contract(&mut self) -> Addr {
        if self.health_contract.is_none() {
            self.deploy_health_contract()
        }
        self.health_contract.clone().unwrap()
    }

    fn deploy_health_contract(&mut self) {
        let contract = mock_health_contract();
        let code_id = self.app.store_code(contract);

        let addr = self
            .app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &InstantiateMsg {
                    owner: self.deployer.clone().into(),
                },
                &[],
                "mock-health-contract",
                Some(self.deployer.clone().into()),
            )
            .unwrap();
        self.health_contract = Some(addr);
    }
}
