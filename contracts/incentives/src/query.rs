use std::collections::HashMap;

use astroport::asset::Asset;
use cosmwasm_std::{
    Addr, Coin, Coins, Decimal, Deps, Env, Order, Order::Ascending, StdError, StdResult, Uint128,
};
use cw_storage_plus::Bound;
use mars_types::{
    address_provider,
    address_provider::MarsAddressType,
    incentives::{
        ActiveEmission, ConfigResponse, EmissionResponse, IncentiveStateResponse,
        StakedLpPositionResponse, WhitelistEntry,
    },
};

use crate::{
    helpers::{
        calculate_rewards_from_astroport_incentive_state,
        compute_updated_astroport_incentive_states, compute_user_unclaimed_rewards,
    },
    state,
    state::{
        ASTROPORT_INCENTIVE_STATES, CONFIG, DEFAULT_LIMIT, EMISSIONS, EPOCH_DURATION,
        INCENTIVE_STATES, LP_DEPOSITS, MAX_LIMIT, OWNER, WHITELIST, WHITELIST_COUNT,
    },
    ContractError,
};

pub fn query_active_emissions(
    deps: Deps,
    env: Env,
    collateral_denom: &str,
) -> StdResult<Vec<ActiveEmission>> {
    Ok(INCENTIVE_STATES
        .prefix(collateral_denom)
        .keys(deps.storage, None, None, Order::Ascending)
        .map(|incentive_denom| {
            let incentive_denom = incentive_denom?;
            let emission =
                query_emission(deps, collateral_denom, &incentive_denom, env.block.time.seconds())?;

            Ok::<ActiveEmission, _>((incentive_denom, emission).into())
        })
        .collect::<StdResult<Vec<_>>>()?
        .into_iter()
        .filter(|emission| emission.emission_rate != Uint128::zero())
        .collect())
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let owner_state = OWNER.query(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: owner_state.owner,
        proposed_new_owner: owner_state.proposed,
        address_provider: config.address_provider,
        max_whitelisted_denoms: config.max_whitelisted_denoms,
        epoch_duration: EPOCH_DURATION.load(deps.storage)?,
        whitelist_count: WHITELIST_COUNT.may_load(deps.storage)?.unwrap_or_default(),
    })
}

pub fn query_incentive_state(
    deps: Deps,
    collateral_denom: String,
    incentive_denom: String,
) -> StdResult<IncentiveStateResponse> {
    let incentive_state =
        INCENTIVE_STATES.load(deps.storage, (&collateral_denom, &incentive_denom))?;
    Ok(IncentiveStateResponse::from(collateral_denom, incentive_denom, incentive_state))
}

pub fn query_incentive_states(
    deps: Deps,
    start_after_collateral_denom: Option<String>,
    start_after_incentive_denom: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<IncentiveStateResponse>> {
    let incentive_states = state::paginate_incentive_states(
        deps.storage,
        start_after_collateral_denom,
        start_after_incentive_denom,
        limit,
    )?;

    incentive_states
        .into_iter()
        .map(|((collateral_denom, incentive_denom), ai)| {
            Ok(IncentiveStateResponse::from(collateral_denom, incentive_denom, ai))
        })
        .collect()
}

/// Query the unclaimed astroport rewards
pub fn query_unclaimed_astroport_rewards(
    deps: Deps,
    mars_incentives_addr: &str,
    astroport_incentives_addr: &str,
    lp_denom: &str,
) -> Result<Vec<Coin>, ContractError> {
    let result: Vec<Asset> = deps.querier.query_wasm_smart(
        astroport_incentives_addr,
        &astroport::incentives::QueryMsg::PendingRewards {
            lp_token: lp_denom.to_string(),
            user: mars_incentives_addr.to_string(),
        },
    )?;

    let native_coins = result
        .into_iter()
        .filter_map(|x| x.try_into().ok()) // filter out non native coins
        .collect();
    Ok(native_coins)
}

/// Fetch rewards for a user, grouped by LP position
/// Has optional pagination on LP denom
pub fn query_lp_rewards_for_user(
    deps: Deps,
    env: &Env,
    astroport_incentives_addr: &Addr,
    user_id_key: &str,
    maybe_start_after_lp_denom: Option<&str>,
    limit: Option<u32>,
) -> Result<Vec<(String, Vec<Coin>)>, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = match maybe_start_after_lp_denom {
        Some(start_after_lp_denom) => {
            let start = Bound::exclusive(start_after_lp_denom);
            Some(start)
        }
        None => None,
    };

    let lp_deposits = LP_DEPOSITS
        .prefix(user_id_key)
        .range(deps.storage, start, None, Ascending)
        .take(limit)
        .map(|res: StdResult<(String, Uint128)>| {
            let (lp_denom, amount) = res.expect("Lp Position does not exist");
            (
                lp_denom.clone(),
                query_lp_rewards_for_position(
                    deps,
                    env,
                    astroport_incentives_addr,
                    user_id_key,
                    &Coin {
                        denom: lp_denom,
                        amount,
                    },
                )
                .expect("LP Rewards query failed"),
            )
        })
        .collect();

    Ok(lp_deposits)
}

