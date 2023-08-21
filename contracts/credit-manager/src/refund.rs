use cosmwasm_std::{to_binary, Addr, CosmosMsg, DepsMut, Env, Response, WasmMsg};
use mars_rover::{
    error::ContractResult,
    msg::{
        execute::{ActionAmount, ActionCoin, CallbackMsg},
        ExecuteMsg,
    },
};

use crate::{query::query_coin_balances, utils::query_nft_token_owner};

pub fn refund_coin_balances(deps: DepsMut, env: Env, account_id: &str) -> ContractResult<Response> {
    let coins = query_coin_balances(deps.as_ref(), account_id)?;
    let account_nft_owner = query_nft_token_owner(deps.as_ref(), account_id)?;
    let withdraw_msgs = coins
        .into_iter()
        .map(|coin| {
            let action_amount = ActionAmount::Exact(coin.amount);
            let action_coin = ActionCoin {
                denom: coin.denom,
                amount: action_amount,
            };
            Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::Withdraw {
                    account_id: account_id.to_string(),
                    coin: action_coin,
                    recipient: Addr::unchecked(account_nft_owner.clone()),
                }))?,
            }))
        })
        .collect::<ContractResult<Vec<_>>>()?;
    Ok(Response::new()
        .add_messages(withdraw_msgs)
        .add_attribute("action", "callback/refund_coin_balances")
        .add_attribute("account_id", account_id.to_string()))
}
