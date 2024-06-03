use cosmwasm_std::{
    Addr, BankMsg, Coin, CosmosMsg, DepsMut, MessageInfo, Response, StdResult, Uint128,
};

use crate::{
    query::{query_pending_astroport_rewards, query_unclaimed_rewards},
    state::{PENDING_ASTROPORT_REWARDS, UNCLAIMED_REWARDS},
};

pub fn claim_astro_lp_rewards(
    deps: DepsMut,
    info: MessageInfo,
    account_id: String,
    lp_denom: String,
) -> StdResult<Response> {
    let pending_astro_rewards: Vec<Coin> =
        query_pending_astroport_rewards(deps.as_ref(), account_id.clone(), lp_denom)?;

    let transfer_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: pending_astro_rewards,
    });

    Ok(Response::new().add_message(transfer_msg))
}

pub fn claim_rewards(
    deps: DepsMut,
    info: MessageInfo,
    account_id: Option<String>,
) -> StdResult<Response> {
    let unclaimed = query_unclaimed_rewards(deps.as_ref(), info.sender.as_str(), &account_id)?;

    UNCLAIMED_REWARDS.remove(deps.storage, (info.sender.clone(), account_id.unwrap_or_default()));

    // Mock env responsible for seeding contract with coins
    let transfer_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: unclaimed,
    });

    Ok(Response::new().add_message(transfer_msg))
}

/// Privileged sudo message for adding unclaimed rewards for user
pub fn balance_change(
    deps: DepsMut,
    info: MessageInfo,
    user_addr: Addr,
    account_id: Option<String>,
    denom: String,
    user_amount_scaled_before: Uint128,
) -> StdResult<Response> {
    let mut unclaimed = query_unclaimed_rewards(deps.as_ref(), user_addr.as_str(), &account_id)?;

    unclaimed.push(Coin {
        denom,
        amount: user_amount_scaled_before,
    });

    UNCLAIMED_REWARDS.save(
        deps.storage,
        (info.sender, account_id.unwrap_or_default()),
        &unclaimed,
    )?;

    Ok(Response::new())
}

/// Privileged sudo message for setting incentive rewards for astroport LP's
pub fn set_incentive_rewards(
    deps: DepsMut,
    _: MessageInfo,
    collateral_denom: String,
    incentive_denom: String,
    emission_per_second: Uint128,
    start_time: u64,
) -> StdResult<Response> {
    // Rename variables to match the desired usage
    let account_id = start_time.to_string();
    let lp_denom = collateral_denom;
    let incentive_amount = emission_per_second;

    let mut pending_astro_rewards: Vec<Coin> =
        query_pending_astroport_rewards(deps.as_ref(), account_id.clone(), lp_denom.clone())?;

    pending_astro_rewards.push(Coin {
        denom: incentive_denom,
        amount: incentive_amount,
    });

    PENDING_ASTROPORT_REWARDS.save(deps.storage, (account_id, lp_denom), &pending_astro_rewards)?;

    Ok(Response::new())
}
