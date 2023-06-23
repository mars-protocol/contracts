use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Decimal, QuerierWrapper, StdResult};
use mars_params::{
    msg::QueryMsg,
    types::{asset::AssetParams, vault::VaultConfig},
};

#[cw_serde]
pub struct ParamsBase<T>(T);

impl<T> ParamsBase<T> {
    pub fn new(address: T) -> ParamsBase<T> {
        ParamsBase(address)
    }

    pub fn address(&self) -> &T {
        &self.0
    }
}

pub type ParamsUnchecked = ParamsBase<String>;
pub type Params = ParamsBase<Addr>;

impl From<Params> for ParamsUnchecked {
    fn from(mars_params: Params) -> Self {
        Self(mars_params.0.to_string())
    }
}

impl ParamsUnchecked {
    pub fn check(&self, api: &dyn Api) -> StdResult<Params> {
        Ok(ParamsBase(api.addr_validate(self.address())?))
    }
}

impl Params {
    pub fn query_asset_params(
        &self,
        querier: &QuerierWrapper,
        denom: &str,
    ) -> StdResult<AssetParams> {
        querier.query_wasm_smart(
            self.address().to_string(),
            &QueryMsg::AssetParams {
                denom: denom.to_string(),
            },
        )
    }

    pub fn query_vault_config(
        &self,
        querier: &QuerierWrapper,
        vault_address: &Addr,
    ) -> StdResult<VaultConfig> {
        querier.query_wasm_smart(
            self.address().to_string(),
            &QueryMsg::VaultConfig {
                address: vault_address.to_string(),
            },
        )
    }

    pub fn query_target_health_factor(&self, querier: &QuerierWrapper) -> StdResult<Decimal> {
        querier.query_wasm_smart(self.address().to_string(), &QueryMsg::TargetHealthFactor {})
    }
}
