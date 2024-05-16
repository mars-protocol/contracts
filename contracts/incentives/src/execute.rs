use std::collections::HashMap;

use astroport::incentives::ExecuteMsg as AstroExecuteMsg;
use cosmwasm_std::{
    attr, to_json_binary, Addr, BankMsg, Coin, Coins, CosmosMsg, Decimal, DepsMut, Env, Event,
    MessageInfo, Order, Order::Ascending, Response, StdError, StdResult, Storage, Uint128, WasmMsg,
};
use mars_owner::OwnerUpdate;
use mars_types::{
    address_provider,
    address_provider::MarsAddressType,
    error::MarsError,
    incentives::{IncentiveState, WhitelistEntry},
    keys::{UserId, UserIdKey},
};
use mars_types::incentives::LpModification;
use mars_utils::helpers::{option_string_to_addr, validate_native_denom};

use crate::{
    helpers,
    helpers::{
        assert_caller_is_credit_manager, calculate_rewards_from_astroport_incentive_state,
        claim_rewards_msg, compute_updated_astroport_incentive_states,
        compute_user_accrued_rewards, compute_user_unclaimed_rewards, update_incentive_index,
    },
    query::{query_red_bank_address, query_unclaimed_astroport_rewards},
    state,
    state::{
        ASTROPORT_INCENTIVE_STATES, CONFIG, EMISSIONS, EPOCH_DURATION, INCENTIVE_STATES,
        LP_DEPOSITS, OWNER, TOTAL_LP_DEPOSITS, USER_ASSET_INDICES, USER_ASTROPORT_INCENTIVE_STATES,
        USER_UNCLAIMED_REWARDS, WHITELIST, WHITELIST_COUNT,
    },
    ContractError,
    ContractError::NoStakedLp,
};

