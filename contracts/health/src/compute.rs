use std::collections::HashMap;

use cosmwasm_std::{Deps, StdResult};
use mars_rover_health_computer::{DenomsData, HealthComputer, VaultsData};
use mars_types::{
    credit_manager::Positions,
    health::{AccountKind, HealthResult, HealthState, HealthValuesResponse},
    oracle::ActionKind,
};

use crate::querier::HealthQuerier;

/// Uses `mars-rover-health-computer` which is a data agnostic package given
/// it's compiled to .wasm and shared with the frontend.
/// This function queries all necessary data to pass to `HealthComputer`.
pub fn compute_health(
    deps: Deps,
    kind: AccountKind,
    q: HealthQuerier,
    positions: Positions,
    action: ActionKind,
) -> HealthResult<HealthValuesResponse> {
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
    let staked_lp_denoms = positions.staked_astro_lps.iter().map(|d| &d.denom).collect::<Vec<_>>();

    // Collect prices + asset
    let mut denoms_data: DenomsData = Default::default();
    deposit_denoms
        .into_iter()
        .chain(debt_denoms)
        .chain(lend_denoms)
        .chain(vault_base_token_denoms)
        .chain(staked_lp_denoms)
        .try_for_each(|denom| -> StdResult<()> {
            let params_opt = q.params.query_asset_params(&deps.querier, denom)?;
            // If the asset is not supported, we skip it (both params and price)
            if let Some(params) = params_opt {
                denoms_data.params.insert(denom.clone(), params);

                let price = q.oracle.query_price(&deps.querier, denom, action.clone())?.price;
                denoms_data.prices.insert(denom.clone(), price);
            }
            Ok(())
        })?;

    // Collect all vault data
    let mut vaults_data: VaultsData = Default::default();
    positions.vaults.iter().try_for_each(|v| -> HealthResult<()> {
        let vault_coin_value = v.query_values(&deps.querier, &q.oracle, action.clone())?;
        vaults_data.vault_values.insert(v.vault.address.clone(), vault_coin_value);
        let config = q.query_vault_config(&v.vault)?;
        vaults_data.vault_configs.insert(v.vault.address.clone(), config);
        Ok(())
    })?;

    let computer = HealthComputer {
        kind,
        positions,
        denoms_data,
        vaults_data,
    };

    Ok(computer.compute_health()?.into())
}

pub fn health_values(
    deps: Deps,
    account_id: &str,
    kind: AccountKind,
    action: ActionKind,
) -> HealthResult<HealthValuesResponse> {
    let q = HealthQuerier::new(&deps)?;
    let positions = q.query_positions(account_id)?;
    compute_health(deps, kind, q, positions, action)
}

pub fn health_state(
    deps: Deps,
    account_id: &str,
    kind: AccountKind,
    action: ActionKind,
) -> HealthResult<HealthState> {
    let q = HealthQuerier::new(&deps)?;
    let positions = q.query_positions(account_id)?;

    // Helpful to not have to do computations & query the oracle for cases
    // like liquidations where oracle circuit breakers may hinder it.
    if positions.debts.is_empty() {
        return Ok(HealthState::Healthy);
    }

    let health = compute_health(deps, kind, q, positions, action)?;
    if !health.above_max_ltv {
        Ok(HealthState::Healthy)
    } else {
        Ok(HealthState::Unhealthy {
            max_ltv_health_factor: health.max_ltv_health_factor.unwrap(),
        })
    }
}
