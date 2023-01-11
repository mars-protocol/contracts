use cosmwasm_std::{Addr, BankMsg, Coin, CosmosMsg, DepsMut, Response};
use mars_rover::error::{ContractError, ContractResult};

use crate::utils::decrement_coin_balance;

pub fn withdraw(
    deps: DepsMut,
    account_id: &str,
    coin: Coin,
    recipient: Addr,
) -> ContractResult<Response> {
    if coin.amount.is_zero() {
        return Err(ContractError::NoAmount);
    }

    decrement_coin_balance(deps.storage, account_id, &coin)?;

    // send coin to recipient
    let transfer_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: recipient.to_string(),
        amount: vec![coin.clone()],
    });

    Ok(Response::new()
        .add_message(transfer_msg)
        .add_attribute("action", "rover/credit-manager/callback/withdraw")
        .add_attribute("account_id", account_id)
        .add_attribute("coin_withdrawn", coin.to_string()))
}
