use cosmwasm_std::{Addr, Deps, QuerierWrapper, StdResult};
use mars_params::types::vault::VaultConfig;
use mars_rover::{
    adapters::{oracle::Oracle, params::Params, vault::Vault},
    msg::query::{ConfigResponse, Positions, QueryMsg as CmQueryMsg},
};
use mars_rover_health_types::HealthResult;

use crate::state::CREDIT_MANAGER;

pub struct HealthQuerier<'a> {
    querier: &'a QuerierWrapper<'a>,
    credit_manager: Addr,
    pub params: Params,
    pub oracle: Oracle,
}

impl<'a> HealthQuerier<'a> {
    pub fn new(deps: &'a Deps) -> StdResult<Self> {
        let credit_manager = CREDIT_MANAGER.load(deps.storage)?;
        let config: ConfigResponse =
            deps.querier.query_wasm_smart(credit_manager.to_string(), &CmQueryMsg::Config {})?;

        Ok(Self {
            querier: &deps.querier,
            credit_manager,
            params: Params::new(Addr::unchecked(config.params)),
            oracle: Oracle::new(Addr::unchecked(config.oracle)),
        })
    }

    pub fn query_positions(&self, account_id: &str) -> HealthResult<Positions> {
        Ok(self.querier.query_wasm_smart(
            self.credit_manager.to_string(),
            &CmQueryMsg::Positions {
                account_id: account_id.to_string(),
            },
        )?)
    }

    pub fn query_vault_config(&self, vault: &Vault) -> HealthResult<VaultConfig> {
        Ok(self
            .params
            .query_vault_config(self.querier, &Addr::unchecked(vault.address.to_string()))?)
    }
}
