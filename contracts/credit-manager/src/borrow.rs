use cosmwasm_std::{Coin, DepsMut, Env, Response, StdError, StdResult, Uint128};

use rover::error::{ContractError, ContractResult};

use crate::deposit::assert_coin_is_whitelisted;
use crate::state::{COIN_BALANCES, DEBT_SHARES, RED_BANK, TOTAL_DEBT_SHARES};

pub static DEFAULT_DEBT_SHARES_PER_COIN_BORROWED: Uint128 = Uint128::new(1_000_000);

/// calculate by how many the user's debt units should be increased
/// if total debt is zero, then we define 1 unit of coin borrowed = 1,000,000 debt unit
/// else, get debt ownership % and multiply by total existing shares
///
/// increment total debt shares, token debt shares, and asset amount
pub fn borrow(deps: DepsMut, env: Env, token_id: &str, coin: Coin) -> ContractResult<Response> {
    if coin.amount.is_zero() {
        return Err(ContractError::NoAmount);
    }

    assert_coin_is_whitelisted(deps.storage, &coin.denom)?;

    let red_bank = RED_BANK.load(deps.storage)?;
    let total_debt_amount =
        red_bank.query_debt(&deps.querier, &env.contract.address, &coin.denom)?;

    let debt_shares_to_add = if total_debt_amount.is_zero() {
        coin.amount
            .checked_mul(DEFAULT_DEBT_SHARES_PER_COIN_BORROWED)
            .map_err(StdError::overflow)?
    } else {
        TOTAL_DEBT_SHARES
            .load(deps.storage, &coin.denom)?
            .checked_multiply_ratio(coin.amount, total_debt_amount)?
    };

    TOTAL_DEBT_SHARES.update(deps.storage, &coin.denom, |shares| -> StdResult<_> {
        shares
            .unwrap_or_else(Uint128::zero)
            .checked_add(debt_shares_to_add)
            .map_err(StdError::overflow)
    })?;

    DEBT_SHARES.update(
        deps.storage,
        (token_id, &coin.denom),
        |shares| -> StdResult<_> {
            shares
                .unwrap_or_else(Uint128::zero)
                .checked_add(debt_shares_to_add)
                .map_err(StdError::overflow)
        },
    )?;

    COIN_BALANCES.update(
        deps.storage,
        (token_id, &coin.denom),
        |current_amount| -> StdResult<_> {
            current_amount
                .unwrap_or_else(Uint128::zero)
                .checked_add(coin.amount)
                .map_err(StdError::overflow)
        },
    )?;

    Ok(Response::new()
        .add_message(red_bank.borrow_msg(&coin)?)
        .add_attribute("action", "rover/credit_manager/borrow")
        .add_attribute("debt_shares_added", debt_shares_to_add)
        .add_attribute("coins_borrowed", coin.amount))
}
