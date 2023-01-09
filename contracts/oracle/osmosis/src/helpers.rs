use mars_oracle_base::{ContractError, ContractResult};
use mars_osmosis::helpers::{has_denom, Pool};

use crate::DowntimeDetector;

/// 48 hours in seconds
const TWO_DAYS_IN_SECONDS: u64 = 172800u64;

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

/// Assert Osmosis twap configuration
pub fn assert_osmosis_twap(
    window_size: u64,
    downtime_detector: &Option<DowntimeDetector>,
) -> ContractResult<()> {
    if window_size > TWO_DAYS_IN_SECONDS {
        return Err(ContractError::InvalidPriceSource {
            reason: format!("expecting window size to be within {TWO_DAYS_IN_SECONDS} sec"),
        });
    }

    if let Some(dd) = downtime_detector {
        if dd.recovery == 0 {
            return Err(ContractError::InvalidPriceSource {
                reason: "downtime recovery can't be 0".to_string(),
            });
        }
    }

    Ok(())
}
