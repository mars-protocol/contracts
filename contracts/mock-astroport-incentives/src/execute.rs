use astroport_v5::{
    asset::AssetInfo,
    incentives::{IncentivesSchedule, InputSchedule},
};
use cosmwasm_std::{Coin, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128};

use crate::{
    query::query_rewards,
    state::{ASTRO_LP_INCENTIVE_DEPOSITS, INCENTIVE_SCHEDULES},
};

pub fn incentivise(
    deps: DepsMut,
    env: Env,
    lp_token: String,
    schedule: InputSchedule,
) -> StdResult<Response> {
    let incentives_schedule = IncentivesSchedule::from_input(&env, &schedule)?;

    let reward_denom = match &incentives_schedule.reward_info {
        AssetInfo::NativeToken {
            denom,
        } => denom.to_string(),
        _ => unimplemented!("mock does not support cw20 assets!"),
    };

    // Store the incentive schedule in the state
    INCENTIVE_SCHEDULES.save(deps.storage, (&lp_token, &reward_denom), &incentives_schedule)?;

    Ok(Response::new())
}

// We just use this as a blank to consume the deposit message. This mock assumes only the incentives contract
// will ever deposit / withdraw / interact
pub fn deposit(deps: DepsMut, _: Env, info: MessageInfo) -> StdResult<Response> {
    let sender = info.sender.to_string();
    let coins = info.funds;

    for coin in coins {
        let lp_token = coin.denom.clone();
        ASTRO_LP_INCENTIVE_DEPOSITS.update(deps.storage, (&sender, &lp_token), |value_opt| {
            value_opt
                .unwrap_or_else(Uint128::zero)
                .checked_add(coin.amount)
                .map_err(|_| StdError::generic_err("overflow"))
        })?;
    }

    Ok(Response::new())
}

pub fn withdraw(
    deps: DepsMut,
    _: Env,
    info: MessageInfo,
    lp_token: String,
    amount: Uint128,
) -> StdResult<Response> {
    let sender = info.sender.to_string();

    // Send the rewards to the user
    let withdraw_lp_msg = cosmwasm_std::CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![Coin {
            amount,
            denom: lp_token.clone(),
        }],
    });

    ASTRO_LP_INCENTIVE_DEPOSITS.update(deps.storage, (&sender, &lp_token), |value_opt| {
        value_opt
            .unwrap_or_else(Uint128::zero)
            .checked_sub(amount)
            .map_err(|_| StdError::generic_err("overflow"))
    })?;

    Ok(Response::new().add_message(withdraw_lp_msg))
}

pub fn claim_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    lp_tokens: Vec<String>,
) -> StdResult<Response> {
    let rewards = lp_tokens
        .iter()
        .map(|lp_token: &String| {
            query_rewards(deps.as_ref(), env.clone(), info.sender.to_string(), lp_token.to_string())
                .unwrap()
        })
        .fold(vec![], |mut acc, item| {
            acc.extend(item);
            acc
        });

    let response = Response::new();

    if rewards.is_empty() {
        return Ok(response);
    }

    // Send the rewards to the user
    let reward_msg = cosmwasm_std::CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: rewards.into_iter().map(|asset| asset.as_coin().unwrap()).collect(),
    });

    Ok(response.add_message(reward_msg))
}
