use cosmwasm_std::{Coin, Decimal, Decimal256, Deps, Empty, Env, Uint128};
use cw_storage_plus::Map;
use mars_oracle_base::{ContractResult, PriceSourceChecked};
use mars_types::oracle::{ActionKind, Config};

use crate::{
    helpers::{compute_pcl_lp_price, query_token_precision},
    state::ASTROPORT_FACTORY,
};

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
        &coin0_price,
        &coin1_price,
        coin0.amount,
        coin1.amount,
        total_shares,
        price_scale,
        curve_invariant,
    )
}
