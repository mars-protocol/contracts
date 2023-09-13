use cosmwasm_std::{
    to_binary, Addr, BankMsg, Coin, CosmosMsg, DepsMut, Env, QuerierWrapper, Response, StdResult,
    WasmMsg,
};
use mars_rover::{
    error::{ContractError, ContractResult},
    msg::{
        execute::{CallbackMsg, ChangeExpected},
        ExecuteMsg,
    },
    traits::Denoms,
};
use mars_rover_health_types::AccountKind;

use crate::{
    state::INCENTIVES,
    update_coin_balances::query_balance,
    utils::{get_account_kind, update_balances_msgs},
};

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

    // For HLS accounts there are special requirements enforced for this account type.
    // assert_hls_rules only allows assets with HLS params set in the params contract
    // and where the collateral is whitelisted.
    // We withdraw all claimed rewards for HLS accounts to the recipient address.
    let kind = get_account_kind(deps.storage, account_id)?;
    let msgs = match kind {
        AccountKind::Default => update_balances_msgs(
            &deps.querier,
            &env.contract.address,
            account_id,
            unclaimed_rewards.to_denoms(),
            ChangeExpected::Increase,
        )?,
        AccountKind::HighLeveredStrategy => {
            let msg = send_rewards_msg(
                &deps.querier,
                &env.contract.address,
                account_id,
                recipient.clone(),
                unclaimed_rewards.to_denoms(),
            )?;
            vec![msg]
        }
    };

    Ok(Response::new()
        .add_message(incentives.claim_rewards_msg(account_id)?)
        .add_messages(msgs)
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
