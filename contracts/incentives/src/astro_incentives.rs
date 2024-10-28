use std::collections::HashMap;

use astroport_v5::incentives::ExecuteMsg;
use cosmwasm_std::{
    ensure_eq, to_json_binary, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, Env, Event, MessageInfo,
    Order::{self, Ascending},
    Response, StdResult, Storage, Uint128, WasmMsg,
};
use mars_types::{
    address_provider::{helpers::query_contract_addrs, MarsAddressType},
    credit_manager::{ActionAmount, ActionCoin},
    error::MarsError,
    incentives::LpModification,
};

use crate::{
    helpers::{
        calculate_rewards_for_staked_astro_lp_position, claim_rewards_msg,
        compute_updated_astro_incentive_states, MaybeMutStorage,
    },
    query::query_unclaimed_astro_lp_rewards,
    state::{
        ASTRO_INCENTIVE_STATES, ASTRO_TOTAL_LP_DEPOSITS, ASTRO_USER_LP_DEPOSITS, CONFIG,
        USER_ASTRO_INCENTIVE_STATES,
    },
    ContractError::{self, NoStakedLp},
};

/// Fetches all pending rewards from all users LP in astroport, and updates the lp incentive states
fn claim_global_staked_lp_rewards(
    deps: &mut DepsMut,
    astroport_incentives_addr: &str,
    mars_incentives_addr: &str,
    lp_denom: &str,
) -> Result<Response, ContractError> {
    let pending_rewards: Vec<Coin> = query_unclaimed_astro_lp_rewards(
        deps.as_ref(),
        mars_incentives_addr,
        astroport_incentives_addr,
        lp_denom,
    )?;

    let res = update_incentive_states_for_lp_denom(deps.storage, lp_denom, pending_rewards)?;

    Ok(res
        .add_event(Event::new("mars/incentives/claimed_astro_incentive_rewards"))
        .add_message(claim_rewards_msg(astroport_incentives_addr, lp_denom)?))
}

pub fn execute_unstake_lp(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account_id: String,
    lp_coin: ActionCoin,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let addresses = query_contract_addrs(
        deps.as_ref(),
        &config.address_provider,
        vec![MarsAddressType::AstroportIncentives, MarsAddressType::CreditManager],
    )?;

    let astroport_incentives_addr = &addresses[&MarsAddressType::AstroportIncentives];
    let credit_manager_addr = &addresses[&MarsAddressType::CreditManager];

    ensure_eq!(info.sender, credit_manager_addr, ContractError::Mars(MarsError::Unauthorized {}));

    let amount = match lp_coin.amount {
        ActionAmount::Exact(amount) => amount,
        ActionAmount::AccountBalance => ASTRO_USER_LP_DEPOSITS
            .may_load(deps.storage, (&account_id, &lp_coin.denom))?
            .unwrap_or(Uint128::zero()),
    };

    if amount.is_zero() {
        return Err(NoStakedLp {
            account_id: account_id.clone(),
            denom: lp_coin.denom.clone(),
        });
    }

    let lp_coin_checked = Coin {
        denom: lp_coin.denom,
        amount,
    };

    update_user_lp_position(
        deps,
        &account_id,
        lp_coin_checked,
        astroport_incentives_addr.as_ref(),
        env.contract.address.as_str(),
        credit_manager_addr.as_ref(),
        LpModification::Withdraw,
    )
}

pub fn execute_stake_lp(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account_id: String,
    lp_coin: Coin,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let addresses = query_contract_addrs(
        deps.as_ref(),
        &config.address_provider,
        vec![MarsAddressType::AstroportIncentives, MarsAddressType::CreditManager],
    )?;

    let astroport_incentives_addr = &addresses[&MarsAddressType::AstroportIncentives];
    let credit_manager_addr = &addresses[&MarsAddressType::CreditManager];

    ensure_eq!(info.sender, credit_manager_addr, ContractError::Mars(MarsError::Unauthorized {}));

    update_user_lp_position(
        deps,
        &account_id,
        lp_coin,
        astroport_incentives_addr.as_str(),
        env.contract.address.as_str(),
        credit_manager_addr.as_str(),
        LpModification::Deposit,
    )
}

