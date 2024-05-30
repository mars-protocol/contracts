use std::{cmp::min, str::FromStr};

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
        coin0_decimals,
        coin1_decimals,
        coin0_price,
        coin1_price,
        coin0.amount,
        coin1.amount,
        total_shares,
        price_scale,
        curve_invariant,
    )
}

pub fn compute_pcl_lp_price(
    coin0_decimals: u8,
    coin1_decimals: u8,
    coin0_price: Decimal,
    coin1_price: Decimal,
    coin0_amount: Uint128,
    coin1_amount: Uint128,
    total_shares: Uint128,
    price_scale: Decimal,
    curve_invariant: Decimal256,
) -> ContractResult<Decimal> {
    let lp_price_model = compute_pcl_lp_price_model(
        coin0_price,
        coin1_price,
        coin0_decimals,
        coin1_decimals,
        total_shares,
        price_scale,
        curve_invariant,
    )?;

    let lp_price_real = compute_pcl_lp_price_real(
        coin0_amount,
        coin1_amount,
        coin0_price,
        coin1_price,
        total_shares,
    )?;

    let pcl_lp_price = min(lp_price_model, lp_price_real);

    Ok(pcl_lp_price)
}

pub fn compute_pcl_lp_price_model(
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
        Decimal256::from(coin0_price) * Decimal256::from_str("10")?.pow(u32::from(coin0_decimals));
    let coin1_price_scaled =
        Decimal256::from(coin1_price) * Decimal256::from_str("10")?.pow(u32::from(coin1_decimals));

    // LP price according to the model
    // lp_price_model = 2 * virtual_price * sqrt(coin0_price * coin1_price)
    let lp_price_model_256 = Decimal256::from_str("2")?
        .checked_mul(virtual_price)?
        .checked_mul(coin0_price_scaled.checked_mul(coin1_price_scaled)?.sqrt())?;
    let lp_price_model = Decimal::try_from(lp_price_model_256)?;

    Ok(lp_price_model)
}

pub fn compute_pcl_lp_price_real(
    coin0_amount: Uint128,
    coin1_amount: Uint128,
    coin0_price: Decimal,
    coin1_price: Decimal,
    total_shares: Uint128,
) -> ContractResult<Decimal> {
    let tvl_real = coin0_amount.checked_mul_floor(coin0_price)?
        + coin1_amount.checked_mul_floor(coin1_price)?;
    let lp_price_real = Decimal::checked_from_ratio(tvl_real, total_shares)?;
    Ok(lp_price_real)
}
