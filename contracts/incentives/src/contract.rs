use std::collections::HashMap;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, coins, to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env,
    Event, MessageInfo, Order, Response, StdError, StdResult, Uint128,
};
use mars_owner::{OwnerInit::SetInitialOwner, OwnerUpdate};
use mars_red_bank_types::{
    address_provider::{self, MarsAddressType},
    error::MarsError,
    incentives::{
        Config, ConfigResponse, ExecuteMsg, IncentiveState, IncentiveStateResponse, InstantiateMsg,
        QueryMsg,
    },
};
use mars_utils::helpers::{option_string_to_addr, validate_native_denom};

use crate::{
    error::ContractError,
    helpers::{
        self, compute_user_accrued_rewards, compute_user_unclaimed_rewards, update_incentive_index,
    },
    state::{
        self, CONFIG, EMISSIONS, INCENTIVE_STATES, OWNER, USER_ASSET_INDICES,
        USER_UNCLAIMED_REWARDS, WHITELIST,
    },
};

pub const CONTRACT_NAME: &str = "crates.io:mars-incentives";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// INIT

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    OWNER.initialize(
        deps.storage,
        deps.api,
        SetInitialOwner {
            owner: msg.owner,
        },
    )?;

    let config = Config {
        address_provider: deps.api.addr_validate(&msg.address_provider)?,
        epoch_duration: msg.epoch_duration,
        min_incentive_emission: msg.min_incentive_emission,
    };

    CONFIG.save(deps.storage, &config)?;

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
        } => execute_update_whitelist(deps, env, info, add_denoms, remove_denoms),
        ExecuteMsg::SetAssetIncentive {
            collateral_denom,
            incentive_denom,
            emission_per_second,
            start_time,
            duration,
        } => execute_set_asset_incentive(
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
            denom,
            user_amount_scaled_before,
            total_amount_scaled_before,
        } => execute_balance_change(
            deps,
            env,
            info,
            user_addr,
            denom,
            user_amount_scaled_before,
            total_amount_scaled_before,
        ),
        ExecuteMsg::ClaimRewards {
            start_after_collateral_denom,
            start_after_incentive_denom,
            limit,
        } => execute_claim_rewards(
            deps,
            env,
            info,
            start_after_collateral_denom,
            start_after_incentive_denom,
            limit,
        ),
        ExecuteMsg::UpdateConfig {
            address_provider,
        } => Ok(execute_update_config(deps, env, info, address_provider)?),
        ExecuteMsg::UpdateOwner(update) => update_owner(deps, info, update),
    }
}

pub fn execute_update_whitelist(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    add_denoms: Vec<String>,
    remove_denoms: Vec<String>,
) -> Result<Response, ContractError> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    for denom in add_denoms.iter() {
        validate_native_denom(&denom)?;
        WHITELIST.insert(deps.storage, &denom)?;
    }

    let config = CONFIG.load(deps.storage)?;

    for denom in remove_denoms.iter() {
        // Before removing from whitelist we must handle ongoing incentives,
        // i.e. update the incentive index, and remove any emissions.
        // So we first get all keys by in the INCENTIVE_STATES Map and then filter out the ones
        // that match the incentive denom we are removing.
        // This could be done more efficiently if we could prefix by incentive_denom, but
        // the map key is (collateral_denom, incentive_denom) so we can't, without introducing
        // another map, or using IndexedMap.
        let keys = INCENTIVE_STATES
            .keys(deps.storage, None, None, Order::Ascending)
            .filter(|res| {
                res.as_ref().map_or_else(|_| false, |(_, incentive_denom)| incentive_denom == denom)
            })
            .collect::<StdResult<Vec<_>>>()?;
        for (collateral_denom, incentive_denom) in keys {
            let total_collateral = helpers::query_red_bank_total_collateral(
                deps.as_ref(),
                &config.address_provider,
                &collateral_denom,
            )?;
            update_incentive_index(
                &mut deps.branch().storage.into(),
                &collateral_denom,
                &incentive_denom,
                total_collateral,
                env.block.time.seconds(),
            )?;

            // Remove any incentive emissions
            let emissions = EMISSIONS
                .prefix((&collateral_denom, &incentive_denom))
                .range(deps.storage, None, None, Order::Ascending)
                .collect::<StdResult<Vec<_>>>()?;
            for (start_time, _) in emissions {
                EMISSIONS.remove(deps.storage, (&collateral_denom, &incentive_denom, start_time));
            }
        }

        // Finally remove the incentive denom from the whitelist
        WHITELIST.remove(deps.storage, &denom)?;
    }

    let mut event = Event::new("mars/incentives/update_whitelist");
    if !add_denoms.is_empty() {
        event = event.add_attribute("add_denoms", add_denoms.join(",").to_string());
    }
    if !remove_denoms.is_empty() {
        event = event.add_attribute("remove_denoms", remove_denoms.join(",").to_string());
    }

    Ok(Response::default().add_event(event))
}

