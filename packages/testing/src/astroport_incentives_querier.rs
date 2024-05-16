use std::{collections::HashMap, error::Error};

use cosmwasm_std::{to_json_binary, Addr, Binary, ContractResult, QuerierResult, Uint128};
// todo this dependency is probably not correct?
use cw_it::astroport::astroport_v3::{asset, asset::Asset, incentives::QueryMsg};

pub struct AstroportIncentivesQuerier {
    /// Holds mock rewards, key is (user, lp_token_denom)
    pub unclaimed_rewards: HashMap<(String, String), Vec<Asset>>,
    pub deposits: HashMap<(String, String), Uint128>,
    pub incentives_addr: Addr,
}

impl Default for AstroportIncentivesQuerier {
    fn default() -> Self {
        AstroportIncentivesQuerier {
            incentives_addr: Addr::unchecked(""),
            unclaimed_rewards: HashMap::new(),
            deposits: HashMap::new(),
        }
    }
}

impl AstroportIncentivesQuerier {
    pub fn handle_query(&self, contract_addr: &Addr, query: QueryMsg) -> QuerierResult {
        if contract_addr != self.incentives_addr {
            panic!(
                "[mock]: made an astroport incentives query but astroport incentive contract address is incorrect, was: {}, should be {}",
                contract_addr,
                self.incentives_addr,
            );
        }

        let ret: ContractResult<Binary> = match query {
            QueryMsg::Deposit {
                lp_token,
                user,
            } => to_json_binary(self.deposits.get(&(user, lp_token)).unwrap_or(&Uint128::zero()))
                .into(),
            QueryMsg::PendingRewards {
                lp_token,
                user,
            } => to_json_binary(&self.unclaimed_rewards.get(&(user, lp_token)).unwrap_or(&vec![]))
                .into(),

            _ => Err("[mock]: query not supported").into(),
        };

        Ok(ret).into()
    }
}
