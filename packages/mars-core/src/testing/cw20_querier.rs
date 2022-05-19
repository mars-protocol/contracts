use std::collections::HashMap;

use cosmwasm_std::{to_binary, Addr, QuerierResult, SystemError, Uint128};
use cw20::{AllAccountsResponse, BalanceResponse, Cw20QueryMsg, TokenInfoResponse};

use crate::ma_token;

#[derive(Clone, Debug, Default)]
pub struct Cw20Querier {
    /// maps cw20 contract address to user balances
    pub balances: HashMap<Addr, HashMap<Addr, Uint128>>,
    /// maps cw20 contract address to token info response
    pub token_info_responses: HashMap<Addr, TokenInfoResponse>,
}

impl Cw20Querier {
    pub fn handle_cw20_query(&self, contract_addr: &Addr, query: Cw20QueryMsg) -> QuerierResult {
        match query {
            Cw20QueryMsg::AllAccounts { start_after, limit } => {
                if start_after.is_some() {
                    return Err(SystemError::InvalidRequest {
                        error: "mock cw20 only supports `start_after` to be `None`".to_string(),
                        request: Default::default(),
                    })
                    .into();
                }

                let contract_balances = match self.balances.get(contract_addr) {
                    Some(balances) => balances,
                    None => {
                        return Err(SystemError::InvalidRequest {
                            error: format!(
                                "no balance available for account address {}",
                                contract_addr
                            ),
                            request: Default::default(),
                        })
                        .into()
                    }
                };

                let mut accounts = contract_balances
                    .keys()
                    .take(limit.unwrap_or(10) as usize)
                    .map(|addr| addr.to_string())
                    .collect::<Vec<_>>();

                // sort accounts alphabetically
                accounts.sort();

                Ok(to_binary(&AllAccountsResponse { accounts }).into()).into()
            }

            Cw20QueryMsg::Balance { address } => {
                let contract_balances = match self.balances.get(contract_addr) {
                    Some(balances) => balances,
                    None => {
                        return Err(SystemError::InvalidRequest {
                            error: format!(
                                "no balance available for account address {}",
                                contract_addr
                            ),
                            request: Default::default(),
                        })
                        .into()
                    }
                };

                let user_balance = match contract_balances.get(&Addr::unchecked(address)) {
                    Some(balance) => balance,
                    None => {
                        return Err(SystemError::InvalidRequest {
                            error: format!(
                                "no balance available for account address {}",
                                contract_addr
                            ),
                            request: Default::default(),
                        })
                        .into()
                    }
                };

                Ok(to_binary(&BalanceResponse {
                    balance: *user_balance,
                })
                .into())
                .into()
            }

            Cw20QueryMsg::TokenInfo {} => {
                let token_info_response = match self.token_info_responses.get(contract_addr) {
                    Some(tir) => tir,
                    None => {
                        return Err(SystemError::InvalidRequest {
                            error: format!(
                                "no token_info mock for account address {}",
                                contract_addr
                            ),
                            request: Default::default(),
                        })
                        .into()
                    }
                };

                Ok(to_binary(token_info_response).into()).into()
            }

            other_query => Err(SystemError::InvalidRequest {
                error: format!("[mock]: query not supported {:?}", other_query),
                request: Default::default(),
            })
            .into(),
        }
    }

    pub fn handle_ma_token_query(
        &self,
        contract_addr: &Addr,
        query: ma_token::msg::QueryMsg,
    ) -> QuerierResult {
        match query {
            ma_token::msg::QueryMsg::BalanceAndTotalSupply { address } => {
                let contract_balances = match self.balances.get(contract_addr) {
                    Some(balances) => balances,
                    None => {
                        return Err(SystemError::InvalidRequest {
                            error: format!(
                                "no balance available for account address {}",
                                contract_addr
                            ),
                            request: Default::default(),
                        })
                        .into()
                    }
                };

                let user_balance = match contract_balances.get(&Addr::unchecked(address)) {
                    Some(balance) => balance,
                    None => {
                        return Err(SystemError::InvalidRequest {
                            error: format!(
                                "no balance available for account address {}",
                                contract_addr
                            ),
                            request: Default::default(),
                        })
                        .into()
                    }
                };
                let token_info_response = match self.token_info_responses.get(contract_addr) {
                    Some(tir) => tir,
                    None => {
                        return Err(SystemError::InvalidRequest {
                            error: format!(
                                "no token_info mock for account address {}",
                                contract_addr
                            ),
                            request: Default::default(),
                        })
                        .into()
                    }
                };

                Ok(to_binary(&ma_token::msg::BalanceAndTotalSupplyResponse {
                    balance: *user_balance,
                    total_supply: token_info_response.total_supply,
                })
                .into())
                .into()
            }

            other_query => Err(SystemError::InvalidRequest {
                error: format!("[mock]: query not supported {:?}", other_query),
                request: Default::default(),
            })
            .into(),
        }
    }
}

pub fn mock_token_info_response() -> TokenInfoResponse {
    TokenInfoResponse {
        name: "".to_string(),
        symbol: "".to_string(),
        decimals: 0,
        total_supply: Uint128::zero(),
    }
}