pub fn execute_update_whitelist(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    add_denoms: Vec<WhitelistEntry>,
    remove_denoms: Vec<String>,
) -> Result<Response, ContractError> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    let config = CONFIG.load(deps.storage)?;

    // Add add_denoms and remove_denoms to a set to check for duplicates
    let denoms = add_denoms.iter().map(|entry| &entry.denom).chain(remove_denoms.iter());
    let mut denoms_set = std::collections::HashSet::new();
    for denom in denoms {
        if !denoms_set.insert(denom) {
            return Err(ContractError::DuplicateDenom {
                denom: denom.clone(),
            });
        }
    }

    let prev_whitelist_count = WHITELIST_COUNT.may_load(deps.storage)?.unwrap_or_default();
    let mut whitelist_count = prev_whitelist_count;

    for denom in remove_denoms.iter() {
        // If denom is not on the whitelist, we can't remove it
        if !WHITELIST.has(deps.storage, denom) {
            return Err(ContractError::NotWhitelisted {
                denom: denom.clone(),
            });
        }

        whitelist_count -= 1;

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
        WHITELIST.remove(deps.storage, denom);
    }

    for entry in add_denoms.iter() {
        let WhitelistEntry {
            denom,
            min_emission_rate,
        } = entry;
        // If the denom is not already whitelisted, increase the counter and check that we are not
        // exceeding the max whitelist limit. If the denom is already whitelisted, we don't need
        // to change the counter and instead just update the min_emission.
        if !WHITELIST.has(deps.storage, denom) {
            whitelist_count += 1;
            if whitelist_count > config.max_whitelisted_denoms {
                return Err(ContractError::MaxWhitelistLimitReached {
                    max_whitelist_limit: config.max_whitelisted_denoms,
                });
            }
        }

        validate_native_denom(denom)?;
        WHITELIST.save(deps.storage, denom, min_emission_rate)?;
    }

    // Set the new whitelist count, if it has changed
    if whitelist_count != prev_whitelist_count {
        WHITELIST_COUNT.save(deps.storage, &whitelist_count)?;
    }

    let mut event = Event::new("mars/incentives/update_whitelist");
    event = event.add_attribute("add_denoms", format!("{:?}", add_denoms));
    event = event.add_attribute("remove_denoms", format!("{:?}", remove_denoms));

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
    if !WHITELIST.key(&incentive_denom).has(deps.storage) {
        return Err(ContractError::NotWhitelisted {
            denom: incentive_denom,
        });
    }

    let config = CONFIG.load(deps.storage)?;
    let epoch_duration = EPOCH_DURATION.load(deps.storage)?;
    let current_time = env.block.time.seconds();

    // Validate incentive schedule
    helpers::validate_incentive_schedule(
        deps.storage,
        &info,
        epoch_duration,
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

        epoch_start_time += epoch_duration;
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
    account_id: Option<String>,
    collateral_denom: String,
    user_amount_scaled_before: Uint128,
    total_amount_scaled_before: Uint128,
) -> Result<Response, ContractError> {
    // this method can only be invoked by the Red Bank contract
    let red_bank_addr = query_red_bank_address(deps.as_ref())?;
    if info.sender != red_bank_addr {
        return Err(MarsError::Unauthorized {}.into());
    }

    let acc_id = account_id.clone().unwrap_or("".to_string());

    let user_id = UserId::credit_manager(user_addr.clone(), acc_id.clone());
    let user_id_key: UserIdKey = user_id.try_into()?;

    let base_event = Event::new("mars/incentives/balance_change")
        .add_attribute("action", "balance_change")
        .add_attribute("denom", collateral_denom.clone())
        .add_attribute("user", user_addr.to_string());
    let base_event = if account_id.is_some() {
        base_event.add_attribute("account_id", &acc_id)
    } else {
        base_event
    };
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
            USER_ASSET_INDICES.key((&user_id_key.clone(), &collateral_denom, &incentive_denom));

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
                    &acc_id,
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

// ASTROPORT INCENTIVES

/// Fetch the new rewards from astroport, and update our global incentive states.
fn claim_rewards_from_astro(
    deps: &mut DepsMut,
    astroport_incentives_addr: &str,
    mars_incentives_addr: &str,
    account_id: &str,
    lp_denom: &str,
) -> Result<Response, ContractError> {
    let pending_rewards: Vec<Coin> = query_unclaimed_astroport_rewards(
        deps.as_ref(),
        &mars_incentives_addr,
        astroport_incentives_addr,
        &lp_denom,
    )
    .unwrap_or(vec![]);

    let res = update_lp_incentive_states(deps.storage, &lp_denom, &account_id, pending_rewards)?;
    let mut modification_event = Event::new("mars/incentives/claimed_astro_incentive_rewards");


    Ok(res.add_message(claim_rewards_msg(&astroport_incentives_addr, &lp_denom)?))
}

pub fn execute_unstake_astro_lp(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account_id: String,
    lp_coin: Coin,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let addresses = address_provider::helpers::query_contract_addrs(
        deps.as_ref(),
        &config.address_provider,
        vec![MarsAddressType::AstroportIncentives, MarsAddressType::CreditManager],
    )?;

    assert_caller_is_credit_manager(info.sender, &addresses[&MarsAddressType::CreditManager])?;

    update_user_lp_position(
        &mut deps,
        &account_id,
        lp_coin,
        &addresses[&MarsAddressType::AstroportIncentives].to_string(),
        env.contract.address.as_str(),
        &addresses[&MarsAddressType::CreditManager].to_string(),
        LpModification::Withdraw,
    )
}

pub fn execute_stake_astro_lp(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account_id: String,
    lp_coin: Coin,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let addresses = address_provider::helpers::query_contract_addrs(
        deps.as_ref(),
        &config.address_provider,
        vec![MarsAddressType::AstroportIncentives, MarsAddressType::CreditManager],
    )?;

    assert_caller_is_credit_manager(info.sender, &addresses[&MarsAddressType::CreditManager])?;

    update_user_lp_position(
        &mut deps,
        &account_id,
        lp_coin,
        &addresses[&MarsAddressType::AstroportIncentives].as_str(),
        &env.contract.address.to_string(),
        &addresses[&MarsAddressType::CreditManager].to_string(),
        LpModification::Deposit,
    )
}

fn update_user_lp_position(
    deps: &mut DepsMut,
    account_id: &str,
    lp_coin: Coin,
    astroport_incentives_addr: &str,
    mars_incentives_addr: &str,
    credit_manager_addr: &str,
    modification: LpModification,
) -> Result<Response, ContractError> {
    let staked_lp_amount = LP_DEPOSITS
        .may_load(deps.storage, (&account_id, &lp_coin.denom))?
        .unwrap_or(Uint128::zero());

    // Claim all rewards from astroport before any modification
    let mut res = claim_astro_rewards_for_lp_position(
        deps,
        &astroport_incentives_addr,
        &mars_incentives_addr,
        credit_manager_addr,
        &account_id,
        &lp_coin.denom,
        staked_lp_amount,
    )?;

    res = match modification {
        // Deposit stakes lp coin in astroport incentives
        LpModification::Deposit => {
            // Update our accounting
            increment_lp_deposit(deps.storage, &account_id, &lp_coin)?;

            // stake in astroport incentives
            res.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: astroport_incentives_addr.to_string(),
                funds: vec![lp_coin],
                msg: to_json_binary(&AstroExecuteMsg::Deposit {
                    recipient: Some(mars_incentives_addr.to_string()),
                })?,
            }))
        }

        LpModification::Withdraw => {
            // Update our lp amount accounting
            decrement_lp_deposit(deps.storage, &account_id, &lp_coin)?;

            // Add two messages
            // - unstake from astroport incentives (lp_amount)
            // - send to credit manager (lp_amount)
            res.add_messages([
                // Withdraw from astroport lp staking
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: astroport_incentives_addr.to_string(),
                    msg: to_json_binary(&AstroExecuteMsg::Withdraw {
                        lp_token: (&lp_coin.denom).to_string(),
                        amount: lp_coin.amount,
                    })?,
                    funds: vec![lp_coin.clone()],
                }),
                // Send lp_coins to credit manager
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: credit_manager_addr.to_string(),
                    amount: vec![lp_coin],
                }),
            ])
        }
    };

    let modification_event = Event::new("mars/incentives/update_user_lp_position")
        .add_attribute("action", modification)
        .add_attribute("account_id", account_id.to_string())
        .add_attribute("lp_amount", lp_coin.amount)
        .add_attribute("lp_denom", lp_coin.denom);

    Ok(res.add_event(modification_event))
}

