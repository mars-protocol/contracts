use std::collections::HashMap;

use cosmwasm_std::{to_binary, Binary, ContractResult, QuerierResult};
use mars_red_bank_types::red_bank::{
    Market, QueryMsg, UserCollateralResponse, UserPositionResponse,
};

#[derive(Default)]
pub struct RedBankQuerier {
    pub markets: HashMap<String, Market>,
    pub users_denoms_collaterals: HashMap<(String, String), UserCollateralResponse>,
    pub users_positions: HashMap<String, UserPositionResponse>,
}

impl RedBankQuerier {
    pub fn handle_query(&self, query: QueryMsg) -> QuerierResult {
        let ret: ContractResult<Binary> = match query {
            QueryMsg::Market {
                denom,
            } => match self.markets.get(&denom) {
                Some(market) => to_binary(&market).into(),
                None => Err(format!("[mock]: could not find the market for {denom}")).into(),
            },
            QueryMsg::UserCollateral {
                user,
                denom,
            } => match self.users_denoms_collaterals.get(&(user.clone(), denom)) {
                Some(collateral) => to_binary(&collateral).into(),
                None => Err(format!("[mock]: could not find the collateral for {user}")).into(),
            },
            QueryMsg::UserPosition {
                user,
            } => match self.users_positions.get(&user) {
                Some(market) => to_binary(&market).into(),
                None => Err(format!("[mock]: could not find the position for {user}")).into(),
            },
            _ => Err("[mock]: Unsupported red_bank query".to_string()).into(),
        };
        Ok(ret).into()
    }
}
