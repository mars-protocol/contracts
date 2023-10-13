use cosmwasm_std::{Addr, Decimal, Deps, Empty, Env, StdError, Uint128};
use cw_storage_plus::Map;
use mars_types::oracle::{ActionKind, Config};
use pyth_sdk_cw::{query_price_feed, Price, PriceFeed, PriceFeedResponse, PriceIdentifier};

use super::*;
use crate::error::ContractError::InvalidPrice;

// We don't support any denom with more than 18 decimals
const MAX_DENOM_DECIMALS: u8 = 18;

/// We want to discriminate which actions should trigger a circuit breaker check.
/// The objective is to allow liquidations to happen without requiring too many checks (always be open for liquidations)
/// while not allowing other actions to be taken in cases of extreme volatility (which could indicate price manipulation attacks).
#[allow(clippy::too_many_arguments)]
pub fn query_pyth_price<P: PriceSourceChecked<Empty>>(
    deps: &Deps,
    env: &Env,
    contract_addr: Addr,
    price_feed_id: PriceIdentifier,
    max_staleness: u64,
    max_confidence: Decimal,
    max_deviation: Decimal,
    denom_decimals: u8,
    config: &Config,
    price_sources: &Map<&str, P>,
    kind: ActionKind,
) -> ContractResult<Decimal> {
    // Use current price source for USD to check how much 1 USD is worth in base_denom
    let usd_price = price_sources
        .load(deps.storage, "usd")
        .map_err(|_| StdError::generic_err("Price source not found for denom 'usd'"))?
        .query_price(deps, env, "usd", config, price_sources, kind.clone())?;

    let price_feed_response = query_price_feed(&deps.querier, contract_addr, price_feed_id)?;

    match kind {
        ActionKind::Default => query_pyth_price_for_default(
            env,
            max_staleness,
            max_confidence,
            max_deviation,
            denom_decimals,
            usd_price,
            price_feed_response,
        ),
        ActionKind::Liquidation => query_pyth_price_for_liquidation(
            env,
            max_staleness,
            denom_decimals,
            usd_price,
            price_feed_response,
        ),
    }
}

fn query_pyth_price_for_default(
    env: &Env,
    max_staleness: u64,
    max_confidence: Decimal,
    max_deviation: Decimal,
    denom_decimals: u8,
    usd_price: Decimal,
    price_feed_response: PriceFeedResponse,
) -> ContractResult<Decimal> {
    let price_feed = price_feed_response.price_feed;

    let current_time = env.block.time.seconds();
    let current_price =
        assert_pyth_current_price_not_too_old(price_feed, current_time, max_staleness)?;
    let ema_price = assert_pyth_ema_price_not_too_old(price_feed, current_time, max_staleness)?;

    // Check if the current and EMA price is > 0
    if current_price.price <= 0 || ema_price.price <= 0 {
        return Err(InvalidPrice {
            reason: "price can't be <= 0".to_string(),
        });
    }

    let current_price_dec = scale_to_exponent(current_price.price as u128, current_price.expo)?;
    let ema_price_dec = scale_to_exponent(ema_price.price as u128, ema_price.expo)?;

    assert_pyth_price_confidence(current_price, ema_price_dec, max_confidence)?;
    assert_pyth_price_deviation(current_price_dec, ema_price_dec, max_deviation)?;

    let current_price_dec = scale_pyth_price(
        current_price.price as u128,
        current_price.expo,
        denom_decimals,
        usd_price,
    )?;

    Ok(current_price_dec)
}

fn query_pyth_price_for_liquidation(
    env: &Env,
    max_staleness: u64,
    denom_decimals: u8,
    usd_price: Decimal,
    price_feed_response: PriceFeedResponse,
) -> ContractResult<Decimal> {
    let price_feed = price_feed_response.price_feed;

    let current_time = env.block.time.seconds();
    let current_price =
        assert_pyth_current_price_not_too_old(price_feed, current_time, max_staleness)?;

    // Check if the current price is > 0
    if current_price.price <= 0 {
        return Err(InvalidPrice {
            reason: "price can't be <= 0".to_string(),
        });
    }

    let current_price_dec = scale_pyth_price(
        current_price.price as u128,
        current_price.expo,
        denom_decimals,
        usd_price,
    )?;

    Ok(current_price_dec)
}

