use cosmwasm_std::{Coin, CosmosMsg, DepsMut, Env, Response, Storage};
use mars_rover::{
    error::{ContractError, ContractResult},
    msg::execute::CallbackMsg,
};

use crate::{
    liquidate::calculate_liquidation,
    state::{COIN_BALANCES, REWARDS_COLLECTOR},
    utils::{decrement_coin_balance, increment_coin_balance},
};

pub fn liquidate_deposit(
    deps: DepsMut,
    env: Env,
    liquidator_account_id: &str,
    liquidatee_account_id: &str,
    debt_coin: Coin,
    request_coin_denom: &str,
) -> ContractResult<Response> {
    let request_coin_balance = COIN_BALANCES
        .load(deps.storage, (liquidatee_account_id, request_coin_denom))
        .map_err(|_| ContractError::CoinNotAvailable(request_coin_denom.to_string()))?;

    let (debt, liquidator_request, liquidatee_request) = calculate_liquidation(
        &deps,
        liquidatee_account_id,
        &debt_coin,
        request_coin_denom,
        request_coin_balance,
    )?;

    let repay_msg =
        repay_debt(deps.storage, &env, liquidator_account_id, liquidatee_account_id, &debt)?;

    // Transfer requested coin from liquidatee to liquidator
    decrement_coin_balance(deps.storage, liquidatee_account_id, &liquidatee_request)?;
    increment_coin_balance(deps.storage, liquidator_account_id, &liquidator_request)?;

    // Transfer protocol fee to rewards-collector account
    let rewards_collector_account = REWARDS_COLLECTOR.load(deps.storage)?.account_id;
    let protocol_fee_amount = liquidatee_request.amount.checked_sub(liquidator_request.amount)?;
    increment_coin_balance(
        deps.storage,
        &rewards_collector_account,
        &Coin::new(protocol_fee_amount.u128(), liquidatee_request.denom.clone()),
    )?;

    Ok(Response::new()
        .add_message(repay_msg)
        .add_attribute("action", "liquidate_deposit")
        .add_attribute("account_id", liquidator_account_id)
        .add_attribute("liquidatee_account_id", liquidatee_account_id)
        .add_attribute("coin_debt_repaid", debt.to_string())
        .add_attribute("coin_liquidated", liquidatee_request.to_string())
        .add_attribute(
            "protocol_fee_coin",
            Coin::new(protocol_fee_amount.u128(), request_coin_denom).to_string(),
        ))
}

pub fn repay_debt(
    storage: &mut dyn Storage,
    env: &Env,
    liquidator_account_id: &str,
    liquidatee_account_id: &str,
    debt: &Coin,
) -> ContractResult<CosmosMsg> {
    // Transfer debt coin from liquidator's coin balance to liquidatee
    // Will be used to pay off the debt via CallbackMsg::Repay {}
    decrement_coin_balance(storage, liquidator_account_id, debt)?;
    increment_coin_balance(storage, liquidatee_account_id, debt)?;
    let msg = (CallbackMsg::Repay {
        account_id: liquidatee_account_id.to_string(),
        coin: debt.into(),
    })
    .into_cosmos_msg(&env.contract.address)?;
    Ok(msg)
}
