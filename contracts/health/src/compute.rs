use std::collections::HashMap;

use cosmwasm_std::{Deps, StdResult};
use mars_rover_health_computer::{DenomsData, HealthComputer, VaultsData};
use mars_rover_health_types::{HealthError::ContractNotSet, HealthResponse, HealthResult};

use crate::{
    querier::HealthQuerier,
    state::{CREDIT_MANAGER, PARAMS},
};

/// Uses `mars-rover-health-computer` which is a data agnostic package given
/// it's compiled to .wasm and shared with the frontend.
/// This function queries all necessary data to pass to `HealthComputer`.
pub fn compute_health(deps: Deps, account_id: &str) -> HealthResult<HealthResponse> {
    let credit_manager_addr = CREDIT_MANAGER
        .may_load(deps.storage)?
        .ok_or(ContractNotSet("credit_manger".to_string()))?;
    let params_contract_addr =
        PARAMS.may_load(deps.storage)?.ok_or(ContractNotSet("params".to_string()))?;

    let querier = HealthQuerier::new(&deps.querier, &credit_manager_addr, &params_contract_addr);

    let positions = querier.query_positions(account_id)?;

    // Get the denoms that need prices + markets
    let deposit_denoms = positions.deposits.iter().map(|d| &d.denom).collect::<Vec<_>>();
    let debt_denoms = positions.debts.iter().map(|d| &d.denom).collect::<Vec<_>>();
    let lend_denoms = positions.lends.iter().map(|d| &d.denom).collect::<Vec<_>>();
    let vault_infos = positions
        .vaults
        .iter()
        .map(|v| {
            let info = v.vault.query_info(&deps.querier)?;
            Ok((v.vault.address.clone(), info))
        })
        .collect::<StdResult<HashMap<_, _>>>()?;
    let vault_base_token_denoms = vault_infos.values().map(|v| &v.base_token).collect::<Vec<_>>();

    // Collect prices + asset
    let (oracle, params) = querier.query_deps()?;
    let mut denoms_data: DenomsData = Default::default();
    deposit_denoms
        .into_iter()
        .chain(debt_denoms)
        .chain(lend_denoms)
        .chain(vault_base_token_denoms)
        .try_for_each(|denom| -> StdResult<()> {
            let price = oracle.query_price(&deps.querier, denom)?.price;
            denoms_data.prices.insert(denom.clone(), price);
            let params = params.query_asset_params(&deps.querier, denom)?;
            denoms_data.params.insert(denom.clone(), params);
            Ok(())
        })?;

    // Collect all vault data
    let mut vaults_data: VaultsData = Default::default();
    positions.vaults.iter().try_for_each(|v| -> HealthResult<()> {
        let vault_coin_value = v.query_values(&deps.querier, &oracle)?;
        vaults_data.vault_values.insert(v.vault.address.clone(), vault_coin_value);
        let config = querier.query_vault_config(&v.vault)?;
        vaults_data.vault_configs.insert(v.vault.address.clone(), config);
        Ok(())
    })?;

    let computer = HealthComputer {
        positions,
        denoms_data,
        vaults_data,
    };

    Ok(computer.compute_health()?.into())
}