/// Fetch the rewards owed to a user.
///
/// The incentives contract deposits / stakes all LP on behalf
/// off the user, so the rewards accounting is all tracked internally
/// by the incentives contract.
pub fn query_lp_rewards_for_position(
    deps: Deps,
    env: &Env,
    astroport_incentives_addr: &Addr,
    user_id_key: &str,
    lp_coin: &Coin,
) -> Result<Vec<Coin>, ContractError> {
    let lp_denom = &lp_coin.denom;
    let pending_rewards: Vec<Coin> = query_unclaimed_astroport_rewards(
        deps,
        env.contract.address.as_ref(),
        astroport_incentives_addr.as_ref(),
        lp_denom,
    )
    .unwrap_or_default();

    // Update our global indexes for each reward. We only accept native tokens,
    // cw20 will just be swallowed by contract
    let incentives_to_update =
        compute_updated_astroport_incentive_states(deps.storage, pending_rewards, lp_denom)?;

    let mut incentive_states: HashMap<String, Decimal> = ASTROPORT_INCENTIVE_STATES
        .prefix(lp_denom)
        .range(deps.storage, None, None, Ascending)
        .collect::<StdResult<HashMap<String, Decimal>>>()?;

    // Update our incentive states with the newly updated incentive states to ensure we are up to date.
    incentive_states.extend(incentives_to_update);

    let reward_coins = calculate_rewards_from_astroport_incentive_state(
        deps.storage,
        user_id_key,
        lp_coin,
        incentive_states,
    )?;

    Ok(reward_coins)
}

