use cosmwasm_std::{Addr, Decimal, Deps, Empty, Env, StdError, Uint128};
use cw_storage_plus::Map;
use mars_red_bank_types::oracle::Config;
use pyth_sdk_cw::query_price_feed;
pub use pyth_sdk_cw::PriceIdentifier;

use super::*;
use crate::error::ContractError::InvalidPrice;

pub fn query_pyth_price<P: PriceSourceChecked<Empty>>(
    deps: &Deps,
    env: &Env,
    contract_addr: Addr,
    price_feed_id: PriceIdentifier,
    max_staleness: u64,
    denom_decimals: u8,
    config: &Config,
    price_sources: &Map<&str, P>,
) -> ContractResult<Decimal> {
    // Use current price source for USD to check how much 1 USD is worth in base_denom
    let usd_price = price_sources
        .load(deps.storage, "usd")
        .map_err(|_| StdError::generic_err("Price source not found for denom 'usd'"))?
        .query_price(deps, env, "usd", config, price_sources)?;

    let current_time = env.block.time.seconds();

    let price_feed_response = query_price_feed(&deps.querier, contract_addr, price_feed_id)?;
    let price_feed = price_feed_response.price_feed;

    // Check if the current price is not too old
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

/// Price feeds represent numbers in a fixed-point format.
/// The same exponent is used for both the price and confidence interval.
/// The integer representation of these values can be computed by multiplying by 10^exponent.
///
/// As an example, imagine Pyth reported the following values for ATOM/USD:
/// expo:  -8
/// conf:  574566
/// price: 1365133270
/// The confidence interval is 574566 * 10^(-8) = $0.00574566, and the price is 1365133270 * 10^(-8) = $13.6513327.
///
/// Moreover, we have to represent the price for utoken in base_denom.
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
    let target_expo = Uint128::from(10u8).checked_pow(expo.unsigned_abs())?;
    let pyth_price = if expo < 0 {
        Decimal::checked_from_ratio(value, target_expo)?
    } else {
        let res = Uint128::from(value).checked_mul(target_expo)?;
        Decimal::from_ratio(res, 1u128)
    };

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

    Ok(price)
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
}
