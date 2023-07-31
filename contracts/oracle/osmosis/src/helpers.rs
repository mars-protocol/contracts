use mars_oracle_base::{ContractError, ContractResult};
use mars_osmosis::{
    helpers::{CommonPoolData, Pool},
    BalancerPool,
};

use crate::DowntimeDetector;

/// 48 hours in seconds
const TWO_DAYS_IN_SECONDS: u64 = 172800u64;

/// Assert the Osmosis pool indicated by `pool_id` is of Balancer XYK or StableSwap and assets are OSMO and `denom`
pub fn assert_osmosis_pool_assets(
    pool: &Pool,
    denom: &str,
    base_denom: &str,
) -> ContractResult<()> {
    match pool {
        Pool::Balancer(balancer_pool) => {
            assert_osmosis_xyk_pool(balancer_pool)?;
        }
        Pool::StableSwap(_) => {}
    };

    assert_osmosis_pool_contains_two_assets(pool, denom, base_denom)?;

    Ok(())
}

/// Assert the Osmosis pool indicated by `pool_id` is Balancer XYK type
pub fn assert_osmosis_xyk_lp_pool(pool: &Pool) -> ContractResult<()> {
    match pool {
        Pool::Balancer(balancer_pool) => assert_osmosis_xyk_pool(balancer_pool)?,
        Pool::StableSwap(stable_swap_pool) => {
            return Err(ContractError::InvalidPriceSource {
                reason: format!("StableSwap pool not supported. Pool id {}", stable_swap_pool.id),
            });
        }
    };

    Ok(())
}

fn assert_osmosis_pool_contains_two_assets(
    pool: &Pool,
    denom: &str,
    base_denom: &str,
) -> ContractResult<()> {
    let pool_id = pool.get_pool_id();
    let pool_denoms = pool.get_pool_denoms();

    if !pool_denoms.contains(&base_denom.to_string()) {
        return Err(ContractError::InvalidPriceSource {
            reason: format!("pool {} does not contain the base denom {}", pool_id, base_denom),
        });
    }

    if !pool_denoms.contains(&denom.to_string()) {
        return Err(ContractError::InvalidPriceSource {
            reason: format!("pool {} does not contain {}", pool_id, denom),
        });
    }

    Ok(())
}

/// Assert the Osmosis pool indicated by `pool_id` is of XYK type
pub fn assert_osmosis_xyk_pool(pool: &BalancerPool) -> ContractResult<()> {
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
