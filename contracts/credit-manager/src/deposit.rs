use cosmwasm_std::{Coin, Response, Storage, Uint128};

use mars_rover::coins::Coins;
use mars_rover::error::{ContractError, ContractResult};

use crate::utils::{assert_coin_is_whitelisted, increment_coin_balance};

pub fn deposit(
    storage: &mut dyn Storage,
    response: Response,
    account_id: &str,
    coin: &Coin,
    received_coins: &mut Coins,
) -> ContractResult<Response> {
    assert_coin_is_whitelisted(storage, &coin.denom)?;

    if coin.amount.is_zero() {
        return Ok(response);
    }

    assert_sent_fund(coin, received_coins)?;

    received_coins.deduct(coin)?;

    increment_coin_balance(storage, account_id, coin)?;

    Ok(response
        .add_attribute("action", "rover/credit-manager/callback/deposit")
        .add_attribute("deposit_received", coin.to_string()))
}

/// Assert that fund of exactly the same type and amount was sent along with a message
fn assert_sent_fund(expected: &Coin, received_coins: &Coins) -> ContractResult<()> {
    let received = received_coins
        .amount(&expected.denom)
        .unwrap_or_else(Uint128::zero);

    if received != expected.amount {
        return Err(ContractError::FundsMismatch {
            expected: expected.amount,
            received,
        });
    }

    Ok(())
}
