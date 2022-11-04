use cosmwasm_std::{Addr, BankMsg, Coin, CosmosMsg, DepsMut, Response};

use rover::error::{ContractError, ContractResult};

use crate::utils::{assert_coin_is_whitelisted, decrement_coin_balance};

pub fn withdraw(
    deps: DepsMut,
    account_id: &str,
    coin: Coin,
    recipient: Addr,
) -> ContractResult<Response> {
    assert_coin_is_whitelisted(deps.storage, &coin.denom)?;

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
        .add_attribute("withdrawn", coin.to_string()))
}
