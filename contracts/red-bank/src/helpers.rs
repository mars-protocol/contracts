use std::collections::HashMap;

use cosmwasm_std::{Addr, QuerierWrapper, StdResult, Uint128};
use cw20::TokenInfoResponse;

use mars_outpost::ma_token::msg::QueryMsg;
use mars_outpost::red_bank::Position;

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
