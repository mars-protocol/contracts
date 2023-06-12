use std::collections::HashMap;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, coins, to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env,
    Event, MessageInfo, Order, Response, StdResult, Uint128,
};
use cw_storage_plus::Bound;
use mars_owner::{OwnerInit::SetInitialOwner, OwnerUpdate};
use mars_red_bank_types::{
    address_provider::{self, MarsAddressType},
    error::MarsError,
    incentives::{
        Config, ConfigResponse, ExecuteMsg, IncentiveSchedule, IncentiveStateResponse,
        InstantiateMsg, QueryMsg,
    },
    red_bank,
};
use mars_utils::helpers::{option_string_to_addr, validate_native_denom};

use crate::{
    error::ContractError,
    helpers::{
        self, compute_user_accrued_rewards, compute_user_unclaimed_rewards, update_incentive_index,
    },
    state::{
        self, CONFIG, INCENTIVE_SCHEDULES, INCENTIVE_STATES, OWNER, USER_ASSET_INDICES,
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
        mars_denom: msg.mars_denom,
        epoch_duration: msg.epoch_duration,
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
            mars_denom,
        } => Ok(execute_update_config(deps, env, info, address_provider, mars_denom)?),
        ExecuteMsg::UpdateOwner(update) => update_owner(deps, info, update),
    }
}

pub fn execute_update_whitelist(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    add_denoms: Vec<String>,
    remove_denoms: Vec<String>,
) -> Result<Response, ContractError> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    for denom in add_denoms.iter() {
        validate_native_denom(&denom)?;
        WHITELIST.insert(deps.storage, &denom)?;
    }

    for denom in remove_denoms.iter() {
        // TODO: Handle ongoing incentives
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

    // Query Red Bank to check if market exists
    let config = CONFIG.load(deps.storage)?;
    let red_bank_addr = address_provider::helpers::query_contract_addr(
        deps.as_ref(),
        &config.address_provider,
        MarsAddressType::RedBank,
    )?;
    let market: red_bank::Market = deps
        .querier
        .query_wasm_smart(
            &red_bank_addr,
            &red_bank::QueryMsg::Market {
                denom: collateral_denom.to_string(),
            },
        )
        .map_err(|_| ContractError::InvalidIncentive {
            reason: "Market does not exist on Red Bank".to_string(),
        })?;

    let current_time = env.block.time.seconds();

    // Validate incentive schedule
    let mut new_schedule = IncentiveSchedule {
        start_time,
        duration,
        emission_per_second,
    };
    helpers::validate_incentive_schedule(
        deps.storage,
        &info,
        &config,
        current_time,
        &new_schedule,
        &collateral_denom,
        &incentive_denom,
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

    // If the new schedule overlaps any existing schedules, we merge them so that there is at most
    // one schedule per epoch.
    // First, find all existing schdules that overlap with the new schedule
    let overlapping_schedules = INCENTIVE_SCHEDULES
        .prefix((&collateral_denom, &incentive_denom))
        .range(deps.storage, None, Some(Bound::exclusive(start_time + duration)), Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    // For each overlapping schedule, add the emission to the existing incentive and modify the new
    // incentive, as well as creating new incentives for any gaps between the new incentive and the
    // existing incentives.
    for (_, mut old_schedule) in overlapping_schedules {
        if old_schedule.start_time > new_schedule.start_time {
            // The new schdule starts before the existing schedule, so we need to create a new
            // schedule for the time before the existing schedule starts
            INCENTIVE_SCHEDULES.save(
                deps.storage,
                (&collateral_denom, &incentive_denom, new_schedule.start_time),
                &IncentiveSchedule {
                    emission_per_second: new_schedule.emission_per_second,
                    start_time: new_schedule.start_time,
                    duration: old_schedule.start_time - new_schedule.start_time,
                },
            )?;
            new_schedule.duration -= old_schedule.start_time - new_schedule.start_time;
            new_schedule.start_time = old_schedule.start_time;
        }

        old_schedule.emission_per_second += new_schedule.emission_per_second;

        INCENTIVE_SCHEDULES.save(
            deps.storage,
            (&collateral_denom, &incentive_denom, old_schedule.start_time),
            &old_schedule,
        )?;

        let remaining_duration = new_schedule.duration.saturating_sub(old_schedule.duration);
        new_schedule.duration = remaining_duration;
        new_schedule.start_time = old_schedule.start_time + old_schedule.duration;

        if remaining_duration == 0 {
            // The new incentive is fully covered by the existing incentive, so we can stop
            // processing
            break;
        }
    }

    // If there is any remaining duration on the new incentive, we save it
    if new_schedule.duration > 0 {
        INCENTIVE_SCHEDULES.save(
            deps.storage,
            (&collateral_denom, &incentive_denom, new_schedule.start_time),
            &new_schedule,
        )?;
    }

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
    mars_denom: Option<String>,
) -> Result<Response, ContractError> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    if let Some(md) = &mars_denom {
        validate_native_denom(md)?;
    };

    let mut config = CONFIG.load(deps.storage)?;

    config.address_provider =
        option_string_to_addr(deps.api, address_provider, config.address_provider)?;
    config.mars_denom = mars_denom.unwrap_or(config.mars_denom);

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
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let owner_state = OWNER.query(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: owner_state.owner,
        proposed_new_owner: owner_state.proposed,
        address_provider: config.address_provider,
        mars_denom: config.mars_denom,
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
