use std::collections::HashMap;

use cosmwasm_std::{Addr, Coin, Deps, QuerierWrapper, StdError, StdResult, Uint128};

use cw20::TokenInfoResponse;
use mars_outpost::ma_token::msg::QueryMsg;
use mars_outpost::red_bank::{Market, Position};

use crate::error::ContractError;
use crate::state::{MARKETS, MARKET_DENOMS_BY_INDEX};

// native coins
pub fn get_denom_amount_from_coins(coins: &[Coin], denom: &str) -> Result<Uint128, ContractError> {
    if coins.len() == 1 && coins[0].denom == denom {
        Ok(coins[0].amount)
    } else {
        Err(ContractError::InvalidCoinsSent {
            denom: denom.to_string(),
        })
    }
}

pub fn get_market_from_index(deps: &Deps, index: u32) -> StdResult<(String, Market)> {
    let denom = MARKET_DENOMS_BY_INDEX
        .load(deps.storage, index)
        .map_err(|_| StdError::generic_err(format!("no denom exists with index: {}", index)))?;

    let market = MARKETS
        .load(deps.storage, &denom)
        .map_err(|_| StdError::generic_err(format!("no market exists with denom: {}", denom)))?;

    Ok((denom, market))
}

// bitwise operations
/// Gets bit: true: 1, false: 0
pub fn get_bit(bitmap: Uint128, index: u32) -> StdResult<bool> {
    if index >= 128 {
        return Err(StdError::generic_err("index out of range"));
    }
    Ok(((bitmap.u128() >> index) & 1) == 1)
}

/// Sets bit to 1
pub fn set_bit(bitmap: &mut Uint128, index: u32) -> StdResult<()> {
    if index >= 128 {
        return Err(StdError::generic_err("index out of range"));
    }
    *bitmap = Uint128::from(bitmap.u128() | (1 << index));
    Ok(())
}

/// Sets bit to 0
pub fn unset_bit(bitmap: &mut Uint128, index: u32) -> StdResult<()> {
    if index >= 128 {
        return Err(StdError::generic_err("index out of range"));
    }
    *bitmap = Uint128::from(bitmap.u128() & !(1 << index));
    Ok(())
}

pub fn get_uncollaterized_debt(positions: &HashMap<String, Position>) -> StdResult<Uint128> {
    positions.values().try_fold(Uint128::zero(), |total, p| {
        if p.uncollateralized_debt {
            total.checked_add(p.debt_amount)?;
        }
        Ok(total)
    })
}

pub fn query_total_deposits(querier: &QuerierWrapper, ma_token_addr: &Addr) -> StdResult<Uint128> {
    Ok(querier
        .query_wasm_smart::<TokenInfoResponse>(ma_token_addr.clone(), &QueryMsg::TokenInfo {})?
        .total_supply)
}
