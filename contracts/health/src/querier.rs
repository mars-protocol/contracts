use cosmwasm_std::{Addr, QuerierWrapper};
use mars_params::{msg::QueryMsg as ParamsQueryMsg, types::VaultConfig};
use mars_rover::{
    adapters::{oracle::Oracle, params::Params, vault::Vault},
    msg::query::{ConfigResponse, Positions, QueryMsg as CmQueryMsg},
};
use mars_rover_health_types::HealthResult;

pub struct HealthQuerier<'a> {
    querier: &'a QuerierWrapper<'a>,
    credit_manager_addr: &'a Addr,
    params_contract_addr: &'a Addr,
}

impl<'a> HealthQuerier<'a> {
    pub fn new(
        querier: &'a QuerierWrapper,
        credit_manager_addr: &'a Addr,
        params_contract_addr: &'a Addr,
    ) -> Self {
        Self {
            querier,
            credit_manager_addr,
            params_contract_addr,
        }
    }

    pub fn query_positions(&self, account_id: &str) -> HealthResult<Positions> {
        Ok(self.querier.query_wasm_smart(
            self.credit_manager_addr.to_string(),
            &CmQueryMsg::Positions {
                account_id: account_id.to_string(),
            },
        )?)
    }

    pub fn query_deps(&self) -> HealthResult<(Oracle, Params)> {
        let config: ConfigResponse = self
            .querier
            .query_wasm_smart(self.credit_manager_addr.to_string(), &CmQueryMsg::Config {})?;
        Ok((
            Oracle::new(Addr::unchecked(config.oracle)),
            Params::new(Addr::unchecked(config.params)),
        ))
    }

    pub fn query_vault_config(&self, vault: &Vault) -> HealthResult<VaultConfig> {
        Ok(self.querier.query_wasm_smart(
            self.params_contract_addr.to_string(),
            &ParamsQueryMsg::VaultConfig {
                address: vault.address.to_string(),
            },
        )?)
    }
}
