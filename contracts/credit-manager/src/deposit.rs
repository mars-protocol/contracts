use std::collections::BTreeMap;

use cosmwasm_std::{Coin, Coins, Deps, DepsMut, Response, Uint128};
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
pub fn assert_deposit_caps(
    deps: Deps,
    denom_deposits: BTreeMap<String, Option<Uint128>>,
) -> ContractResult<Response> {
    let params = PARAMS.load(deps.storage)?;

    let mut response = Response::new().add_attribute("action", "callback/assert_deposit_caps");

    for (denom, deposited_opt) in denom_deposits {
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

        // - If there is None in the map, it means that the deposit cap should be enforced. It is related to the Deposit action.
        // - If there is Some in the map, it means that the deposit amount should be compared (value before and after the TX).
        // It is related to the SwapExactIn and ProvideLiquidity actions.
        if let Some(deposited) = deposited_opt {
            // amount is lower than or equal to the previous deposit amount so it is fine
            if amount <= deposited {
                continue;
            }
        }

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

/// Update the total deposit amount for the asset in the denom_deposits map
/// The function either resets the deposit amount to None for Deposit actions
/// or updates the deposit amount based on the received coins and existing parameters.
pub fn update_or_reset_denom_deposits(
    deps: Deps,
    denom_deposits: &mut BTreeMap<String, Option<Uint128>>,
    denom: &str,
    received_coins: &Coins,
    deposit_action: bool,
) -> ContractResult<()> {
    // Overwrite the previous amount for Deposit action.
    // It means that we don't compare deposits before and after the TX.
    //
    // Strictly enforce the deposit cap to prevent any increase in total value.
    // Even if we use the funds for operations within the account (such as liquidation),
    // and withdraw them at the end (resulting in zero net inflow), temporary funds
    // could still be used for malicious actions.
    if deposit_action {
        denom_deposits.insert(denom.to_string(), None);

        return Ok(());
    }

    // Check if the denomination is already in the list.
    // This ensures that a Deposit action (which does not have an associated amount)
    // is not overwritten by a subsequent Swap or ProvideLiquidity action.
    // By confirming the existence of the denomination in the list, we maintain
    // the integrity of the original Deposit action.
    if denom_deposits.contains_key(denom) {
        return Ok(());
    }

    // Load the params
    let params = PARAMS.load(deps.storage)?;

    // Asset is not found (not whitelisted) and it doesn't count towards the cap and Health Factor
    let params_opt = params.query_asset_params(&deps.querier, denom)?;
    if params_opt.is_none() {
        return Ok(());
    }

    // Check total deposit amount for the asset
    let total_deposit_amt = params.query_total_deposit(&deps.querier, denom)?.amount;

    // Check if the asset was sent in the TX
    let received_amt = received_coins.amount_of(denom);

    // Total deposit amount is the sum of all deposits for the asset across Red Bank and Rover.
    // If the asset was sent in the TX, the Credit Manager balance already includes the deposited amount
    // so we need to subtract it from the total deposit amount to see previous state.
    let new_total_deposit_amt = total_deposit_amt.checked_sub(received_amt).unwrap_or_default();

    // Update the total deposit amount for the asset
    denom_deposits.insert(denom.to_string(), Some(new_total_deposit_amt));

    Ok(())
}
