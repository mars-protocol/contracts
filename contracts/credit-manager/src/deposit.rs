use std::collections::BTreeSet;

use cosmwasm_std::{Coin, Deps, DepsMut, Response, Uint128};
use mars_params::msg::TotalDepositResponse;
use mars_rover::{
    coins::Coins,
    error::{ContractError, ContractResult},
};

use crate::{
    state::PARAMS,
    utils::{assert_coin_is_whitelisted, increment_coin_balance},
};

pub fn deposit(
    deps: &mut DepsMut,
    response: Response,
    account_id: &str,
    coin: &Coin,
    received_coins: &mut Coins,
) -> ContractResult<Response> {
    assert_coin_is_whitelisted(deps, &coin.denom)?;

    if coin.amount.is_zero() {
        return Ok(response);
    }

    assert_sent_fund(coin, received_coins)?;

    received_coins.deduct(coin)?;

    increment_coin_balance(deps.storage, account_id, coin)?;

    Ok(response
        .add_attribute("action", "callback/deposit")
        .add_attribute("coin_deposited", coin.to_string()))
}

/// Assert that fund of exactly the same type and amount was sent along with a message
fn assert_sent_fund(expected: &Coin, received_coins: &Coins) -> ContractResult<()> {
    let received = received_coins.amount(&expected.denom).unwrap_or_else(Uint128::zero);

    if received != expected.amount {
        return Err(ContractError::FundsMismatch {
            expected: expected.amount,
            received,
        });
    }

    Ok(())
}

/// Given a list of denoms, assert that the total deposited amount of each denom
/// across Red Bank and Rover does not exceed its deposit cap recorded in the
/// params contract.
pub fn assert_deposit_caps(deps: Deps, denoms: BTreeSet<String>) -> ContractResult<Response> {
    let params = PARAMS.load(deps.storage)?;

    let mut response = Response::new().add_attribute("action", "callback/assert_deposit_caps");

    for denom in denoms {
        let TotalDepositResponse {
            denom,
            amount,
            cap,
        } = params.query_total_deposit(&deps.querier, &denom)?;

        if amount > cap {
            return Err(ContractError::AboveAssetDepositCap {
                new_value: Coin {
                    denom,
                    amount,
                },
                maximum: cap,
            });
        }

        response = response
            .add_attribute("denom", denom)
            .add_attribute("amount", amount)
            .add_attribute("cap", cap);
    }

    Ok(response)
}
