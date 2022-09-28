use cosmwasm_std::QuerierWrapper;

use mars_oracle_base::{ContractError, ContractResult};
use mars_osmosis::helpers::{has_denom, query_pool};

/// Assert the Osmosis pool indicated by `pool_id` contains exactly two assets, and they are OSMO and `denom`
pub fn assert_osmosis_pool_assets(
    querier: &QuerierWrapper,
    pool_id: u64,
    denom: &str,
    base_denom: &str,
) -> ContractResult<()> {
    let pool = query_pool(querier, pool_id)?;

    if pool.pool_assets.len() != 2 {
        return Err(ContractError::InvalidPriceSource {
            reason: format!(
                "expecting pool {} to contain exactly two coins; found {}",
                pool_id,
                pool.pool_assets.len()
            ),
        });
    }

    if !has_denom(base_denom, &pool.pool_assets) {
        return Err(ContractError::InvalidPriceSource {
            reason: format!("pool {} does not contain the base denom {}", pool_id, base_denom),
        });
    }

    if !has_denom(denom, &pool.pool_assets) {
        return Err(ContractError::InvalidPriceSource {
            reason: format!("pool {} does not contain {}", pool_id, denom),
        });
    }

    if pool.pool_assets[0].weight != pool.pool_assets[1].weight {
        return Err(ContractError::InvalidPriceSource {
            reason: format!("assets in pool {} do not have equal weights", pool_id),
        });
    }

    Ok(())
}
