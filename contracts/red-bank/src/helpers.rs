use cosmwasm_std::{Decimal, QuerierWrapper, StdResult};
use mars_params::{msg::QueryMsg, types::AssetParams};

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

pub fn query_close_factor(
    querier: &QuerierWrapper,
    params: impl Into<String>,
) -> StdResult<Decimal> {
    querier.query_wasm_smart(params.into(), &QueryMsg::MaxCloseFactor {})
}