fn increment_lp_deposit(
    store: &mut dyn Storage,
    account_id: &str,
    lp_coin: &Coin,
) -> Result<(), ContractError> {
    // Update user staked lp state
    LP_DEPOSITS.update(store, (&account_id, &lp_coin.denom), |existing| -> StdResult<_> {
        Ok(existing.unwrap_or_default().checked_add(lp_coin.amount)?)
    })?;

    // Update total staked lp state
    TOTAL_LP_DEPOSITS.update(store, &lp_coin.denom, |existing| -> StdResult<_> {
        Ok(existing.unwrap_or_default().checked_add(lp_coin.amount)?)
    })?;

    Ok(())
}

fn decrement_lp_deposit(
    store: &mut dyn Storage,
    account_id: &str,
    lp_coin: &Coin,
) -> Result<(), ContractError> {

    // Update user staked lp state
    LP_DEPOSITS.update(store, (&account_id, &lp_coin.denom), |existing| -> StdResult<_> {
        Ok(existing
            .expect("lp position should exist")
            .checked_sub(lp_coin.amount)?)
    })?;

    // Update total staked lp state
    TOTAL_LP_DEPOSITS.update(store, &lp_coin.denom, |existing| -> StdResult<_> {
        Ok(existing
            .expect("lp position total should exist")
            .checked_add(lp_coin.amount)?)
    })?;

    Ok(())
}

fn update_lp_incentive_states(
    storage: &mut dyn Storage,
    lp_denom: &str,
    account_id: &str,
    pending_rewards: Vec<Coin>,
) -> Result<Response, ContractError> {

    // Update our global indexes for each reward. We only accept native tokens, cw20 will be ignored
    let updated_incentives: HashMap<String, Decimal> =
        compute_updated_astroport_incentive_states(storage, pending_rewards, lp_denom)?;

    for (incentive_denom, updated_incentive) in updated_incentives.iter() {

        // Set user incentive to latest, as we claim every action
        USER_ASTROPORT_INCENTIVE_STATES.save(
            storage,
            (&account_id, &lp_denom, &incentive_denom),
            &updated_incentive,
        )?;
        // Store latest state
        ASTROPORT_INCENTIVE_STATES.save(
            storage,
            (&lp_denom, &incentive_denom),
            &updated_incentive,
        )?;


    }

    return Ok(Response::new());
}

pub fn execute_claim_astro_rewards_for_lp_position(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account_id: &str,
    lp_denom: &str,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let addresses = address_provider::helpers::query_contract_addrs(
        deps.as_ref(),
        &config.address_provider,
        vec![MarsAddressType::AstroportIncentives, MarsAddressType::CreditManager],
    )?;
    let astroport_incentives_addr = &addresses[&MarsAddressType::AstroportIncentives];
    let credit_manager_addr = &addresses[&MarsAddressType::CreditManager];

    // To prevent configuration errors, we fetch address from current contract instead of address_provider
    let mars_incentives_addr = env.contract.address.to_string();

    if info.sender != credit_manager_addr {
        return Err(ContractError::Mars(MarsError::Unauthorized {}));
    }

    let staked_lp_amount =
        LP_DEPOSITS.may_load(deps.storage, (&account_id, &lp_denom))?.ok_or(NoStakedLp {
            account_id: account_id.to_string(),
            denom: lp_denom.to_string(),
        })?;

    claim_astro_rewards_for_lp_position(
        &mut deps,
        astroport_incentives_addr.as_str(),
        &mars_incentives_addr,
        credit_manager_addr.as_str(),
        account_id,
        lp_denom,
        staked_lp_amount,
    )
}

