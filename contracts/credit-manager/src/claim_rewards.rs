use cosmwasm_std::{DepsMut, Env, Response};
use mars_rover::{
    error::{ContractError, ContractResult},
    msg::execute::ChangeExpected,
    traits::Denoms,
};

use crate::{state::INCENTIVES, utils::update_balances_msgs};

pub fn claim_rewards(deps: DepsMut, env: Env, account_id: &str) -> ContractResult<Response> {
    let incentives = INCENTIVES.load(deps.storage)?;

    let unclaimed_rewards = incentives.query_unclaimed_rewards(&deps.querier, account_id)?;
    if unclaimed_rewards.is_empty() {
        return Err(ContractError::NoAmount);
    }

    let update_balances_msgs = update_balances_msgs(
        &deps.querier,
        &env.contract.address,
        account_id,
        unclaimed_rewards.to_denoms(),
        ChangeExpected::Increase,
    )?;

    Ok(Response::new()
        .add_message(incentives.claim_rewards_msg(account_id)?)
        .add_messages(update_balances_msgs)
        .add_attribute("action", "claim_rewards")
        .add_attribute("account_id", account_id))
}