/// Assert Pyth configuration
pub fn assert_pyth(
    max_confidence: Decimal,
    max_deviation: Decimal,
    denom_decimals: u8,
) -> ContractResult<()> {
    if !max_confidence.le(&Decimal::percent(20u64)) {
        return Err(ContractError::InvalidPriceSource {
            reason: "max_confidence must be in the range of <0;0.2>".to_string(),
        });
    }

    if !max_deviation.le(&Decimal::percent(20u64)) {
        return Err(ContractError::InvalidPriceSource {
            reason: "max_deviation must be in the range of <0;0.2>".to_string(),
        });
    }

    if denom_decimals > MAX_DENOM_DECIMALS {
        return Err(ContractError::InvalidPriceSource {
            reason: format!("denom_decimals must be <= {}", MAX_DENOM_DECIMALS),
        });
    }

    Ok(())
}

/// Check if the current price is not too old
pub fn assert_pyth_current_price_not_too_old(
    price_feed: PriceFeed,
    current_time: u64,
    max_staleness: u64,
) -> ContractResult<Price> {
    let current_price_opt = price_feed.get_price_no_older_than(current_time as i64, max_staleness);
    let Some(current_price) = current_price_opt else {
        return Err(InvalidPrice {
            reason: format!(
                "current price publish time is too old/stale. published: {}, now: {}",
                price_feed.get_price_unchecked().publish_time,
                current_time
            ),
        });
    };
    Ok(current_price)
}

/// Check if the ema price is not too old
pub fn assert_pyth_ema_price_not_too_old(
    price_feed: PriceFeed,
    current_time: u64,
    max_staleness: u64,
) -> ContractResult<Price> {
    let ema_price_opt = price_feed.get_ema_price_no_older_than(current_time as i64, max_staleness);
    let Some(ema_price) = ema_price_opt else {
        return Err(InvalidPrice {
            reason: format!(
                "EMA price publish time is too old/stale. published: {}, now: {}",
                price_feed.get_ema_price_unchecked().publish_time,
                current_time
            ),
        });
    };
    Ok(ema_price)
}

/// Check price confidence
pub fn assert_pyth_price_confidence(
    current_price: Price,
    ema_price_dec: Decimal,
    max_confidence: Decimal,
) -> ContractResult<()> {
    let confidence = scale_to_exponent(current_price.conf as u128, current_price.expo)?;
    let price_confidence = confidence.checked_div(ema_price_dec)?;
    if price_confidence > max_confidence {
        return Err(InvalidPrice {
            reason: format!("price confidence deviation {price_confidence} exceeds max allowed {max_confidence}")
        });
    }
    Ok(())
}

/// Check price deviation
pub fn assert_pyth_price_deviation(
    current_price_dec: Decimal,
    ema_price_dec: Decimal,
    max_deviation: Decimal,
) -> ContractResult<()> {
    let delta = current_price_dec.abs_diff(ema_price_dec);
    let price_deviation = delta.checked_div(ema_price_dec)?;
    if price_deviation > max_deviation {
        return Err(InvalidPrice {
            reason: format!(
                "price deviation {price_deviation} exceeds max allowed {max_deviation}"
            ),
        });
    }
    Ok(())
}

