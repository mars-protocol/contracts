use cosmwasm_std::{to_binary, Addr, Binary, ContractResult, QuerierResult, Uint128};
use std::collections::HashMap;

use crate::incentives::msg::QueryMsg;

pub struct IncentivesQuerier {
    /// incentives contract address to be used in queries
    pub incentives_address: Addr,
    /// maps human address to a specific unclaimed Mars rewards balance (which will be staked with the staking contract and distributed as xMars)
    pub unclaimed_rewards_at: HashMap<Addr, Uint128>,
}

impl Default for IncentivesQuerier {
    fn default() -> Self {
        IncentivesQuerier {
            incentives_address: Addr::unchecked(""),
            unclaimed_rewards_at: HashMap::new(),
        }
    }
}

impl IncentivesQuerier {
    pub fn handle_query(&self, contract_addr: &Addr, query: QueryMsg) -> QuerierResult {
        if contract_addr != &self.incentives_address {
            panic!( "[mock]: made an incentives query but incentive contract address is incorrect, was: {}, should be {}",  contract_addr, self.incentives_address );
        }

        let ret: ContractResult<Binary> = match query {
            QueryMsg::UserUnclaimedRewards { user_address } => {
                match self
                    .unclaimed_rewards_at
                    .get(&(Addr::unchecked(user_address.clone())))
                {
                    Some(balance) => to_binary(balance).into(),
                    None => Err(format!(
                        "[mock]: no unclaimed rewards for account address {}",
                        &user_address
                    ))
                    .into(),
                }
            }
            _ => Err("[mock]: query not supported ").into(),
        };

        Ok(ret).into()
    }
}