pub fn execute_set_asset_incentive(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collateral_denom: String,
    incentive_denom: String,
    emission_per_second: Uint128,
    start_time: u64,
    duration: u64,
) -> Result<Response, ContractError> {
    validate_native_denom(&collateral_denom)?;
    validate_native_denom(&incentive_denom)?;

    // Check that the incentive denom is whitelisted
    if !WHITELIST.contains(deps.storage, &incentive_denom) {
        return Err(ContractError::NotWhitelisted {
            denom: incentive_denom,
        });
    }

    let config = CONFIG.load(deps.storage)?;
    let current_time = env.block.time.seconds();

    // Validate incentive schedule
    helpers::validate_incentive_schedule(
        deps.storage,
        &info,
        &config,
        current_time,
        &collateral_denom,
        &incentive_denom,
        emission_per_second,
        start_time,
        duration,
    )?;

    // Update current incentive index
    let total_collateral = helpers::query_red_bank_total_collateral(
        deps.as_ref(),
        &config.address_provider,
        &collateral_denom,
    )?;
    update_incentive_index(
        &mut deps.branch().storage.into(),
        &collateral_denom,
        &incentive_denom,
        total_collateral,
        current_time,
    )?;

    // To simplify the logic and prevent too much gas usage, we split the new schedule into separate
    // schedules that are exactly one epoch long. This way we can easily merge them with existing
    // schedules.
    // Loop over each epoch duration of the new schedule and merge into any existing schedules
    let mut epoch_start_time = start_time;
    while epoch_start_time < start_time + duration {
        // Check if an schedule exists for the current epoch. If it does, merge the new schedule
        // with the existing schedule. Else add a new schedule.
        let key = (collateral_denom.as_str(), incentive_denom.as_str(), epoch_start_time);
        let existing_schedule = EMISSIONS.may_load(deps.storage, key)?;
        if let Some(existing_schedule) = existing_schedule {
            EMISSIONS.save(deps.storage, key, &(existing_schedule + emission_per_second))?;
        } else {
            EMISSIONS.save(deps.storage, key, &emission_per_second)?;
        }

        epoch_start_time += config.epoch_duration;
    }

    // Set up the incentive state if it doesn't exist
    INCENTIVE_STATES.update(deps.storage, (&collateral_denom, &incentive_denom), |old| {
        Ok::<_, StdError>(old.unwrap_or_else(|| IncentiveState {
            index: Decimal::zero(),
            last_updated: current_time,
        }))
    })?;

    let response = Response::new().add_attributes(vec![
        attr("action", "set_asset_incentive"),
        attr("collateral_denom", collateral_denom),
        attr("incentive_denom", incentive_denom),
        attr("emission_per_second", emission_per_second),
        attr("start_time", start_time.to_string()),
        attr("duration", duration.to_string()),
    ]);
    Ok(response)
}

