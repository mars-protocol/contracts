use std::{cmp::min, str::FromStr};

use cosmwasm_std::{Decimal, Decimal256, Uint128, Uint256};

use crate::ContractResult;

pub fn compute_pcl_lp_price(
    coin0_price: Decimal,
    coin1_price: Decimal,
    coin0_amount: Uint128,
    coin1_amount: Uint128,
    total_shares: Uint128,
    price_scale: Decimal,
    curve_invariant: Decimal256,
) -> ContractResult<Decimal> {
    // Price is the Pyth oracle price of coin0 in terms of coin1
    let price = coin0_price.checked_div(coin1_price)?;

    // xcp represents the virtual value of the pool
    // xcp = curve_invariant / (2 * sqrt(price_scale))
    let xcp = curve_invariant.checked_div(
        Decimal256::from(price_scale).sqrt().checked_mul(Decimal256::from_str("2")?)?,
    )?;

    // Virtual price represents the theoretic price of one share. This virtual price is used as input
    // for the Curve V2 model to determine the modelled lp price.
    // virtual_price = xcp / total_shares
    let virtual_price = xcp.checked_div(Decimal256::from_ratio(total_shares, 1u128))?;

    // LP price according to the model
    // lp_price_model = 2 * virtual_price * sqrt(price)
    let lp_price_model_256 = Decimal256::from_str("2")?
        .checked_mul(virtual_price)?
        .checked_mul(Decimal256::from(price).sqrt())?;
    let lp_price_model = Decimal::try_from(lp_price_model_256)?;

    // Need to use Uint256 because coin0_amount * price + coin1_amount may overflow the 128-bit limit
    // E.g. 1000 BTC + 21000 ETH in a pool, with a price of 65000 and 3000:
    // price = 650 / 0.0000000000003 = 1_267_000_000_000_000
    // 1_000_000_000_000 * 1_267_000_000_000_000 + 21_000_000_000_000_000_000_000 > Uint128::MAX
    let tvl_real =
        Uint256::from(coin0_amount) * Decimal256::from(price) + Uint256::from(coin1_amount);
    let lp_price_real_256 = Decimal256::from_ratio(tvl_real, total_shares);
    let lp_price_real = Decimal::try_from(lp_price_real_256)?;

    Ok(min(lp_price_model, lp_price_real))
}
