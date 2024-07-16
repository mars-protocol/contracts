use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, Addr, Api, Coin, CosmosMsg, QuerierWrapper, StdResult, WasmMsg,
};

use crate::{
    credit_manager::ActionCoin,
    incentives::{ExecuteMsg, PaginatedStakedLpResponse, QueryMsg, StakedLpPositionResponse},
};

#[cw_serde]
pub struct IncentivesUnchecked(String);

impl IncentivesUnchecked {
    pub fn new(address: String) -> Self {
        Self(address)
    }

    pub fn address(&self) -> &str {
        &self.0
    }

    pub fn check(&self, api: &dyn Api, credit_manager: Addr) -> StdResult<Incentives> {
        let addr = api.addr_validate(self.address())?;
        Ok(Incentives::new(addr, credit_manager))
    }
}

impl From<Incentives> for IncentivesUnchecked {
    fn from(red_bank: Incentives) -> Self {
        Self(red_bank.addr.to_string())
    }
}

#[cw_serde]
pub struct Incentives {
    pub addr: Addr,
    credit_manager: Addr,
}

impl Incentives {
    pub fn new(addr: Addr, credit_manager: Addr) -> Self {
        Self {
            addr,
            credit_manager,
        }
    }

    pub fn claim_rewards_msg(&self, account_id: &str) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.addr.to_string(),
            msg: to_json_binary(&ExecuteMsg::ClaimRewards {
                account_id: Some(account_id.to_string()),
                start_after_collateral_denom: None,
                start_after_incentive_denom: None,
                limit: None,
            })?,
            funds: vec![],
        }))
    }

    pub fn claim_staked_astro_lp_rewards_msg(
        &self,
        account_id: &str,
        lp_denom: &str,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.addr.to_string(),
            msg: to_json_binary(&ExecuteMsg::ClaimStakedAstroLpRewards {
                account_id: account_id.to_string(),
                lp_denom: lp_denom.to_string(),
            })?,
            funds: vec![],
        }))
    }

    pub fn stake_astro_lp_msg(&self, account_id: &str, lp_coin: Coin) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.addr.to_string(),
            msg: to_json_binary(&ExecuteMsg::StakeAstroLp {
                account_id: account_id.to_string(),
                lp_coin: lp_coin.clone(),
            })?,
            funds: vec![lp_coin],
        }))
    }

    pub fn unstake_astro_lp_msg(&self, account_id: &str, lp_coin: &Coin) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.addr.to_string(),
            msg: to_json_binary(&ExecuteMsg::UnstakeAstroLp {
                account_id: account_id.to_string(),
                lp_coin: ActionCoin::from(lp_coin),
            })?,
            funds: vec![],
        }))
    }

    pub fn query_unclaimed_rewards(
        &self,
        querier: &QuerierWrapper,
        account_id: &str,
    ) -> StdResult<Vec<Coin>> {
        querier.query_wasm_smart(
            self.addr.to_string(),
            &QueryMsg::UserUnclaimedRewards {
                user: self.credit_manager.to_string(),
                account_id: Some(account_id.to_string()),
                start_after_collateral_denom: None,
                start_after_incentive_denom: None,
                limit: None,
            },
        )
    }

    pub fn query_staked_astro_lp_rewards(
        &self,
        querier: &QuerierWrapper,
        account_id: &str,
        lp_denom: &str,
    ) -> StdResult<Vec<Coin>> {
        querier.query_wasm_smart(
            self.addr.to_string(),
            &QueryMsg::StakedAstroLpRewards {
                account_id: account_id.to_string(),
                lp_denom: lp_denom.to_string(),
            },
        )
    }

    pub fn query_staked_astro_lp_position(
        &self,
        querier: &QuerierWrapper,
        account_id: &str,
        lp_denom: &str,
    ) -> StdResult<StakedLpPositionResponse> {
        querier.query_wasm_smart(
            self.addr.to_string(),
            &QueryMsg::StakedAstroLpPosition {
                account_id: account_id.to_string(),
                lp_denom: lp_denom.to_string(),
            },
        )
    }

    pub fn query_staked_astro_lp_positions(
        &self,
        querier: &QuerierWrapper,
        account_id: &str,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<PaginatedStakedLpResponse> {
        querier.query_wasm_smart(
            self.addr.to_string(),
            &QueryMsg::StakedAstroLpPositions {
                account_id: account_id.to_string(),
                start_after,
                limit,
            },
        )
    }

    pub fn query_all_staked_astro_lp_coins(
        &self,
        querier: &QuerierWrapper,
        account_id: &str,
    ) -> StdResult<Vec<Coin>> {
        let mut start_after = Option::<String>::None;
        let mut has_more = true;
        let mut all_coins = Vec::new();

        while has_more {
            let response =
                self.query_staked_astro_lp_positions(querier, account_id, start_after, None)?;
            for item in response.data {
                if !item.lp_coin.amount.is_zero() {
                    all_coins.push(item.lp_coin);
                }
            }
            start_after = all_coins.last().map(|item| item.denom.clone());
            has_more = response.metadata.has_more;
        }

        Ok(all_coins)
    }
}
