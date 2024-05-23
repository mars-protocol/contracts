use std::collections::HashMap;

use astroport::incentives::ExecuteMsg;
use cosmwasm_std::{
    to_json_binary, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, Env, Event, MessageInfo,
    Order::Ascending, Response, StdResult, Storage, Uint128, WasmMsg,
};
use mars_types::{
    address_provider::{self, helpers::query_contract_addrs, MarsAddressType}, error::MarsError,
    incentives::LpModification,
};

use crate::{
    helpers::{
        assert_caller_is_credit_manager, calculate_rewards_from_astroport_incentive_state,
        claim_rewards_msg, compute_updated_astroport_incentive_states,
    },
    query::query_unclaimed_astroport_rewards,
    state::{
        ASTRO_INCENTIVE_STATES, CONFIG, ASTRO_USER_LP_DEPOSITS, ASTRO_TOTAL_LP_DEPOSITS,
        USER_ASTRO_INCENTIVE_STATES,
    },
    ContractError,
    ContractError::NoStakedLp,
};

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
    )?;

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
    let addresses = query_contract_addrs(
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

    let addresses = query_contract_addrs(
        deps.as_ref(),
        &config.address_provider,
        vec![MarsAddressType::AstroportIncentives, MarsAddressType::CreditManager],
    )?;

    assert_caller_is_credit_manager(info.sender, &addresses[&MarsAddressType::CreditManager])?;

    update_user_lp_position(
        &mut deps,
        &account_id,
        lp_coin,
        addresses[&MarsAddressType::AstroportIncentives].as_str(),
        env.contract.address.as_str(),
        addresses[&MarsAddressType::CreditManager].as_str(),
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
    let staked_lp_amount = ASTRO_USER_LP_DEPOSITS
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
                msg: to_json_binary(&ExecuteMsg::Deposit {
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
                    msg: to_json_binary(&ExecuteMsg::Withdraw {
                        lp_token: (&lp_coin.denom).to_string(),
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

fn increment_lp_deposit(
    store: &mut dyn Storage,
    account_id: &str,
    lp_coin: &Coin,
) -> Result<(), ContractError> {
    // Update user staked lp state
    ASTRO_USER_LP_DEPOSITS.update(store, (&account_id, &lp_coin.denom), |existing| -> StdResult<_> {
        Ok(existing.unwrap_or_default().checked_add(lp_coin.amount)?)
    })?;

    // Update total staked lp state
    ASTRO_TOTAL_LP_DEPOSITS.update(store, &lp_coin.denom, |existing| -> StdResult<_> {
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
    ASTRO_USER_LP_DEPOSITS.update(store, (&account_id, &lp_coin.denom), |existing| -> StdResult<_> {
        Ok(existing
            .ok_or(ContractError::NoStakedLp { 
                account_id: account_id.to_string(), 
                denom: lp_coin.denom.clone() })?
            .checked_sub(lp_coin.amount)?)
    })?;

    // Update total staked lp state
    ASTRO_TOTAL_LP_DEPOSITS.update(store, &lp_coin.denom, |existing| -> StdResult<_> {
        Ok(existing
            .ok_or(ContractError::NoDeposits { denom: lp_coin.denom.clone() })?
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
        USER_ASTRO_INCENTIVE_STATES.save(
            storage,
            (&account_id, &lp_denom, &incentive_denom),
            &updated_incentive,
        )?;
        // Store latest state
        ASTRO_INCENTIVE_STATES.save(
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

    let addresses = query_contract_addrs(
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
        ASTRO_USER_LP_DEPOSITS.may_load(deps.storage, (&account_id, &lp_denom))?.ok_or(NoStakedLp {
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
        
        for coin in &user_claimable_rewards {
            event = event.add_attribute("denom", coin.denom.to_string()).add_attribute("amount", coin.amount.to_string());
        }
        
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
    storage: &dyn Storage,
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
        .range(storage, None, None, Ascending)
        .collect::<StdResult<HashMap<String, Decimal>>>()?;

    calculate_rewards_from_astroport_incentive_state(
        storage,
        account_id,
        &lp_coin,
        lp_incentive_states,
    )
}
