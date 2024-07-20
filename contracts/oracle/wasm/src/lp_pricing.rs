use std::{
    cmp::{max, min},
    str::FromStr,
};

use cosmwasm_std::{Coin, Decimal, Decimal256, Deps, Empty, Env, Uint128};
use cw_storage_plus::Map;
use mars_oracle_base::{ContractResult, PriceSourceChecked};
use mars_types::oracle::{ActionKind, Config};

use crate::{helpers::query_token_precision, state::ASTROPORT_FACTORY};

#[allow(clippy::too_many_arguments)]
pub fn query_pcl_lp_price<P: PriceSourceChecked<Empty>>(
    deps: &Deps,
    env: &Env,
    config: &Config,
    price_sources: &Map<&str, P>,
    kind: ActionKind,
    coin0: Coin,
    coin1: Coin,
    total_shares: Uint128,
    price_scale: Decimal,
    curve_invariant: Decimal256,
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

    let astroport_factory = ASTROPORT_FACTORY.load(deps.storage)?;
    let coin0_decimals = query_token_precision(&deps.querier, &astroport_factory, &coin0.denom)?;
    let coin1_decimals = query_token_precision(&deps.querier, &astroport_factory, &coin1.denom)?;

    compute_pcl_lp_price(
        coin0_price,
        coin1_price,
        coin0_decimals,
        coin1_decimals,
        total_shares,
        price_scale,
        curve_invariant,
    )
}

pub fn compute_pcl_lp_price(
    coin0_price: Decimal,
    coin1_price: Decimal,
    coin0_decimals: u8,
    coin1_decimals: u8,
    total_shares: Uint128,
    price_scale: Decimal,
    curve_invariant: Decimal256,
) -> ContractResult<Decimal> {
    // xcp represents the virtual value of the pool
    // xcp = curve_invariant / (2 * sqrt(price_scale))
    let xcp = curve_invariant.checked_div(
        Decimal256::from(price_scale).sqrt().checked_mul(Decimal256::from_str("2")?)?,
    )?;

    // Virtual price represents the theoretic price of one share. This virtual price is used as input
    // for the Curve V2 model to determine the modelled lp price.
    // virtual_price = xcp / total_shares
    let virtual_price = xcp.checked_div(Decimal256::from_ratio(total_shares, 1u128))?;

    // The curve_invariant is calculated with amounts scaled by Astroport, e.g. 1e18 ueth is stored as 1 eth.
    // So we need to scale the prices accordingly, so that they represent the price of 1 whole unit.
    let coin0_price_scaled =
        Decimal256::from(coin0_price) * Decimal256::from_str("10")?.pow(coin0_decimals as u32);
    let coin1_price_scaled =
        Decimal256::from(coin1_price) * Decimal256::from_str("10")?.pow(coin1_decimals as u32);

    // LP price according to the model
    // lp_price_model = 2 * virtual_price * sqrt(coin0_price * coin1_price)
    let lp_price_model_256 = Decimal256::from_str("2")?
        .checked_mul(virtual_price)?
        .checked_mul(coin0_price_scaled.checked_mul(coin1_price_scaled)?.sqrt())?;
    let lp_price_model = Decimal::try_from(lp_price_model_256)?;

    Ok(lp_price_model)
}

#[allow(clippy::too_many_arguments)]
pub fn query_stable_swap_lp_price<P: PriceSourceChecked<Empty>>(
    deps: &Deps,
    env: &Env,
    config: &Config,
    price_sources: &Map<&str, P>,
    kind: ActionKind,
    coin0: Coin,
    coin1: Coin,
    total_shares: Uint128,
    curve_invariant: Uint128,
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

    let astroport_factory = ASTROPORT_FACTORY.load(deps.storage)?;
    let coin0_decimals = query_token_precision(&deps.querier, &astroport_factory, &coin0.denom)?;
    let coin1_decimals = query_token_precision(&deps.querier, &astroport_factory, &coin1.denom)?;

    compute_ss_lp_price(
        coin0_price,
        coin1_price,
        coin0_decimals,
        coin1_decimals,
        total_shares,
        curve_invariant,
    )
}

pub fn compute_ss_lp_price(
    coin0_price: Decimal,
    coin1_price: Decimal,
    coin0_decimals: u8,
    coin1_decimals: u8,
    total_shares: Uint128,
    curve_invariant: Uint128,
) -> ContractResult<Decimal> {
    // StableSwap pool lp price calculation:
    //    virtual_price = curve_invariant / total_shares
    //    lp_price = virtual_price * min(coin0_price, coin1_price)
    let virtual_price = Decimal::checked_from_ratio(curve_invariant, total_shares)?;

    // The curve_invariant takes on the precision of the asset with the greatest precision.
    // https://github.dev/astroport-fi/astroport-core/blob/a0a71af801be3f72c64b81f798e1b0805cf0f594/contracts/pair_stable/src/contract.rs#L91
    // E.g. a stable pool with asset1_decimals = 6 and asset2_decimals = 18 will have a
    // curve_invariant with 18 decimals.
    let greatest_precision = max(coin0_decimals, coin1_decimals);

    // 1 BTC = 65 000 USD
    //
    // We have to provide prices for the whole unit:
    // 10^8 ubtc = 65 * 10^9 uusd
    // 1 ubtc = 65 * 10^9 / 10^8 uusd
    // 1 ubtc = 650 uusd
    // If we multiply by BTC decimals:
    // 1 BTC = 650 * 10^8 uusd
    // 1 BTC = 65 * 10^9 uusd
    let coin0_price_scaled =
        Decimal256::from(coin0_price) * Decimal256::from_str("10")?.pow(coin0_decimals as u32);
    let coin1_price_scaled =
        Decimal256::from(coin1_price) * Decimal256::from_str("10")?.pow(coin1_decimals as u32);

    let lp_price_256_scaled =
        Decimal256::from(virtual_price).checked_mul(min(coin0_price_scaled, coin1_price_scaled))?;

    // https://github.dev/astroport-fi/astroport-core/blob/a0a71af801be3f72c64b81f798e1b0805cf0f594/packages/astroport/src/asset.rs#L836
    //
    // The price needs to be adjusted with the greatest_precision to denominate correctly in
    // uusd per share. Since the prices are already scaled to accompany the asset precisions,
    // we can ignore the `self.decimals()` (in the linked Astroport code) of the calculation and
    // just divide by the greatest precision.
    let lp_price_256 = lp_price_256_scaled
        .checked_div(Decimal256::from_str("10")?.pow(greatest_precision as u32))?;
    let lp_price = Decimal::try_from(lp_price_256)?;

    Ok(lp_price)
}
