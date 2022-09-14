use cosmwasm_std::{Addr, QuerierWrapper, StdResult, Uint128};
use cw20::TokenInfoResponse;

use mars_outpost::ma_token::msg::QueryMsg;

pub fn query_total_deposits(querier: &QuerierWrapper, ma_token_addr: &Addr) -> StdResult<Uint128> {
    Ok(querier
        .query_wasm_smart::<TokenInfoResponse>(ma_token_addr.clone(), &QueryMsg::TokenInfo {})?
        .total_supply)
}
