use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, QuerierWrapper, StdResult};
use mars_red_bank_types::oracle::ActionKind;
use mars_rover_health_types::{AccountKind, HealthState, HealthValuesResponse, QueryMsg};

#[cw_serde]
pub struct HealthContractBase<T>(T);

impl<T> HealthContractBase<T> {
    pub fn new(address: T) -> HealthContractBase<T> {
        HealthContractBase(address)
    }

    pub fn address(&self) -> &T {
        &self.0
    }
}

pub type HealthContractUnchecked = HealthContractBase<String>;
pub type HealthContract = HealthContractBase<Addr>;

impl From<HealthContract> for HealthContractUnchecked {
    fn from(health: HealthContract) -> Self {
        Self(health.address().to_string())
    }
}

impl HealthContractUnchecked {
    pub fn check(&self, api: &dyn Api) -> StdResult<HealthContract> {
        Ok(HealthContractBase::new(api.addr_validate(self.address())?))
    }
}

impl HealthContract {
    pub fn query_health_state(
        &self,
        querier: &QuerierWrapper,
        account_id: &str,
        kind: AccountKind,
        action: ActionKind,
    ) -> StdResult<HealthState> {
        querier.query_wasm_smart(
            self.address().to_string(),
            &QueryMsg::HealthState {
                account_id: account_id.to_string(),
                kind,
                action,
            },
        )
    }

    pub fn query_health_values(
        &self,
        querier: &QuerierWrapper,
        account_id: &str,
        kind: AccountKind,
        action: ActionKind,
    ) -> StdResult<HealthValuesResponse> {
        querier.query_wasm_smart(
            self.address().to_string(),
            &QueryMsg::HealthValues {
                account_id: account_id.to_string(),
                kind,
                action,
            },
        )
    }
}
