use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_binary, Addr, Api, Coin, CosmosMsg, QuerierWrapper, StdResult, WasmMsg};
use mars_red_bank_types::{incentives, incentives::QueryMsg};

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
            msg: to_binary(&incentives::ExecuteMsg::ClaimRewards {
                account_id: Some(account_id.to_string()),
                start_after_collateral_denom: None,
                start_after_incentive_denom: None,
                limit: None,
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
}
