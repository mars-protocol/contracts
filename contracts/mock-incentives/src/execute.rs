use cosmwasm_std::{
    Addr, BankMsg, Coin, CosmosMsg, DepsMut, MessageInfo, Response, StdResult, Uint128,
};

use crate::{query::query_unclaimed_rewards, state::UNCLAIMED_REWARDS};

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
        (info.sender, account_id.clone().unwrap_or_default()),
        &unclaimed,
    )?;

    Ok(Response::new())
}