/// Claims astroport rewards for a user.
///
/// Response returned includes msg to send rewards to credit manager
fn claim_astro_rewards_for_lp_position(
    deps: &mut DepsMut,
    astroport_incentives_addr: &str,
    mars_incentives_addr: &str,
    credit_manager_addr: &str,
    account_id: &str,
    lp_denom: &str,
    staked_lp_amount: Uint128,
) -> Result<Response, ContractError> {
    let mut res = claim_rewards_from_astro(
        deps,
        astroport_incentives_addr,
        mars_incentives_addr,
        account_id,
        lp_denom,
    )?;

    let mut event = Event::new("mars/incentives/claimed_lp_rewards")
        .add_attribute("account_id", account_id.to_string());

    res = if staked_lp_amount != Uint128::zero() {
        let user_claimable_rewards =
            calculate_claimable_rewards(deps.storage, account_id, lp_denom, staked_lp_amount)?;

        // Send the claimed rewards to the credit manager
        let send_rewards_to_cm_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: credit_manager_addr.to_string(),
            amount: user_claimable_rewards,
        });

        for coin in user_claimable_rewards {
            event = event
                .add_attribute("denom", coin.denom)
                .add_attribute("amount", coin.amount);
        }

        res.add_message(send_rewards_to_cm_msg)
    } else {
        res
    };

    Ok(res.add_event(event))
}

fn calculate_claimable_rewards(
    storage: &dyn Storage,
    account_id: &str,
    lp_denom: &str,
    staked_lp_amount: Uint128,
) -> Result<Vec<Coin>, ContractError> {
    let lp_coin = Coin {
        amount: staked_lp_amount,
        denom: lp_denom.to_string(),
    };

    let lp_incentive_states: HashMap<String, Decimal> = ASTROPORT_INCENTIVE_STATES
        .prefix(lp_denom)
        .range(storage, None, None, Ascending)
        .collect::<StdResult<HashMap<String, Decimal>>>()?;

    calculate_rewards_from_astroport_incentive_state(
        storage,
        account_id,
        &lp_coin,
        lp_incentive_states,
    )
}

pub fn execute_claim_rewards(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account_id: Option<String>,
    start_after_collateral_denom: Option<String>,
    start_after_incentive_denom: Option<String>,
    limit: Option<u32>,
) -> Result<Response, ContractError> {
    let user_addr = info.sender;
    let acc_id = account_id.clone().unwrap_or("".to_string());
    let user_id = UserId::credit_manager(user_addr.clone(), acc_id.clone());
    let user_id_key: UserIdKey = user_id.try_into()?;

    let red_bank_addr = query_red_bank_address(deps.as_ref())?;

    let mut response = Response::new();
    let base_event = Event::new("mars/incentives/claim_rewards")
        .add_attribute("action", "claim_rewards")
        .add_attribute("user", user_addr.to_string());
    let base_event = if account_id.is_some() {
        base_event.add_attribute("account_id", &acc_id)
    } else {
        base_event
    };
    response = response.add_event(base_event);

    let asset_incentives = state::paginate_incentive_states(
        deps.storage,
        start_after_collateral_denom,
        start_after_incentive_denom,
        limit,
    )?;

    let mut total_unclaimed_rewards = Coins::default();

    for ((collateral_denom, incentive_denom), _) in asset_incentives {
        let querier = deps.querier;
        let unclaimed_rewards = compute_user_unclaimed_rewards(
            &mut deps.branch().storage.into(),
            &querier,
            &env.block,
            &red_bank_addr,
            &user_addr,
            &account_id,
            &collateral_denom,
            &incentive_denom,
        )?;

        // clear unclaimed rewards
        USER_UNCLAIMED_REWARDS.save(
            deps.storage,
            (&user_id_key, &collateral_denom, &incentive_denom),
            &Uint128::zero(),
        )?;

        total_unclaimed_rewards.add(Coin {
            denom: incentive_denom,
            amount: unclaimed_rewards,
        })?;
    }

    if !total_unclaimed_rewards.is_empty() {
        response = response
            .add_event(
                Event::new("mars/incentives/claim_rewards/claimed_rewards")
                    .add_attribute("coins", total_unclaimed_rewards.to_string()),
            )
            .add_message(BankMsg::Send {
                to_address: user_addr.into(),
                amount: total_unclaimed_rewards.into(),
            });
    }

    Ok(response)
}

// CONFIG

pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address_provider: Option<String>,
    max_whitelisted_denoms: Option<u8>,
) -> Result<Response, ContractError> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    let mut config = CONFIG.load(deps.storage)?;

    config.address_provider =
        option_string_to_addr(deps.api, address_provider, config.address_provider)?;

    if let Some(max_whitelisted_denoms) = max_whitelisted_denoms {
        config.max_whitelisted_denoms = max_whitelisted_denoms;
    }

    CONFIG.save(deps.storage, &config)?;

    let response = Response::new().add_attribute("action", "update_config");

    Ok(response)
}

pub fn update_owner(
    deps: DepsMut,
    info: MessageInfo,
    update: OwnerUpdate,
) -> Result<Response, ContractError> {
    Ok(OWNER.update(deps, info, update)?)
}
