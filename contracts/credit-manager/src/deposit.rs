use std::collections::BTreeSet;

use cosmwasm_std::{Coin, Coins, Deps, DepsMut, Response};
use mars_types::params::TotalDepositResponse;

use crate::{
    error::{ContractError, ContractResult},
    state::PARAMS,
    utils::increment_coin_balance,
};

pub fn deposit(
    deps: &mut DepsMut,
    response: Response,
    account_id: &str,
    coin: &Coin,
    received_coins: &mut Coins,
) -> ContractResult<Response> {
    if coin.amount.is_zero() {
        return Ok(response);
    }

    assert_sent_fund(coin, received_coins)?;

    received_coins.sub(coin.clone())?;

    increment_coin_balance(deps.storage, account_id, coin)?;

    Ok(response
        .add_attribute("action", "callback/deposit")
        .add_attribute("coin_deposited", coin.to_string()))
}

/// Assert that fund of exactly the same type and amount was sent along with a message
fn assert_sent_fund(expected: &Coin, received_coins: &Coins) -> ContractResult<()> {
    let received = received_coins.amount_of(&expected.denom);

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
        // Asset is not found (not whitelisted) and it doesn't count towards the cap and Health Factor
        let params_opt = params.query_asset_params(&deps.querier, &denom)?;
        if params_opt.is_none() {
            continue;
        }

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