pub fn execute_balance_change(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user_addr: Addr,
    collateral_denom: String,
    user_amount_scaled_before: Uint128,
    total_amount_scaled_before: Uint128,
) -> Result<Response, ContractError> {
    // this method can only be invoked by the Red Bank contract
    let red_bank_addr = query_red_bank_address(deps.as_ref())?;
    if info.sender != red_bank_addr {
        return Err(MarsError::Unauthorized {}.into());
    }

    let base_event = Event::new("mars/incentives/balance_change")
        .add_attribute("action", "balance_change")
        .add_attribute("denom", collateral_denom.clone())
        .add_attribute("user", user_addr.to_string());
    let mut events = vec![base_event];

    let incentive_states = INCENTIVE_STATES
        .prefix(&collateral_denom)
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    for (incentive_denom, _) in incentive_states {
        let incentive_state = update_incentive_index(
            &mut deps.branch().storage.into(),
            &collateral_denom,
            &incentive_denom,
            total_amount_scaled_before,
            env.block.time.seconds(),
        )?;

        // Check if user has accumulated uncomputed rewards (which means index is not up to date)
        let user_asset_index_key =
            USER_ASSET_INDICES.key((&user_addr, &collateral_denom, &incentive_denom));

        let user_asset_index =
            user_asset_index_key.may_load(deps.storage)?.unwrap_or_else(Decimal::zero);

        let mut accrued_rewards = Uint128::zero();

        if user_asset_index != incentive_state.index {
            // Compute user accrued rewards and update state
            accrued_rewards = compute_user_accrued_rewards(
                user_amount_scaled_before,
                user_asset_index,
                incentive_state.index,
            )?;

            // Store user accrued rewards as unclaimed
            if !accrued_rewards.is_zero() {
                state::increase_unclaimed_rewards(
                    deps.storage,
                    &user_addr,
                    &collateral_denom,
                    &incentive_denom,
                    accrued_rewards,
                )?;
            }

            user_asset_index_key.save(deps.storage, &incentive_state.index)?;
        }

        events.push(
            Event::new("mars/incentives/balance_change/reward_accrued")
                .add_attribute("incentive_denom", incentive_denom)
                .add_attribute("rewards_accrued", accrued_rewards)
                .add_attribute("asset_index", incentive_state.index.to_string()),
        );
    }

    Ok(Response::new().add_events(events))
}

pub fn execute_claim_rewards(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    start_after_collateral_denom: Option<String>,
    start_after_incentive_denom: Option<String>,
    limit: Option<u32>,
) -> Result<Response, ContractError> {
    let red_bank_addr = query_red_bank_address(deps.as_ref())?;
    let user_addr = info.sender;

    let mut response = Response::new();
    let base_event = Event::new("mars/incentives/claim_rewards")
        .add_attribute("action", "claim_rewards")
        .add_attribute("user", user_addr.to_string());
    let mut events = vec![base_event];

    let asset_incentives = state::paginate_incentive_states(
        deps.storage,
        start_after_collateral_denom,
        start_after_incentive_denom,
        limit,
    )?;

    let mut total_unclaimed_rewards: HashMap<String, Uint128> = HashMap::new();

    for ((collateral_denom, incentive_denom), _) in asset_incentives {
        let querier = deps.querier.clone();
        let unclaimed_rewards = compute_user_unclaimed_rewards(
            &mut deps.branch().storage.into(),
            &querier,
            &env.block,
            &red_bank_addr,
            &user_addr,
            &collateral_denom,
            &incentive_denom,
        )?;

        // clear unclaimed rewards
        USER_UNCLAIMED_REWARDS.save(
            deps.storage,
            (&user_addr, &collateral_denom, &incentive_denom),
            &Uint128::zero(),
        )?;

        total_unclaimed_rewards
            .entry(incentive_denom)
            .and_modify(|amount| *amount += unclaimed_rewards)
            .or_insert(unclaimed_rewards);
    }

    for (denom, amount) in total_unclaimed_rewards.iter() {
        response = response.add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: user_addr.to_string(),
            amount: coins(amount.u128(), denom),
        }));
        events.push(
            Event::new("mars/incentives/claim_rewards/claimed_reward")
                .add_attribute("denom", denom)
                .add_attribute("amount", *amount),
        );
    }

    Ok(response.add_events(events))
}

pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address_provider: Option<String>,
) -> Result<Response, ContractError> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    let mut config = CONFIG.load(deps.storage)?;

    config.address_provider =
        option_string_to_addr(deps.api, address_provider, config.address_provider)?;

    CONFIG.save(deps.storage, &config)?;

    let response = Response::new().add_attribute("action", "update_config");

    Ok(response)
}

fn update_owner(
    deps: DepsMut,
    info: MessageInfo,
    update: OwnerUpdate,
) -> Result<Response, ContractError> {
    Ok(OWNER.update(deps, info, update)?)
}

// QUERIES

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::IncentiveState {
            collateral_denom,
            incentive_denom,
        } => to_binary(&query_incentive_state(deps, collateral_denom, incentive_denom)?),
        QueryMsg::IncentiveStates {
            start_after_collateral_denom,
            start_after_incentive_denom,
            limit,
        } => to_binary(&query_incentive_states(
            deps,
            start_after_collateral_denom,
            start_after_incentive_denom,
            limit,
        )?),
        QueryMsg::UserUnclaimedRewards {
            user,
            start_after_collateral_denom,
            start_after_incentive_denom,
            limit,
        } => to_binary(&query_user_unclaimed_rewards(
            deps,
            env,
            user,
            start_after_collateral_denom,
            start_after_incentive_denom,
            limit,
        )?),
        QueryMsg::Whitelist {} => to_binary(&query_whitelist(deps)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let owner_state = OWNER.query(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: owner_state.owner,
        proposed_new_owner: owner_state.proposed,
        address_provider: config.address_provider,
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

pub fn query_user_unclaimed_rewards(
    deps: Deps,
    env: Env,
    user: String,
    start_after_collateral_denom: Option<String>,
    start_after_incentive_denom: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<Coin>> {
    let red_bank_addr = query_red_bank_address(deps)?;
    let user_addr = deps.api.addr_validate(&user)?;

    let incentive_states = state::paginate_incentive_states(
        deps.storage,
        start_after_collateral_denom,
        start_after_incentive_denom,
        limit,
    )?;

    let mut total_unclaimed_rewards: HashMap<String, Uint128> = HashMap::new();

    for ((collateral_denom, incentive_denom), _) in incentive_states {
        let unclaimed_rewards = compute_user_unclaimed_rewards(
            &mut deps.storage.into(),
            &deps.querier,
            &env.block,
            &red_bank_addr,
            &user_addr,
            &collateral_denom,
            &incentive_denom,
        )?;
        total_unclaimed_rewards
            .entry(incentive_denom)
            .and_modify(|amount| *amount += unclaimed_rewards)
            .or_insert(unclaimed_rewards);
    }

    Ok(total_unclaimed_rewards
        .into_iter()
        .map(|(denom, amount)| Coin {
            denom,
            amount,
        })
        .collect())
}

fn query_red_bank_address(deps: Deps) -> StdResult<Addr> {
    let config = CONFIG.load(deps.storage)?;
    address_provider::helpers::query_contract_addr(
        deps,
        &config.address_provider,
        MarsAddressType::RedBank,
    )
}

fn query_whitelist(deps: Deps) -> StdResult<Vec<String>> {
    let whitelist: Vec<String> =
        WHITELIST.items(deps.storage, None, None, Order::Ascending).collect::<StdResult<_>>()?;
    Ok(whitelist)
}
