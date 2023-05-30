use std::{mem::take, str::FromStr};

use anyhow::Result as AnyResult;
use cosmwasm_std::{Addr, Decimal};
use cw_multi_test::{App, AppResponse, BasicApp, Executor};
use mars_owner::{OwnerResponse, OwnerUpdate};

use mars_params::types::AssetParams;
use mars_params::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    types::{AssetParamsUpdate, EmergencyUpdate, VaultConfig, VaultConfigUpdate},
};

use crate::helpers::mock_params_contract;

pub struct MockEnv {
    pub app: BasicApp,
    pub params_contract: Addr,
}

pub struct MockEnvBuilder {
    pub app: BasicApp,
    pub max_close_factor: Option<Decimal>,
    pub emergency_owner: Option<String>,
}

#[allow(clippy::new_ret_no_self)]
impl MockEnv {
    pub fn new() -> MockEnvBuilder {
        MockEnvBuilder {
            app: App::default(),
            max_close_factor: None,
            emergency_owner: None,
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

    pub fn update_max_close_factor(
        &mut self,
        sender: &Addr,
        mcf: Decimal,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.params_contract.clone(),
            &ExecuteMsg::UpdateMaxCloseFactor(mcf),
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

    pub fn query_max_close_factor(&self) -> Decimal {
        self.app
            .wrap()
            .query_wasm_smart(self.params_contract.clone(), &QueryMsg::MaxCloseFactor {})
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
                max_close_factor: self.get_max_close_factor(),
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

    pub fn get_max_close_factor(&self) -> Decimal {
        self.max_close_factor.unwrap_or(Decimal::from_str("0.5").unwrap())
    }

    //--------------------------------------------------------------------------------------------------
    // Setter functions
    //--------------------------------------------------------------------------------------------------
    pub fn max_close_factor(&mut self, mcf: Decimal) -> &mut Self {
        self.max_close_factor = Some(mcf);
        self
    }

    pub fn emergency_owner(&mut self, eo: &str) -> &mut Self {
        self.emergency_owner = Some(eo.to_string());
        self
    }
}
