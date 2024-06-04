use cosmwasm_std::{DepsMut, Event, Response};

use crate::{
    error::{ContractError, ContractResult},
    state::INCENTIVES,
    utils::increment_coin_balance,
};

pub fn claim_lp_rewards(
    deps: DepsMut,
    account_id: &str,
    lp_denom: &str,
) -> ContractResult<Response> {
    let incentives = INCENTIVES.load(deps.storage)?;

    // Query rewards user is receiving, update balance
    let rewards =
        incentives.query_astroport_staked_lp_rewards(&deps.querier, account_id, lp_denom)?;
    if rewards.is_empty() {
        return Err(ContractError::NoAmount);
    }

    for reward in rewards.iter() {
        increment_coin_balance(deps.storage, account_id, reward)?;
    }

    let claim_rewards_msg = incentives.claim_lp_rewards_msg(account_id, lp_denom)?;

    let event = Event::new("mars/incentives/claim_lp_rewards")
        .add_attribute("rewards", format!("{:?}", rewards));

    Ok(Response::new()
        .add_message(claim_rewards_msg)
        .add_event(event)
        .add_attribute("action", "claim_lp_rewards")
        .add_attribute("account_id", account_id))
}
