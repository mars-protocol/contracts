use std::cmp::min;

use cosmwasm_std::{
    to_json_binary, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, Uint128, WasmMsg,
};
use cw_utils::one_coin;
use mars_types::credit_manager::{ActionCoin, CallbackMsg::Repay, ExecuteMsg};

use crate::{
    error::ContractResult,
    state::{COIN_BALANCES, RED_BANK},
    utils::{decrement_coin_balance, increment_coin_balance},
};

pub fn repay(deps: DepsMut, account_id: &str, coin: &ActionCoin) -> ContractResult<Response> {
    let red_bank = RED_BANK.load(deps.storage)?;
    let debt_amount = red_bank.query_debt(&deps.querier, &coin.denom, account_id)?;

    let coin_balance =
        COIN_BALANCES.may_load(deps.storage, (account_id, &coin.denom))?.unwrap_or_default();

    let amount_to_repay = min(debt_amount, coin.amount.value().unwrap_or(coin_balance));
    let coin_to_repay = Coin {
        denom: coin.denom.to_string(),
        amount: amount_to_repay,
    };

    decrement_coin_balance(deps.storage, account_id, &coin_to_repay)?;

    let red_bank_repay_msg = red_bank.repay_msg(&coin_to_repay, account_id)?;

    Ok(Response::new()
        .add_message(red_bank_repay_msg)
        .add_attribute("action", "repay")
        .add_attribute("account_id", account_id)
        .add_attribute("coin_repaid", coin_to_repay.to_string()))
}

pub fn repay_for_recipient(
    deps: DepsMut,
    env: Env,
    benefactor_account_id: &str,
    recipient_account_id: &str,
    coin: ActionCoin,
) -> ContractResult<Response> {
    let red_bank = RED_BANK.load(deps.storage)?;
    let debt_amount = red_bank.query_debt(&deps.querier, &coin.denom, recipient_account_id)?;
    let coin_balance = COIN_BALANCES
        .may_load(deps.storage, (benefactor_account_id, &coin.denom))?
        .unwrap_or_default();
    let amount_to_repay = min(debt_amount, coin.amount.value().unwrap_or(coin_balance));
    let coin_to_repay = &Coin {
        denom: coin.denom,
        amount: amount_to_repay,
    };

    decrement_coin_balance(deps.storage, benefactor_account_id, coin_to_repay)?;
    increment_coin_balance(deps.storage, recipient_account_id, coin_to_repay)?;

    let repay_callback_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        funds: vec![],
        msg: to_json_binary(&ExecuteMsg::Callback(Repay {
            account_id: recipient_account_id.to_string(),
            coin: ActionCoin::from(coin_to_repay),
        }))?,
    });

    Ok(Response::new()
        .add_message(repay_callback_msg)
        .add_attribute("action", "repay_for_recipient")
        .add_attribute("benefactor_account_id", benefactor_account_id)
        .add_attribute("recipient_account_id", recipient_account_id)
        .add_attribute("coin_repaid", coin_to_repay.to_string()))
}

pub fn repay_from_wallet(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account_id: String,
) -> ContractResult<Response> {
    let coin_sent = one_coin(&info)?;

    let red_bank = RED_BANK.load(deps.storage)?;
    let debt_amount = red_bank.query_debt(&deps.querier, &coin_sent.denom, &account_id)?;

    let amount_to_repay = min(debt_amount, coin_sent.amount);
    let coin_to_repay = Coin {
        denom: coin_sent.denom.clone(),
        amount: amount_to_repay,
    };

    increment_coin_balance(deps.storage, &account_id, &coin_to_repay)?;

    let repay_callback_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        funds: vec![],
        msg: to_json_binary(&ExecuteMsg::Callback(Repay {
            account_id: account_id.to_string(),
            coin: ActionCoin::from(&coin_to_repay),
        }))?,
    });

    // if attempting to repay too much, refund back the extra
    let refund_amount = if coin_sent.amount > coin_to_repay.amount {
        coin_sent.amount.checked_sub(coin_to_repay.amount)?
    } else {
        Uint128::zero()
    };

    let mut response = Response::new()
        .add_message(repay_callback_msg)
        .add_attribute("action", "repay_from_wallet")
        .add_attribute("from_address", info.sender.to_string())
        .add_attribute("account_id", account_id)
        .add_attribute("coin_repaid", coin_to_repay.to_string())
        .add_attribute("refunded", refund_amount.to_string());

    if !refund_amount.is_zero() {
        let transfer_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![Coin {
                denom: coin_sent.denom,
                amount: refund_amount,
            }],
        });
        response = response.add_message(transfer_msg);
    }

    Ok(response)
}
