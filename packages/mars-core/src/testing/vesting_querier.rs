use std::collections::HashMap;

use cosmwasm_std::{to_binary, Addr, QuerierResult, Uint128};

use crate::vesting;

pub struct VestingQuerier {
    /// vesting contract address to be used in queries
    pub vesting_address: Addr,
    /// maps human address and a block to a specific voting power
    pub voting_power_at: HashMap<(Addr, u64), Uint128>,
}

impl Default for VestingQuerier {
    fn default() -> Self {
        VestingQuerier {
            vesting_address: Addr::unchecked(""),
            voting_power_at: HashMap::new(),
        }
    }
}

impl VestingQuerier {
    pub fn handle_query(
        &self,
        contract_addr: &Addr,
        query: vesting::msg::QueryMsg,
    ) -> QuerierResult {
        if contract_addr != &self.vesting_address {
            panic!(
                "[mock]: made an vesting query but incentive contract address is incorrect, was: {}, should be {}",  
                contract_addr,
                self.vesting_address
            );
        }

        match query {
            vesting::msg::QueryMsg::VotingPowerAt {
                user_address,
                block,
            } => {
                match self
                    .voting_power_at
                    .get(&(Addr::unchecked(user_address), block))
                {
                    Some(voting_power) => Ok(to_binary(voting_power).into()).into(),
                    // If voting power is not set, return zero
                    None => Ok(to_binary(&Uint128::zero()).into()).into(),
                }
            }

            _ => panic!("[mock]: unimplemented"),
        }
    }
}
