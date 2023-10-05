use cosmwasm_std::{
    to_binary, Addr, BankMsg, Coin, CosmosMsg, DepsMut, Env, QuerierWrapper, Response, StdResult,
    WasmMsg,
};
use mars_rover::{
    error::{ContractError, ContractResult},
    msg::{execute::CallbackMsg, ExecuteMsg},
    traits::Denoms,
};

use crate::{state::INCENTIVES, update_coin_balances::query_balance};

pub fn claim_rewards(
    deps: DepsMut,
    env: Env,
    account_id: &str,
    recipient: Addr,
) -> ContractResult<Response> {
    let incentives = INCENTIVES.load(deps.storage)?;

    let unclaimed_rewards = incentives.query_unclaimed_rewards(&deps.querier, account_id)?;
    if unclaimed_rewards.is_empty() {
        return Err(ContractError::NoAmount);
    }

    // Incentive denom may not be listed in params contract.
    // When rewards are claimed to the account, they are considered deposits (collateral).
    // If the account requires HF check, health contract won't be able to find
    // incentive denom params (such as MaxLTV) for HF calculation and the TX will be rejected.
    // To address this issue we withdraw all claimed rewards to the recipient address.
    let msg = send_rewards_msg(
        &deps.querier,
        &env.contract.address,
        account_id,
        recipient.clone(),
        unclaimed_rewards.to_denoms(),
    )?;

    Ok(Response::new()
        .add_message(incentives.claim_rewards_msg(account_id)?)
        .add_message(msg)
        .add_attribute("action", "claim_rewards")
        .add_attribute("account_id", account_id)
        .add_attribute("recipient", recipient.to_string()))
}

fn send_rewards_msg(
    querier: &QuerierWrapper,
    credit_manager_addr: &Addr,
    account_id: &str,
    recipient: Addr,
    denoms: Vec<&str>,
) -> StdResult<CosmosMsg> {
    let coins = denoms
        .iter()
        .map(|denom| query_balance(querier, credit_manager_addr, denom))
        .collect::<StdResult<Vec<_>>>()?;
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: credit_manager_addr.to_string(),
        funds: vec![],
        msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::SendRewardsToAddr {
            account_id: account_id.to_string(),
            previous_balances: coins,
            recipient,
        }))?,
    }))
}

pub fn send_rewards(
    deps: DepsMut,
    credit_manager_addr: &Addr,
    account_id: &str,
    recipient: Addr,
    previous_balances: Vec<Coin>,
) -> ContractResult<Response> {
    let coins = previous_balances
        .into_iter()
        .map(|coin| {
            let current_balance = query_balance(&deps.querier, credit_manager_addr, &coin.denom)?;
            let amount_to_withdraw = current_balance.amount.checked_sub(coin.amount)?;
            Ok(Coin {
                denom: coin.denom,
                amount: amount_to_withdraw,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    // send coin to recipient
    let transfer_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: recipient.to_string(),
        amount: coins,
    });

    Ok(Response::new()
        .add_message(transfer_msg)
        .add_attribute("action", "callback/send_rewards")
        .add_attribute("account_id", account_id)
        .add_attribute("recipient", recipient.to_string()))
}