pub fn query_user_unclaimed_rewards(
    deps: Deps,
    env: Env,
    user: String,
    account_id: Option<String>,
    start_after_collateral_denom: Option<String>,
    start_after_incentive_denom: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<Coin>, ContractError> {
    let user_addr = deps.api.addr_validate(&user)?;
    let red_bank_addr = query_red_bank_address(deps)?;

    let incentive_states = state::paginate_incentive_states(
        deps.storage,
        start_after_collateral_denom,
        start_after_incentive_denom,
        limit,
    )?;

    let mut total_unclaimed_rewards = Coins::default();

    for ((collateral_denom, incentive_denom), _) in incentive_states {
        let unclaimed_rewards = compute_user_unclaimed_rewards(
            &mut deps.storage.into(),
            &deps.querier,
            &env.block,
            &red_bank_addr,
            &user_addr,
            &account_id,
            &collateral_denom,
            &incentive_denom,
        )?;

        total_unclaimed_rewards.add(Coin {
            denom: incentive_denom,
            amount: unclaimed_rewards,
        })?;
    }

    Ok(total_unclaimed_rewards.into())
}

pub fn query_red_bank_address(deps: Deps) -> StdResult<Addr> {
    let config = CONFIG.load(deps.storage)?;
    address_provider::helpers::query_contract_addr(
        deps,
        &config.address_provider,
        MarsAddressType::RedBank,
    )
}

pub fn query_whitelist(deps: Deps) -> StdResult<Vec<WhitelistEntry>> {
    let whitelist: Vec<WhitelistEntry> = WHITELIST
        .range(deps.storage, None, None, Order::Ascending)
        .map(|res| {
            let (denom, min_emission_rate) = res?;
            Ok(WhitelistEntry {
                denom,
                min_emission_rate,
            })
        })
        .collect::<StdResult<_>>()?;
    Ok(whitelist)
}

pub fn query_emission(
    deps: Deps,
    collateral_denom: &str,
    incentive_denom: &str,
    timestamp: u64,
) -> StdResult<Uint128> {
    let epoch_duration = EPOCH_DURATION.load(deps.storage)?;
    let emission = EMISSIONS
        .prefix((collateral_denom, incentive_denom))
        .range(
            deps.storage,
            Some(Bound::inclusive(timestamp.saturating_sub(epoch_duration - 1))),
            Some(Bound::inclusive(timestamp)),
            Order::Ascending,
        )
        .next()
        .transpose()?
        .map(|(_, emission)| emission)
        .unwrap_or_default();

    Ok(emission)
}

pub fn query_emissions(
    deps: Deps,
    collateral_denom: String,
    incentive_denom: String,
    start_after_timestamp: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<EmissionResponse>> {
    let min = start_after_timestamp.map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let emissions = EMISSIONS
        .prefix((&collateral_denom, &incentive_denom))
        .range(deps.storage, min, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;

    Ok(emissions.into_iter().map(|x| x.into()).collect())
}

pub fn query_user_lp_position(
    deps: Deps,
    env: Env,
    account_id: String,
    denom: String,
) -> StdResult<StakedLpPositionResponse> {
    // fetch position for lp position
    let config = CONFIG.load(deps.storage)?;
    let astroport_incentive_addr = address_provider::helpers::query_contract_addr(
        deps,
        &config.address_provider,
        MarsAddressType::AstroportIncentives,
    )?;

    // query the position
    let amount = LP_DEPOSITS.may_load(deps.storage, (&account_id, &denom))?.ok_or(
        ContractError::NoStakedLp {
            account_id: account_id.clone(),
            denom: denom.clone(),
        },
    )?;

    let lp_coin = Coin {
        denom,
        amount,
    };

    let rewards = query_lp_rewards_for_position(
        deps,
        &env,
        &astroport_incentive_addr,
        &account_id,
        &lp_coin,
    )?;

    let result = StakedLpPositionResponse {
        lp_coin,
        rewards,
    };

    Ok(result)
}

pub fn query_user_lp_positions(
    deps: Deps,
    env: Env,
    account_id: String,
    start_after_denom: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<StakedLpPositionResponse>> {
    let config = CONFIG.load(deps.storage)?;
    let astroport_incentive_addr = address_provider::helpers::query_contract_addr(
        deps,
        &config.address_provider,
        MarsAddressType::AstroportIncentives,
    )?;

    let min = start_after_denom.as_ref().map(|denom| Bound::exclusive(denom.as_str()));
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let deposits = LP_DEPOSITS
        .prefix(&account_id)
        .range(deps.storage, min, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;

    Ok(deposits
        .into_iter()
        .map(|(denom, amount)| {
            let lp_coin = Coin {
                denom,
                amount,
            };
            let rewards = query_lp_rewards_for_position(
                deps,
                &env,
                &astroport_incentive_addr,
                &account_id,
                &lp_coin,
            )
            .unwrap_or_default();

            StakedLpPositionResponse {
                lp_coin,
                rewards,
            }
        })
        .collect())
}