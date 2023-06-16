use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, QuerierWrapper, StdResult};
use mars_rover_health_types::{AccountKind, HealthResponse, QueryMsg};

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
    pub fn query_health(
        &self,
        querier: &QuerierWrapper,
        account_id: &str,
        kind: AccountKind,
    ) -> StdResult<HealthResponse> {
        querier.query_wasm_smart(
            self.address().to_string(),
            &QueryMsg::Health {
                account_id: account_id.to_string(),
                kind,
            },
        )
    }
}
