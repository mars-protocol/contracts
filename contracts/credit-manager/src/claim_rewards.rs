use cosmwasm_std::{DepsMut, Response};
use mars_types::traits::Stringify;

use crate::{
    error::{ContractError, ContractResult},
    state::INCENTIVES,
    utils::increment_coin_balance,
};

pub fn claim_rewards(deps: DepsMut, account_id: &str) -> ContractResult<Response> {
    let incentives = INCENTIVES.load(deps.storage)?;

    let unclaimed_rewards = incentives.query_unclaimed_rewards(&deps.querier, account_id)?;
    if unclaimed_rewards.is_empty() {
        return Err(ContractError::NoAmount);
    }

    for reward in unclaimed_rewards.iter() {
        increment_coin_balance(deps.storage, account_id, reward)?;
    }

    Ok(Response::new()
        .add_message(incentives.claim_rewards_msg(account_id)?)
        .add_attribute("action", "claim_rewards")
        .add_attribute("account_id", account_id)
        .add_attribute("rewards", unclaimed_rewards.as_slice().to_string()))
}
