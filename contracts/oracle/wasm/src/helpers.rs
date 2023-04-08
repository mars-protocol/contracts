use astroport::{
    asset::{Asset, AssetInfo, PairInfo},
    pair::{PoolResponse, QueryMsg as PairQueryMsg},
};
use cosmwasm_std::{QuerierWrapper, StdResult, Uint128};
use mars_oracle_base::{ContractError, ContractResult};

pub fn query_astroport_pair_info(
    querier: &QuerierWrapper,
    pair_contract: impl Into<String>,
) -> StdResult<PairInfo> {
    querier.query_wasm_smart(pair_contract, &PairQueryMsg::Pair {})
}

pub fn query_astroport_pool(
    querier: &QuerierWrapper,
    pair_contract: impl Into<String>,
) -> StdResult<PoolResponse> {
    querier.query_wasm_smart(pair_contract, &PairQueryMsg::Pool {})
}

pub fn astro_native_info(denom: &str) -> AssetInfo {
    AssetInfo::NativeToken {
        denom: denom.to_string(),
    }
}

pub fn astro_native_asset(denom: impl Into<String>, amount: impl Into<Uint128>) -> Asset {
    Asset {
        info: astro_native_info(&denom.into()),
        amount: amount.into(),
    }
}

pub fn assert_astroport_pair_contains_denoms(
    pair_info: &PairInfo,
    denoms: &[&str],
) -> ContractResult<()> {
    for denom in denoms {
        if !pair_info.asset_infos.contains(&astro_native_info(denom)) {
            return Err(ContractError::InvalidPriceSource {
                reason: format!(
                    "pair {} does not contain the denom {}",
                    pair_info.contract_addr, denom
                ),
            });
        }
    }

    Ok(())
}