/// We have to represent the price for utoken in base_denom.
/// Pyth price should be normalized with token decimals.
///
/// Let's try to convert ATOM/USD reported by Pyth to uatom/base_denom:
/// - base_denom = uusd
/// - price source set for usd (e.g. FIXED price source where 1 usd = 1000000 uusd)
/// - denom_decimals (ATOM) = 6
///
/// 1 ATOM = 10^6 uatom
///
/// 1 ATOM = price * 10^expo USD
/// 10^6 uatom = price * 10^expo * 1000000 uusd
/// uatom = price * 10^expo * 1000000 / 10^6 uusd
/// uatom = price * 10^expo * 1000000 * 10^(-6) uusd
/// uatom/uusd = 1365133270 * 10^(-8) * 1000000 * 10^(-6)
/// uatom/uusd = 1365133270 * 10^(-8) = 13.6513327
///
/// Generalized formula:
/// utoken/uusd = price * 10^expo * usd_price_in_base_denom * 10^(-denom_decimals)
pub fn scale_pyth_price(
    value: u128,
    expo: i32,
    denom_decimals: u8,
    usd_price: Decimal,
) -> ContractResult<Decimal> {
    let pyth_price = scale_to_exponent(value, expo)?;

    let denom_scaled = Decimal::from_atomics(1u128, denom_decimals as u32)?;

    // Multiplication order matters !!! It can overflow doing different ways.
    // usd_price is represented in smallest unit so it can be quite big number and can be used to reduce number of decimals.
    //
    // Let's assume that:
    // - usd_price = 1000000 = 10^6
    // - expo = -8
    // - denom_decimals = 18
    //
    // If we multiply usd_price by denom_scaled firstly we will decrease number of decimals used in next multiplication by pyth_price:
    // 10^6 * 10^(-18) = 10^(-12)
    // 12 decimals used.
    //
    // BUT if we multiply pyth_price by denom_scaled:
    // 10^(-8) * 10^(-18) = 10^(-26)
    // 26 decimals used (overflow) !!!
    let price = usd_price.checked_mul(denom_scaled)?.checked_mul(pyth_price)?;

    if price.is_zero() {
        return Err(InvalidPrice {
            reason: "price is zero".to_string(),
        });
    }

    Ok(price)
}

/// Price feeds represent numbers in a fixed-point format.
/// The same exponent is used for both the price and confidence interval.
/// The integer representation of these values can be computed by multiplying by 10^exponent.
///
/// As an example, imagine Pyth reported the following values for ATOM/USD:
/// expo:  -8
/// conf:  574566
/// price: 1365133270
/// The confidence interval is 574566 * 10^(-8) = $0.00574566, and the price is 1365133270 * 10^(-8) = $13.6513327.
pub fn scale_to_exponent(value: u128, expo: i32) -> ContractResult<Decimal> {
    let target_expo = Uint128::from(10u8).checked_pow(expo.unsigned_abs())?;
    if expo < 0 {
        Ok(Decimal::checked_from_ratio(value, target_expo)?)
    } else {
        let res = Uint128::from(value).checked_mul(target_expo)?;
        Ok(Decimal::from_ratio(res, 1u128))
    }
}

/// Assert availability of usd price source
pub fn assert_usd_price_source<P: PriceSourceChecked<Empty>>(
    deps: &Deps,
    price_sources: &Map<&str, P>,
) -> ContractResult<()> {
    if !price_sources.has(deps.storage, "usd") {
        return Err(ContractError::InvalidPriceSource {
            reason: "missing price source for usd".to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn scale_real_pyth_price() {
        // ATOM
        let uatom_price_in_uusd =
            scale_pyth_price(1035200881u128, -8, 6u8, Decimal::from_str("1000000").unwrap())
                .unwrap();
        assert_eq!(uatom_price_in_uusd, Decimal::from_str("10.35200881").unwrap());

        // ETH
        let ueth_price_in_uusd =
            scale_pyth_price(181598000001u128, -8, 18u8, Decimal::from_str("1000000").unwrap())
                .unwrap();
        assert_eq!(ueth_price_in_uusd, Decimal::from_str("0.00000000181598").unwrap());
    }

    #[test]
    fn scale_pyth_price_if_expo_above_zero() {
        let ueth_price_in_uusd =
            scale_pyth_price(181598000001u128, 8, 18u8, Decimal::from_str("1000000").unwrap())
                .unwrap();
        assert_eq!(ueth_price_in_uusd, Decimal::from_atomics(181598000001u128, 4u32).unwrap());
    }

    #[test]
    fn scale_big_eth_pyth_price() {
        let ueth_price_in_uusd =
            scale_pyth_price(100000098000001u128, -8, 18u8, Decimal::from_str("1000000").unwrap())
                .unwrap();
        assert_eq!(ueth_price_in_uusd, Decimal::from_atomics(100000098000001u128, 20u32).unwrap());
    }

    #[test]
    fn return_error_if_scaled_pyth_price_is_zero() {
        let price_err =
            scale_pyth_price(1u128, -18, 18u8, Decimal::from_str("1000000").unwrap()).unwrap_err();
        assert_eq!(
            price_err,
            ContractError::InvalidPrice {
                reason: "price is zero".to_string()
            }
        );
    }
}
