use std::collections::HashMap;

use cosmwasm_std::{to_binary, Addr, Binary, ContractResult, QuerierResult, Uint128};
use mars_outpost::incentives::QueryMsg;

pub struct IncentivesQuerier {
    /// incentives contract address to be used in queries
    pub incentives_addr: Addr,
    /// maps human address to a specific unclaimed Mars rewards balance (which will be staked with the staking contract and distributed as xMars)
    pub unclaimed_rewards_at: HashMap<Addr, Uint128>,
}

impl Default for IncentivesQuerier {
    fn default() -> Self {
        IncentivesQuerier {
            incentives_addr: Addr::unchecked(""),
            unclaimed_rewards_at: HashMap::new(),
        }
    }
}

impl IncentivesQuerier {
    pub fn handle_query(&self, contract_addr: &Addr, query: QueryMsg) -> QuerierResult {
        if contract_addr != &self.incentives_addr {
            panic!(
                "[mock]: made an incentives query but incentive contract address is incorrect, was: {}, should be {}",
                contract_addr,
                self.incentives_addr,
            );
        }

        let ret: ContractResult<Binary> = match query {
            QueryMsg::UserUnclaimedRewards {
                user,
            } => match self.unclaimed_rewards_at.get(&(Addr::unchecked(user.clone()))) {
                Some(balance) => to_binary(balance).into(),
                None => Err(format!("[mock]: no unclaimed rewards for account address {}", &user))
                    .into(),
            },
            _ => Err("[mock]: query not supported").into(),
        };

        Ok(ret).into()
    }
}
