use std::collections::HashMap;

use cosmwasm_std::{to_binary, Addr, QuerierResult, SystemError, Uint128};
use cw20::BalanceResponse;

use crate::xmars_token;

#[derive(Clone, Debug)]
pub struct XMarsQuerier {
    /// xmars token address to be used in queries
    pub xmars_address: Addr,
    /// maps human address and a block to a specific xmars balance
    pub balances_at: HashMap<(Addr, u64), Uint128>,
    /// maps block to a specific xmars balance
    pub total_supplies_at: HashMap<u64, Uint128>,
}

impl XMarsQuerier {
    pub fn handle_query(
        &self,
        contract_addr: &Addr,
        query: xmars_token::msg::QueryMsg,
    ) -> QuerierResult {
        if contract_addr != &self.xmars_address {
            panic!(
                "[mock]: made an xmars query but xmars address is incorrect, was: {}, should be {}",
                contract_addr, self.xmars_address
            );
        }

        match query {
            xmars_token::msg::QueryMsg::BalanceAt { address, block } => {
                match self
                    .balances_at
                    .get(&(Addr::unchecked(address.clone()), block))
                {
                    Some(balance) => {
                        Ok(to_binary(&BalanceResponse { balance: *balance }).into()).into()
                    }
                    None => Err(SystemError::InvalidRequest {
                        error: format!(
                            "[mock]: no balance at block {} for account address {}",
                            block, &address
                        ),
                        request: Default::default(),
                    })
                    .into(),
                }
            }

            xmars_token::msg::QueryMsg::TotalSupplyAt { block } => {
                match self.total_supplies_at.get(&block) {
                    Some(balance) => Ok(to_binary(&xmars_token::TotalSupplyResponse {
                        total_supply: *balance,
                    })
                    .into())
                    .into(),
                    None => Err(SystemError::InvalidRequest {
                        error: format!("[mock]: no total supply at block {}", block),
                        request: Default::default(),
                    })
                    .into(),
                }
            }

            other_query => Err(SystemError::InvalidRequest {
                error: format!("[mock]: query not supported {:?}", other_query),
                request: Default::default(),
            })
            .into(),
        }
    }
}

impl Default for XMarsQuerier {
    fn default() -> Self {
        XMarsQuerier {
            xmars_address: Addr::unchecked(""),
            balances_at: HashMap::new(),
            total_supplies_at: HashMap::new(),
        }
    }
}
