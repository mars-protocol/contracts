use cosmwasm_std::{Coin, Decimal, QuerierWrapper, StdResult};
use mars_types::params::{AssetParams, QueryMsg};

pub fn query_asset_params(
    querier: &QuerierWrapper,
    params: impl Into<String>,
    denom: impl Into<String>,
) -> StdResult<AssetParams> {
    querier.query_wasm_smart(
        params.into(),
        &QueryMsg::AssetParams {
            denom: denom.into(),
        },
    )
}

pub fn query_target_health_factor(
    querier: &QuerierWrapper,
    params: impl Into<String>,
) -> StdResult<Decimal> {
    querier.query_wasm_smart(params.into(), &QueryMsg::TargetHealthFactor {})
}

pub fn query_total_deposit(
    querier: &QuerierWrapper,
    params: impl Into<String>,
    denom: impl Into<String>,
) -> StdResult<Coin> {
    querier.query_wasm_smart(
        params.into(),
        &QueryMsg::TotalDeposit {
            denom: denom.into(),
        },
    )
}
