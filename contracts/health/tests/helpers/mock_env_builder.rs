use std::{mem::take, str::FromStr};

use anyhow::Result as AnyResult;
use cosmwasm_std::{coin, Addr, Decimal};
use cw_multi_test::{BasicApp, Executor};
use cw_utils::Duration;
use mars_mock_credit_manager::msg::InstantiateMsg as CmMockInstantiateMsg;
use mars_mock_oracle::msg::InstantiateMsg as OracleInstantiateMsg;
use mars_mock_vault::msg::InstantiateMsg as VaultInstantiateMsg;
use mars_owner::OwnerResponse;
use mars_params::{
    msg::{
        ExecuteMsg::UpdateVaultConfig, InstantiateMsg as ParamsInstantiateMsg,
        VaultConfigUpdate::AddOrUpdate,
    },
    types::{hls::HlsParamsUnchecked, vault::VaultConfigUnchecked},
};
use mars_rover::{adapters::oracle::OracleUnchecked, msg::query::ConfigResponse};
use mars_rover_health_types::{ExecuteMsg::UpdateConfig, InstantiateMsg};

use crate::helpers::{
    mock_credit_manager_contract, mock_health_contract, mock_oracle_contract, mock_params_contract,
    mock_vault_contract, MockEnv,
};

pub struct MockEnvBuilder {
    pub app: BasicApp,
    pub deployer: Addr,
    pub health_contract: Option<Addr>,
    pub cm_contract: Option<Addr>,
    pub vault_contract: Option<Addr>,
    pub oracle: Option<Addr>,
    pub params: Option<Addr>,
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
            cm_contract: self.get_cm_contract(),
            app: take(&mut self.app),
            params: self.get_params_contract(),
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

    fn get_cm_contract(&mut self) -> Addr {
        if self.cm_contract.is_none() {
            self.deploy_cm_contract()
        }
        self.cm_contract.clone().unwrap()
    }

    fn deploy_cm_contract(&mut self) {
        let contract = mock_credit_manager_contract();
        let code_id = self.app.store_code(contract);
        let oracle = self.get_oracle().to_string();
        let params = self.get_params_contract().to_string();

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
                        red_bank: "n/a".to_string(),
                        oracle,
                        params,
                        account_nft: None,
                        max_unlocking_positions: Default::default(),
                        swapper: "n/a".to_string(),
                        zapper: "n/a".to_string(),
                        health_contract: "n/a".to_string(),
                        rewards_collector: None,
                    },
                },
                &[],
                "mock-credit-manager-contract",
                Some(self.deployer.clone().into()),
            )
            .unwrap();
        self.cm_contract = Some(cm_addr);

        // Set mock vault with a starting config
        let vault = self.get_vault_contract();
        let params = self.get_params_contract();
        self.app
            .execute_contract(
                self.deployer.clone(),
                params,
                &UpdateVaultConfig(AddOrUpdate {
                    config: VaultConfigUnchecked {
                        addr: vault.to_string(),
                        deposit_cap: coin(10000000u128, "uusdc"),
                        max_loan_to_value: Decimal::from_str("0.4").unwrap(),
                        liquidation_threshold: Decimal::from_str("0.44").unwrap(),
                        whitelisted: true,
                        hls: Some(HlsParamsUnchecked {
                            max_loan_to_value: Decimal::from_str("0.6").unwrap(),
                            liquidation_threshold: Decimal::from_str("0.7").unwrap(),
                            correlations: vec![],
                        }),
                    },
                }),
                &[],
            )
            .unwrap();
    }

    fn get_params_contract(&mut self) -> Addr {
        if self.params.is_none() {
            let hc = self.deploy_params_contract();
            self.params = Some(hc);
        }
        self.params.clone().unwrap()
    }

    pub fn deploy_params_contract(&mut self) -> Addr {
        let contract_code_id = self.app.store_code(mock_params_contract());
        let owner = self.deployer.clone();

        self.app
            .instantiate_contract(
                contract_code_id,
                owner.clone(),
                &ParamsInstantiateMsg {
                    owner: owner.to_string(),
                    target_health_factor: Decimal::from_str("1.2").unwrap(),
                },
                &[],
                "mock-params-contract",
                Some(owner.to_string()),
            )
            .unwrap()
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
                    is_evil: None,
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
