use cosmwasm_std::{Addr, Decimal, Deps, Env, Uint128};
use pyth_sdk_cw::query_price_feed;
pub use pyth_sdk_cw::PriceIdentifier;

use super::*;
use crate::error::ContractError::InvalidPrice;

pub fn query_pyth_price(
    deps: &Deps,
    env: &Env,
    contract_addr: Addr,
    price_feed_id: PriceIdentifier,
    max_staleness: u64,
) -> ContractResult<Decimal> {
    let current_time = env.block.time.seconds();

    let price_feed_response = query_price_feed(&deps.querier, contract_addr, price_feed_id)?;
    let price_feed = price_feed_response.price_feed;

    // Get the current price and confidence interval from the price feed
    let current_price = price_feed.get_price_unchecked();

    // Check if the current price is not too old
    if (current_time - current_price.publish_time as u64) > max_staleness {
        return Err(InvalidPrice {
            reason: format!(
                "current price publish time is too old/stale. published: {}, now: {}",
                current_price.publish_time, current_time
            ),
        });
    }

    // Check if the current price is > 0
    if current_price.price <= 0 {
        return Err(InvalidPrice {
            reason: "price can't be <= 0".to_string(),
        });
    }

    let current_price_dec = scale_to_exponent(current_price.price as u128, current_price.expo)?;

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
pub fn scale_to_exponent(value: u128, expo: i32) -> ContractResult<Decimal> {
    let target_expo = Uint128::from(10u8).checked_pow(expo.unsigned_abs())?;
    if expo < 0 {
        Ok(Decimal::checked_from_ratio(value, target_expo)?)
    } else {
        let res = Uint128::from(value).checked_mul(target_expo)?;
        Ok(Decimal::from_ratio(res, 1u128))
    }
}
