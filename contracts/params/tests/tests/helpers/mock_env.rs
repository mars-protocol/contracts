use std::{mem::take, str::FromStr};

use anyhow::Result as AnyResult;
use cosmwasm_std::{Addr, Decimal, Empty};
use cw_multi_test::{App, AppResponse, BasicApp, Executor};
use cw_paginate::PaginationResponse;
use mars_owner::{OwnerResponse, OwnerUpdate};
use mars_testing::{
    integration::mock_contracts::mock_rewards_collector_osmosis_contract,
    multitest::helpers::{
        mock_address_provider_contract, mock_incentives_contract, mock_oracle_contract,
        mock_red_bank_contract,
    },
};
use mars_types::{
    address_provider::{self, MarsAddressType},
    incentives, oracle,
    params::{
        AssetParams, AssetParamsUpdate, ConfigResponse, EmergencyUpdate, ExecuteMsg,
        InstantiateMsg, QueryMsg, VaultConfig, VaultConfigUpdate,
    },
    red_bank,
    rewards_collector::{self, RewardConfig, TransferType},
};

use super::contracts::mock_params_contract;

pub struct MockEnv {
    pub app: BasicApp,
    pub params_contract: Addr,
}

pub struct MockEnvBuilder {
    pub app: BasicApp,
    pub deployer: Addr,
    pub target_health_factor: Option<Decimal>,
    pub emergency_owner: Option<String>,
    pub address_provider: Option<Addr>,
}

#[allow(clippy::new_ret_no_self)]
impl MockEnv {
    pub fn new() -> MockEnvBuilder {
        MockEnvBuilder {
            app: App::default(),
            deployer: Addr::unchecked("deployer"),
            target_health_factor: None,
            emergency_owner: None,
            address_provider: None,
        }
    }

    //--------------------------------------------------------------------------------------------------
    // Execute Msgs
    //--------------------------------------------------------------------------------------------------

