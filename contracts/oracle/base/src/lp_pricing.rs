use cosmwasm_std::{Coin, Decimal, Decimal256, Deps, Empty, Env, Isqrt, Uint128, Uint256};
use cw_storage_plus::Map;
use mars_types::oracle::{ActionKind, Config};

use crate::{ContractResult, PriceSourceChecked};

/// The calculation of the value of liquidity token, see: https://blog.alphafinance.io/fair-lp-token-pricing/.
/// This formulation avoids a potential sandwich attack that distorts asset prices by a flashloan.
///
/// NOTE: Price sources must exist for both assets in the pool.
#[allow(clippy::too_many_arguments)]
pub fn query_xyk_lp_price<P: PriceSourceChecked<Empty>>(
    deps: &Deps,
    env: &Env,
    config: &Config,
    price_sources: &Map<&str, P>,
    kind: ActionKind,
    coin0: Coin,
    coin1: Coin,
    total_shares: Uint128,
) -> ContractResult<Decimal> {
    let coin0_price = price_sources.load(deps.storage, &coin0.denom)?.query_price(
        deps,
        env,
        &coin0.denom,
        config,
        price_sources,
        kind.clone(),
    )?;
    let coin1_price = price_sources.load(deps.storage, &coin1.denom)?.query_price(
        deps,
        env,
        &coin1.denom,
        config,
        price_sources,
        kind,
    )?;

    let coin0_value = Uint256::from_uint128(coin0.amount) * Decimal256::from(coin0_price);
    let coin1_value = Uint256::from_uint128(coin1.amount) * Decimal256::from(coin1_price);

    // We need to use Uint256, because Uint128 * Uint128 may overflow the 128-bit limit
    let pool_value_u256 = Uint256::from(2u8) * (coin0_value * coin1_value).isqrt();
    let pool_value_u128 = Uint128::try_from(pool_value_u256)?;

    Ok(Decimal::from_ratio(pool_value_u128, total_shares))
}