fn update_user_lp_position(
    mut deps: DepsMut,
    account_id: &str,
    lp_coin: Coin,
    astroport_incentives_addr: &str,
    mars_incentives_addr: &str,
    credit_manager_addr: &str,
    modification: LpModification,
) -> Result<Response, ContractError> {
    // Astroport raises an error if there is no existing position and we query rewards.
    // Therefore, we check first to ensure we don't fail first time somebody stakes
    // https://github.com/astroport-fi/astroport-core/blob/main/contracts/tokenomics/incentives/src/state.rs#L539
    let total_staked_lp_amount =
        ASTRO_TOTAL_LP_DEPOSITS.may_load(deps.storage, &lp_coin.denom)?.unwrap_or(Uint128::zero());

    // Claim all rewards from astroport before any modification
    let mut res = if !total_staked_lp_amount.is_zero() {
        let staked_lp_amount = ASTRO_USER_LP_DEPOSITS
            .may_load(deps.storage, (account_id, &lp_coin.denom))?
            .unwrap_or(Uint128::zero());
        claim_rewards_for_staked_lp_position(
            &mut deps,
            astroport_incentives_addr,
            mars_incentives_addr,
            credit_manager_addr,
            account_id,
            &lp_coin.denom,
            staked_lp_amount,
        )?
    } else {
        Response::new()
    };

    res = match modification {
        // Deposit stakes lp coin in astroport incentives
        LpModification::Deposit => {
            // Update our accounting
            increment_staked_lp(deps.storage, account_id, &lp_coin)?;

            // stake in astroport incentives
            res.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: astroport_incentives_addr.to_string(),
                funds: vec![lp_coin],
                msg: to_json_binary(&ExecuteMsg::Deposit {
                    recipient: Some(mars_incentives_addr.to_string()),
                })?,
            }))
        }

        LpModification::Withdraw => {
            // Update our lp amount accounting
            decrement_staked_lp(deps.storage, account_id, &lp_coin)?;

            // Add two messages
            // - unstake from astroport incentives (lp_amount)
            // - send to credit manager (lp_amount)
            res.add_messages([
                // Withdraw from astroport lp staking
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: astroport_incentives_addr.to_string(),
                    msg: to_json_binary(&ExecuteMsg::Withdraw {
                        lp_token: lp_coin.denom.clone(),
                        amount: lp_coin.amount,
                    })?,
                    funds: vec![],
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
        .add_attribute("account_id", account_id.to_string());

    Ok(res.add_event(modification_event))
}

fn increment_staked_lp(
    store: &mut dyn Storage,
    account_id: &str,
    lp_coin: &Coin,
) -> Result<(), ContractError> {
    // Update user staked lp state
    ASTRO_USER_LP_DEPOSITS.update(
        store,
        (account_id, &lp_coin.denom),
        |existing| -> StdResult<_> {
            Ok(existing.unwrap_or_default().checked_add(lp_coin.amount)?)
        },
    )?;

    // Update total staked lp state
    ASTRO_TOTAL_LP_DEPOSITS.update(store, &lp_coin.denom, |existing| -> StdResult<_> {
        Ok(existing.unwrap_or_default().checked_add(lp_coin.amount)?)
    })?;

    Ok(())
}

fn decrement_staked_lp(
    store: &mut dyn Storage,
    account_id: &str,
    lp_coin: &Coin,
) -> Result<(), ContractError> {
    update_user_staked_lp(store, lp_coin, account_id)?;
    update_total_staked_lp(store, lp_coin)?;

    Ok(())
}

fn update_user_staked_lp(
    store: &mut dyn Storage,
    lp_coin: &Coin,
    account_id: &str,
) -> Result<(), ContractError> {
    let key = (account_id, lp_coin.denom.as_str());
    let existing_amount =
        ASTRO_USER_LP_DEPOSITS.may_load(store, key)?.ok_or_else(|| ContractError::NoStakedLp {
            account_id: account_id.to_string(),
            denom: lp_coin.denom.clone(),
        })?;

    let new_amount = existing_amount.checked_sub(lp_coin.amount)?;
    if new_amount.is_zero() {
        ASTRO_USER_LP_DEPOSITS.remove(store, key);

        // Get all incentives for (user, lp_token_denom) key
        let prefix = USER_ASTRO_INCENTIVE_STATES.prefix((account_id, lp_coin.denom.as_str()));

        // Iterate over all reward_denom keys
        let keys_to_remove =
            prefix.keys(store, None, None, Order::Ascending).collect::<StdResult<Vec<String>>>()?;

        // Delete each matching (account_id, lp_token_denom, reward_denom) incentive.
        for incentive_denom in keys_to_remove {
            USER_ASTRO_INCENTIVE_STATES
                .remove(store, (account_id, lp_coin.denom.as_str(), &incentive_denom));
        }
    } else {
        ASTRO_USER_LP_DEPOSITS.save(store, key, &new_amount)?;
    }

    Ok(())
}

fn update_total_staked_lp(store: &mut dyn Storage, lp_coin: &Coin) -> Result<(), ContractError> {
    let lp_denom = lp_coin.denom.as_str();

    let total_staked_lp_amount =
        ASTRO_TOTAL_LP_DEPOSITS.may_load(store, lp_denom)?.ok_or_else(|| {
            ContractError::NoDeposits {
                denom: lp_coin.denom.clone(),
            }
        })?;

    let new_total_staked_lp_amount = total_staked_lp_amount.checked_sub(lp_coin.amount)?;

    // If the new amount is zero, remove the entry and all associated incentive states
    if new_total_staked_lp_amount.is_zero() {
        ASTRO_TOTAL_LP_DEPOSITS.remove(store, lp_denom);

        // Get all incentive states for the lp_key
        let prefix = ASTRO_INCENTIVE_STATES.prefix(lp_denom);
        let keys_to_remove =
            prefix.keys(store, None, None, Order::Ascending).collect::<StdResult<Vec<String>>>()?;

        // Remove all incentive states related to the lp_key
        for incentive_denom in keys_to_remove {
            ASTRO_INCENTIVE_STATES.remove(store, (lp_denom, incentive_denom.as_str()));
        }
    } else {
        // Save the updated staked amount if it's not zero
        ASTRO_TOTAL_LP_DEPOSITS.save(store, lp_denom, &new_total_staked_lp_amount)?;
    }
    Ok(())
}

fn update_incentive_states_for_lp_denom(
    storage: &mut dyn Storage,
    lp_denom: &str,
    pending_rewards: Vec<Coin>,
) -> Result<Response, ContractError> {
    // Update our global indexes for each reward. We only accept native tokens, cw20 will be ignored
    let updated_incentives: HashMap<String, Decimal> =
        compute_updated_astro_incentive_states(storage, pending_rewards, lp_denom)?;

    for (incentive_denom, updated_incentive) in updated_incentives.iter() {
        // Store latest state
        ASTRO_INCENTIVE_STATES.save(storage, (lp_denom, incentive_denom), updated_incentive)?;
    }

    Ok(Response::new())
}

pub fn execute_claim_rewards_for_staked_lp_position(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account_id: &str,
    lp_denom: &str,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let addresses = query_contract_addrs(
        deps.as_ref(),
        &config.address_provider,
        vec![MarsAddressType::AstroportIncentives, MarsAddressType::CreditManager],
    )?;
    let astroport_incentives_addr = &addresses[&MarsAddressType::AstroportIncentives];
    let credit_manager_addr = &addresses[&MarsAddressType::CreditManager];

    // To prevent configuration errors, we fetch address from current contract instead of address_provider
    let mars_incentives_addr = env.contract.address.to_string();
    ensure_eq!(info.sender, credit_manager_addr, ContractError::Mars(MarsError::Unauthorized {}));

    let staked_lp_amount = ASTRO_USER_LP_DEPOSITS
        .may_load(deps.storage, (account_id, lp_denom))?
        .ok_or(NoStakedLp {
        account_id: account_id.to_string(),
        denom: lp_denom.to_string(),
    })?;

    claim_rewards_for_staked_lp_position(
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
fn claim_rewards_for_staked_lp_position(
    deps: &mut DepsMut,
    astroport_incentives_addr: &str,
    mars_incentives_addr: &str,
    credit_manager_addr: &str,
    account_id: &str,
    lp_denom: &str,
    staked_lp_amount: Uint128,
) -> Result<Response, ContractError> {
    let mut res = claim_global_staked_lp_rewards(
        deps,
        astroport_incentives_addr,
        mars_incentives_addr,
        lp_denom,
    )?;

    let mut event = Event::new("mars/incentives/claimed_lp_rewards")
        .add_attribute("account_id", account_id.to_string());

    let user_claimable_rewards = calculate_claimable_rewards(
        &mut deps.branch().storage.into(),
        account_id,
        lp_denom,
        staked_lp_amount,
    )?;
    let total_claimed_amount =
        user_claimable_rewards.iter().fold(Uint128::zero(), |acc, coin| acc + coin.amount);

    for coin in &user_claimable_rewards {
        event = event
            .add_attribute("denom", coin.denom.to_string())
            .add_attribute("amount", coin.amount.to_string());
    }

    // Check if rewards not already claimed in the same block
    res = if !user_claimable_rewards.is_empty() && !total_claimed_amount.is_zero() {
        // Send the claimed rewards to the credit manager
        let send_rewards_to_cm_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: credit_manager_addr.to_string(),
            amount: user_claimable_rewards,
        });

        res.add_message(send_rewards_to_cm_msg)
    } else {
        res
    };

    Ok(res.add_event(event))
}

fn calculate_claimable_rewards(
    storage: &mut MaybeMutStorage,
    account_id: &str,
    lp_denom: &str,
    staked_lp_amount: Uint128,
) -> Result<Vec<Coin>, ContractError> {
    let lp_coin = Coin {
        amount: staked_lp_amount,
        denom: lp_denom.to_string(),
    };

    let lp_incentive_states: HashMap<String, Decimal> = ASTRO_INCENTIVE_STATES
        .prefix(lp_denom)
        .range(storage.to_storage(), None, None, Ascending)
        .collect::<StdResult<HashMap<String, Decimal>>>()?;

    calculate_rewards_for_staked_astro_lp_position(
        storage,
        account_id,
        &lp_coin,
        lp_incentive_states,
    )
}
