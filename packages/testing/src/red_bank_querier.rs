use std::collections::HashMap;

use cosmwasm_std::{to_json_binary, Binary, ContractResult, QuerierResult};
use mars_types::red_bank::{
    Market, QueryMsg, UserCollateralResponse, UserDebtResponse, UserPositionResponse,
};

#[derive(Default)]
pub struct RedBankQuerier {
    pub markets: HashMap<String, Market>,
    pub users_denoms_collaterals: HashMap<(String, String), UserCollateralResponse>,
    pub users_denoms_debts: HashMap<(String, String), UserDebtResponse>,
    pub users_positions: HashMap<String, UserPositionResponse>,
}

impl RedBankQuerier {
    pub fn handle_query(&self, query: QueryMsg) -> QuerierResult {
        let ret: ContractResult<Binary> = match query {
            QueryMsg::Market {
                denom,
            } => {
                let maybe_market = self.markets.get(&denom);
                to_json_binary(&maybe_market).into()
            }
            QueryMsg::UserCollateral {
                user,
                account_id: _,
                denom,
            } => match self.users_denoms_collaterals.get(&(user.clone(), denom)) {
                Some(collateral) => to_json_binary(&collateral).into(),
                None => Err(format!("[mock]: could not find the collateral for {user}")).into(),
            },
            QueryMsg::UserDebt {
                user,
                denom,
            } => match self.users_denoms_debts.get(&(user.clone(), denom)) {
                Some(debt) => to_json_binary(&debt).into(),
                None => Err(format!("[mock]:  could not find the debt for {user}")).into(),
            },
            QueryMsg::UserPosition {
                user,
                account_id: _,
            } => match self.users_positions.get(&user) {
                Some(market) => to_json_binary(&market).into(),
                None => Err(format!("[mock]: could not find the position for {user}")).into(),
            },
            _ => Err("[mock]: Unsupported red_bank query".to_string()).into(),
        };
        Ok(ret).into()
    }
}
