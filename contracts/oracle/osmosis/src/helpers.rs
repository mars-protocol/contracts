use mars_oracle_base::{ContractError, ContractResult};
use mars_osmosis::helpers::{has_denom, Pool};

/// Assert the Osmosis pool indicated by `pool_id` is of XYK type and assets are OSMO and `denom`
pub fn assert_osmosis_pool_assets(
    pool: &Pool,
    denom: &str,
    base_denom: &str,
) -> ContractResult<()> {
    assert_osmosis_xyk_pool(pool)?;

    if !has_denom(base_denom, &pool.pool_assets) {
        return Err(ContractError::InvalidPriceSource {
            reason: format!("pool {} does not contain the base denom {}", pool.id, base_denom),
        });
    }

    if !has_denom(denom, &pool.pool_assets) {
        return Err(ContractError::InvalidPriceSource {
            reason: format!("pool {} does not contain {}", pool.id, denom),
        });
    }

    Ok(())
}

/// Assert the Osmosis pool indicated by `pool_id` is of XYK type
pub fn assert_osmosis_xyk_pool(pool: &Pool) -> ContractResult<()> {
    if pool.pool_assets.len() != 2 {
        return Err(ContractError::InvalidPriceSource {
            reason: format!(
                "expecting pool {} to contain exactly two coins; found {}",
                pool.id,
                pool.pool_assets.len()
            ),
        });
    }

    if pool.pool_assets[0].weight != pool.pool_assets[1].weight {
        return Err(ContractError::InvalidPriceSource {
            reason: format!("assets in pool {} do not have equal weights", pool.id),
        });
    }

    Ok(())
}
