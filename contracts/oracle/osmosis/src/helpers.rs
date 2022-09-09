use std::any::type_name;
use std::str::FromStr;

use cosmwasm_std::{Decimal, QuerierWrapper, StdError, StdResult};

use osmosis_std::shim::Timestamp;
use osmosis_std::types::osmosis::gamm::twap::v1beta1::TwapQuerier;
use osmosis_std::types::osmosis::gamm::v1beta1::{GammQuerier, Pool, PoolAsset};
use prost::{DecodeError, Message};

use mars_oracle_base::{ContractError, ContractResult};
use mars_outpost::error::MarsError;

/// Assert the Osmosis pool indicated by `pool_id` contains exactly two assets, and they are OSMO and `denom`
pub fn assert_osmosis_pool_assets(
    querier: &QuerierWrapper,
    pool_id: u64,
    denom: &str,
    base_denom: &str,
) -> ContractResult<()> {
    let pool = query_osmosis_pool(querier, pool_id)?;

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

/// Query an Osmosis pool's coin depths and the supply of of liquidity token
fn query_osmosis_pool(querier: &QuerierWrapper, pool_id: u64) -> StdResult<Pool> {
    let pool_res = GammQuerier::new(querier).pool(pool_id)?;
    let pool_type = type_name::<Pool>();
    let pool = pool_res.pool.ok_or_else(|| StdError::not_found(pool_type))?;
    let pool_res: Result<Pool, DecodeError> = Message::decode(pool.value.as_slice());
    let pool = pool_res.map_err(|_| MarsError::Deserialize {
        target_type: pool_type.to_string(),
    })?;
    Ok(pool)
}

fn has_denom(denom: &str, pool_assets: &[PoolAsset]) -> bool {
    pool_assets.iter().flat_map(|asset| &asset.token).any(|coin| coin.denom == denom)
}

/// Query the spot price of a coin, denominated in OSMO
pub fn query_osmosis_spot_price(
    querier: &QuerierWrapper,
    pool_id: u64,
    denom: &str,
    base_denom: &str,
) -> StdResult<Decimal> {
    let spot_price_res =
        GammQuerier::new(querier).spot_price(pool_id, denom.to_string(), base_denom.to_string())?;
    let price = Decimal::from_str(&spot_price_res.spot_price)?;
    Ok(price)
}

/// Query the twap price of a coin, denominated in OSMO.
/// `start_time` must be within 48 hours of current block time.
pub fn query_osmosis_twap_price(
    querier: &QuerierWrapper,
    pool_id: u64,
    denom: &str,
    base_denom: &str,
    start_time: u64,
    end_time: u64,
) -> StdResult<Decimal> {
    let arithmetic_twap_res = TwapQuerier::new(querier).get_arithmetic_twap(
        pool_id,
        denom.to_string(),
        base_denom.to_string(),
        Some(Timestamp {
            seconds: start_time as i64,
            nanos: 0,
        }),
        Some(Timestamp {
            seconds: end_time as i64,
            nanos: 0,
        }),
    )?;
    let price = Decimal::from_str(&arithmetic_twap_res.arithmetic_twap)?;
    Ok(price)
}
