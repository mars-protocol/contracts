use cosmwasm_std::{to_binary, Binary, ContractResult, QuerierResult};
use mars_outpost::red_bank::{Market, QueryMsg, UserPositionResponse};
use std::collections::HashMap;

#[derive(Default)]
pub struct RedBankQuerier {
    pub markets: HashMap<String, Market>,
    pub users_positions: HashMap<String, UserPositionResponse>,
}

impl RedBankQuerier {
    pub fn handle_query(&self, query: QueryMsg) -> QuerierResult {
        let ret: ContractResult<Binary> = match query {
            QueryMsg::Market {
                denom,
            } => match self.markets.get(&denom) {
                Some(market) => to_binary(&market).into(),
                None => Err(format!("[mock]: could not find the market for {}", denom)).into(),
            },
            QueryMsg::UserPosition {
                user,
            } => match self.users_positions.get(&user) {
                Some(market) => to_binary(&market).into(),
                None => Err(format!("[mock]: could not find the position for {}", user)).into(),
            },
            _ => Err("[mock]: Unsupported red_bank query".to_string()).into(),
        };
        Ok(ret).into()
    }
}
