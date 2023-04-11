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

/// Asserts that the pair contains exactly the specified denoms.
pub fn assert_astroport_pair_contains_denoms(
    pair_info: &PairInfo,
    denoms: &[&str],
) -> ContractResult<()> {
    // sort denoms to compare them
    let mut pair_denoms: Vec<String> =
        pair_info.asset_infos.iter().map(|a| a.to_string()).collect();
    let mut denoms = denoms.to_vec();
    denoms.sort();
    pair_denoms.sort();

    if denoms != pair_denoms {
        return Err(ContractError::InvalidPriceSource {
            reason: format!(
                "pair {} does not contain the denoms {:?}",
                pair_info.contract_addr, denoms
            ),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::Addr;

    use super::*;

    #[test]
    fn test_assert_astroport_pair_contains_denoms() {
        let pair_info = PairInfo {
            contract_addr: Addr::unchecked("pair_contract"),
            asset_infos: vec![
                astro_native_info("uusd"),
                astro_native_info("uatom"),
                astro_native_info("uluna"),
            ],
            liquidity_token: Addr::unchecked("liquidity_token"),
            pair_type: astroport::factory::PairType::Xyk {},
        };

        assert_astroport_pair_contains_denoms(&pair_info, &["uusd", "uatom", "uluna"]).unwrap();
        assert_astroport_pair_contains_denoms(&pair_info, &["uusd", "uluna", "uatom"]).unwrap();
        assert_astroport_pair_contains_denoms(&pair_info, &["uatom", "uusd", "uluna"]).unwrap();
        assert_astroport_pair_contains_denoms(&pair_info, &["uusd"]).unwrap_err();
        assert_astroport_pair_contains_denoms(&pair_info, &["uatom"]).unwrap_err();
        assert_astroport_pair_contains_denoms(&pair_info, &["uluna"]).unwrap_err();

        assert_astroport_pair_contains_denoms(&pair_info, &["uusd", "uatom", "uluna", "ukrw"])
            .unwrap_err();
        assert_astroport_pair_contains_denoms(&pair_info, &["uusd", "uatom", "ukrw"]).unwrap_err();
        assert_astroport_pair_contains_denoms(&pair_info, &["uusd", "ukrw"]).unwrap_err();
        assert_astroport_pair_contains_denoms(&pair_info, &["uatom", "ukrw"]).unwrap_err();
        assert_astroport_pair_contains_denoms(&pair_info, &["uluna", "ukrw"]).unwrap_err();
        assert_astroport_pair_contains_denoms(&pair_info, &["ukrw"]).unwrap_err();
    }
}
