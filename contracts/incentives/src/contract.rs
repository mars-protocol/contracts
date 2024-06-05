#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;
use mars_owner::OwnerInit::SetInitialOwner;
use mars_types::incentives::{Config, ExecuteMsg, InstantiateMsg, QueryMsg};

use crate::{
    astro_incentives, config,
    error::ContractError,
    mars_incentives, migrations, query,
    state::{CONFIG, EPOCH_DURATION, MIGRATION_GUARD, OWNER},
};

pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The epoch duration should be at least one week, perhaps ideally one month. This is to ensure
/// that the max gas limit is not reached when iterating over incentives.
pub const MIN_EPOCH_DURATION: u64 = 604800u64;

// INIT

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    OWNER.initialize(
        deps.storage,
        deps.api,
        SetInitialOwner {
            owner: msg.owner,
        },
    )?;

    let config = Config {
        address_provider: deps.api.addr_validate(&msg.address_provider)?,
        max_whitelisted_denoms: msg.max_whitelisted_denoms,
    };
    CONFIG.save(deps.storage, &config)?;

    if msg.epoch_duration < MIN_EPOCH_DURATION {
        return Err(ContractError::EpochDurationTooShort {
            min_epoch_duration: MIN_EPOCH_DURATION,
        });
    }

    EPOCH_DURATION.save(deps.storage, &msg.epoch_duration)?;

    Ok(Response::default())
}

// HANDLERS

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateWhitelist {
            add_denoms,
            remove_denoms,
        } => config::execute_update_whitelist(deps, env, info, add_denoms, remove_denoms),
        ExecuteMsg::SetAssetIncentive {
            collateral_denom,
            incentive_denom,
            emission_per_second,
            start_time,
            duration,
        } => mars_incentives::execute_set_asset_incentive(
            deps,
            env,
            info,
            collateral_denom,
            incentive_denom,
            emission_per_second,
            start_time,
            duration,
        ),
        ExecuteMsg::BalanceChange {
            user_addr,
            account_id,
            denom,
            user_amount_scaled_before,
            total_amount_scaled_before,
        } => {
            MIGRATION_GUARD.assert_unlocked(deps.storage)?;
            mars_incentives::execute_balance_change(
                deps,
                env,
                info,
                user_addr,
                account_id,
                denom,
                user_amount_scaled_before,
                total_amount_scaled_before,
            )
        }
        ExecuteMsg::ClaimStakedAstroLpRewards {
            account_id,
            lp_denom,
        } => astro_incentives::execute_claim_rewards_for_staked_lp_position(
            deps,
            env,
            info,
            &account_id,
            &lp_denom,
        ),
        ExecuteMsg::ClaimRewards {
            account_id,
            start_after_collateral_denom,
            start_after_incentive_denom,
            limit,
        } => {
            MIGRATION_GUARD.assert_unlocked(deps.storage)?;
            mars_incentives::execute_claim_rewards(
                deps,
                env,
                info,
                account_id,
                start_after_collateral_denom,
                start_after_incentive_denom,
                limit,
            )
        }
        ExecuteMsg::StakeAstroLp {
            account_id,
            lp_coin,
        } => astro_incentives::execute_stake_lp(deps, env, info, account_id, lp_coin),
        ExecuteMsg::UnstakeAstroLp {
            account_id,
            lp_coin,
        } => astro_incentives::execute_unstake_lp(deps, env, info, account_id, lp_coin),
        ExecuteMsg::UpdateConfig {
            address_provider,
            max_whitelisted_denoms,
        } => Ok(config::execute_update_config(
            deps,
            env,
            info,
            address_provider,
            max_whitelisted_denoms,
        )?),
        ExecuteMsg::UpdateOwner(update) => config::update_owner(deps, info, update),
        ExecuteMsg::Migrate(msg) => migrations::v2_0_0::execute_migration(deps, info, msg),
    }
}

// QUERIES

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::StakedAstroLpRewards {
            account_id,
            lp_denom,
        } => to_json_binary(&query::query_staked_astro_lp_rewards_for_denom(
            deps,
            &env,
            &account_id,
            &lp_denom,
        )?),

        QueryMsg::Config {} => to_json_binary(&query::query_config(deps)?),
        QueryMsg::IncentiveState {
            collateral_denom,
            incentive_denom,
        } => {
            to_json_binary(&query::query_incentive_state(deps, collateral_denom, incentive_denom)?)
        }
        QueryMsg::IncentiveStates {
            start_after_collateral_denom,
            start_after_incentive_denom,
            limit,
        } => to_json_binary(&query::query_incentive_states(
            deps,
            start_after_collateral_denom,
            start_after_incentive_denom,
            limit,
        )?),
        QueryMsg::UserUnclaimedRewards {
            user,
            account_id,
            start_after_collateral_denom,
            start_after_incentive_denom,
            limit,
        } => to_json_binary(&query::query_user_unclaimed_rewards(
            deps,
            env,
            user,
            account_id,
            start_after_collateral_denom,
            start_after_incentive_denom,
            limit,
        )?),
        QueryMsg::Whitelist {} => to_json_binary(&query::query_whitelist(deps)?),
        QueryMsg::Emission {
            collateral_denom,
            incentive_denom,
            timestamp,
        } => to_json_binary(&query::query_emission(
            deps,
            &collateral_denom,
            &incentive_denom,
            timestamp,
        )?),
        QueryMsg::Emissions {
            collateral_denom,
            incentive_denom,
            start_after_timestamp,
            limit,
        } => to_json_binary(&query::query_emissions(
            deps,
            collateral_denom,
            incentive_denom,
            start_after_timestamp,
            limit,
        )?),
        QueryMsg::ActiveEmissions {
            collateral_denom,
        } => to_json_binary(&query::query_active_emissions(deps, env, &collateral_denom)?),
        QueryMsg::StakedAstroLpPositions {
            account_id,
            start_after,
            limit,
        } => to_json_binary(&query::query_staked_astro_lp_positions(
            deps,
            env,
            account_id,
            start_after,
            limit,
        )?),
        QueryMsg::StakedAstroLpPosition {
            account_id,
            lp_denom,
        } => {
            to_json_binary(&query::query_staked_astro_lp_position(deps, env, account_id, lp_denom)?)
        }
    }
}

/// MIGRATION

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, msg: Empty) -> Result<Response, ContractError> {
    migrations::v2_0_0::migrate(deps, env, msg)
}
