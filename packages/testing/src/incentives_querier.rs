use std::collections::HashMap;

use cosmwasm_std::{to_json_binary, Addr, Binary, Coin, ContractResult, QuerierResult, Uint128};
use mars_types::incentives::QueryMsg;

pub struct IncentivesQuerier {
    /// incentives contract address to be used in queries
    pub incentives_addr: Addr,
    /// maps human address and incentive denom to a specific unclaimed rewards balance
    pub unclaimed_rewards_at: HashMap<(Addr, String), Uint128>,
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
        if contract_addr != self.incentives_addr {
            panic!(
                "[mock]: made an incentives query but incentive contract address is incorrect, was: {}, should be {}",
                contract_addr,
                self.incentives_addr,
            );
        }

        let ret: ContractResult<Binary> = match query {
            QueryMsg::UserUnclaimedRewards {
                user: _,
                account_id: _,
                start_after_collateral_denom: _,
                start_after_incentive_denom: _,
                limit: _,
            } => {
                let unclaimed_rewards = self
                    .unclaimed_rewards_at
                    .iter()
                    .map(|((_, denom), amount)| Coin {
                        denom: denom.clone(),
                        amount: *amount,
                    })
                    .collect::<Vec<_>>();
                to_json_binary(&unclaimed_rewards).into()
            }
            _ => Err("[mock]: query not supported").into(),
        };

        Ok(ret).into()
    }
}