    pub fn update_asset_params(
        &mut self,
        sender: &Addr,
        update: AssetParamsUpdate,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.params_contract.clone(),
            &ExecuteMsg::UpdateAssetParams(update),
            &[],
        )
    }

    pub fn update_vault_config(
        &mut self,
        sender: &Addr,
        update: VaultConfigUpdate,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.params_contract.clone(),
            &ExecuteMsg::UpdateVaultConfig(update),
            &[],
        )
    }

    pub fn update_owner(&mut self, sender: &Addr, update: OwnerUpdate) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.params_contract.clone(),
            &ExecuteMsg::UpdateOwner(update),
            &[],
        )
    }

    pub fn update_target_health_factor(
        &mut self,
        sender: &Addr,
        thf: Decimal,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.params_contract.clone(),
            &ExecuteMsg::UpdateTargetHealthFactor(thf),
            &[],
        )
    }

    pub fn update_config(
        &mut self,
        sender: &Addr,
        address_provider: Option<String>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.params_contract.clone(),
            &ExecuteMsg::UpdateConfig {
                address_provider,
            },
            &[],
        )
    }

    pub fn emergency_update(
        &mut self,
        sender: &Addr,
        update: EmergencyUpdate,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.params_contract.clone(),
            &ExecuteMsg::EmergencyUpdate(update),
            &[],
        )
    }

    //--------------------------------------------------------------------------------------------------
    // Queries
    //--------------------------------------------------------------------------------------------------

    pub fn query_owner(&self) -> Addr {
        let res = self.query_ownership();
        Addr::unchecked(res.owner.unwrap())
    }

    pub fn query_ownership(&self) -> OwnerResponse {
        self.app.wrap().query_wasm_smart(self.params_contract.clone(), &QueryMsg::Owner {}).unwrap()
    }

    pub fn query_asset_params(&self, denom: &str) -> AssetParams {
        self.app
            .wrap()
            .query_wasm_smart(
                self.params_contract.clone(),
                &QueryMsg::AssetParams {
                    denom: denom.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_all_asset_params(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Vec<AssetParams> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.params_contract.clone(),
                &QueryMsg::AllAssetParams {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_vault_config(&self, addr: &str) -> VaultConfig {
        self.app
            .wrap()
            .query_wasm_smart(
                self.params_contract.clone(),
                &QueryMsg::VaultConfig {
                    address: addr.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_all_vault_configs(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Vec<VaultConfig> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.params_contract.clone(),
                &QueryMsg::AllVaultConfigs {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_all_vault_configs_v2(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> PaginationResponse<VaultConfig> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.params_contract.clone(),
                &QueryMsg::AllVaultConfigsV2 {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_target_health_factor(&self) -> Decimal {
        self.app
            .wrap()
            .query_wasm_smart(self.params_contract.clone(), &QueryMsg::TargetHealthFactor {})
            .unwrap()
    }

    pub fn query_config(&self) -> ConfigResponse {
        self.app
            .wrap()
            .query_wasm_smart(self.params_contract.clone(), &QueryMsg::Config {})
            .unwrap()
    }
}

impl MockEnvBuilder {
    pub fn build(&mut self) -> AnyResult<MockEnv> {
        let code_id = self.app.store_code(mock_params_contract());

        let params_contract = self.app.instantiate_contract(
            code_id,
            Addr::unchecked("owner"),
            &InstantiateMsg {
                owner: "owner".to_string(),
                address_provider: "address_provider".to_string(),
                target_health_factor: self.get_target_health_factor(),
            },
            &[],
            "mock-params-contract",
            None,
        )?;

        if self.emergency_owner.is_some() {
            self.set_emergency_owner(&params_contract, &self.emergency_owner.clone().unwrap());
        }

        Ok(MockEnv {
            app: take(&mut self.app),
            params_contract,
        })
    }

    fn deploy_address_provider(&mut self) -> Addr {
        let contract = mock_address_provider_contract();
        let code_id = self.app.store_code(contract);

        self.app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &address_provider::InstantiateMsg {
                    owner: self.deployer.clone().to_string(),
                    prefix: "".to_string(),
                },
                &[],
                "mock-address-provider",
                None,
            )
            .unwrap()
    }

    fn deploy_oracle(&mut self) -> Addr {
        let code_id = self.app.store_code(mock_oracle_contract());

        let addr = self
            .app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &oracle::InstantiateMsg::<Empty> {
                    owner: self.deployer.to_string(),
                    base_denom: "uusd".to_string(),
                    custom_init: None,
                },
                &[],
                "oracle",
                None,
            )
            .unwrap();

        self.set_address(MarsAddressType::Oracle, addr.clone());

        addr
    }

    fn deploy_red_bank(&mut self, address_provider: &str) -> Addr {
        let code_id = self.app.store_code(mock_red_bank_contract());

        let addr = self
            .app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &red_bank::InstantiateMsg {
                    owner: self.deployer.to_string(),
                    config: red_bank::CreateOrUpdateConfig {
                        address_provider: Some(address_provider.to_string()),
                    },
                },
                &[],
                "red-bank",
                None,
            )
            .unwrap();

        self.set_address(MarsAddressType::RedBank, addr.clone());

        addr
    }

    fn deploy_incentives(&mut self, address_provider_addr: &Addr) -> Addr {
        let code_id = self.app.store_code(mock_incentives_contract());

        let addr = self
            .app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &incentives::InstantiateMsg {
                    owner: self.deployer.to_string(),
                    address_provider: address_provider_addr.to_string(),
                    epoch_duration: 604800,
                    max_whitelisted_denoms: 10,
                },
                &[],
                "incentives",
                None,
            )
            .unwrap();

        self.set_address(MarsAddressType::Incentives, addr.clone());

        addr
    }

    fn deploy_rewards_collector_osmosis(&mut self, address_provider_addr: &Addr) -> Addr {
        let code_id = self.app.store_code(mock_rewards_collector_osmosis_contract());

        let addr = self
            .app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &rewards_collector::InstantiateMsg {
                    owner: self.deployer.to_string(),
                    address_provider: address_provider_addr.to_string(),
                    safety_tax_rate: Decimal::percent(50),
                    revenue_share_tax_rate: Decimal::percent(50),
                    safety_fund_config: RewardConfig {
                        target_denom: "uusdc".to_string(),
                        transfer_type: TransferType::Bank,
                    },
                    revenue_share_config: RewardConfig {
                        target_denom: "uusdc".to_string(),
                        transfer_type: TransferType::Bank,
                    },
                    fee_collector_config: RewardConfig {
                        target_denom: "umars".to_string(),
                        transfer_type: TransferType::Bank,
                    },
                    channel_id: "0".to_string(),
                    timeout_seconds: 900,
                    whitelisted_distributors: vec![],
                },
                &[],
                "rewards-collector",
                None,
            )
            .unwrap();

        self.set_address(MarsAddressType::RewardsCollector, addr.clone());

        addr
    }

    fn set_address(&mut self, address_type: MarsAddressType, address: Addr) {
        let address_provider_addr = self.get_address_provider();

        self.app
            .execute_contract(
                self.deployer.clone(),
                address_provider_addr,
                &address_provider::ExecuteMsg::SetAddress {
                    address_type,
                    address: address.into(),
                },
                &[],
            )
            .unwrap();
    }

    fn get_address_provider(&mut self) -> Addr {
        if self.address_provider.is_none() {
            let addr = self.deploy_address_provider();

            self.address_provider = Some(addr);
        }
        self.address_provider.clone().unwrap()
    }

    fn set_emergency_owner(&mut self, params_contract: &Addr, eo: &str) {
        self.app
            .execute_contract(
                Addr::unchecked("owner"),
                params_contract.clone(),
                &ExecuteMsg::UpdateOwner(OwnerUpdate::SetEmergencyOwner {
                    emergency_owner: eo.to_string(),
                }),
                &[],
            )
            .unwrap();
    }

    //--------------------------------------------------------------------------------------------------
    // Get or defaults
    //--------------------------------------------------------------------------------------------------

    pub fn get_target_health_factor(&self) -> Decimal {
        self.target_health_factor.unwrap_or(Decimal::from_str("1.05").unwrap())
    }

    //--------------------------------------------------------------------------------------------------
    // Setter functions
    //--------------------------------------------------------------------------------------------------
    pub fn target_health_factor(&mut self, thf: Decimal) -> &mut Self {
        self.target_health_factor = Some(thf);
        self
    }

    pub fn emergency_owner(&mut self, eo: &str) -> &mut Self {
        self.emergency_owner = Some(eo.to_string());
        self
    }
}
