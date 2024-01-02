use std::fmt;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, CosmosMsg, Decimal, Empty, Env, QuerierWrapper};

use self::{
    astro_route::AstroportRoute,
    error::{SwapperError, SwapperResult},
    osmo_route::OsmosisRoute,
    traits::Route,
};
use crate::swapper::EstimateExactInSwapResponse;

mod astro_route;
mod error;
mod helpers;
mod osmo_route;
mod traits;

// Max allowed slippage percentage for swap
const MAX_SLIPPAGE_PERCENTAGE: u64 = 10;

#[cw_serde]
pub enum SwapperRoute {
    Astro(AstroportRoute),
    Osmo(OsmosisRoute),
}

impl SwapperRoute {
    pub fn swap_msg(
        &self,
        querier: &QuerierWrapper,
        env: &Env,
        coin_in: Coin,
        denom_out: String,
        slippage: Decimal,
    ) -> SwapperResult<CosmosMsg> {
        match self {
            SwapperRoute::Astro(route) => {
                route.validate(querier, &coin_in.denom, &denom_out)?;
                route.build_exact_in_swap_msg(querier, env, &coin_in, slippage)
            }
            SwapperRoute::Osmo(route) => {
                route.validate(querier, &coin_in.denom, &denom_out)?;
                route.build_exact_in_swap_msg(querier, env, &coin_in, slippage)
            }
        }
    }
}

impl fmt::Display for SwapperRoute {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SwapperRoute::Astro(route) => write!(f, "{}", route),
            SwapperRoute::Osmo(route) => write!(f, "{}", route),
        }
    }
}

impl Route<Empty, Empty> for SwapperRoute {
    fn validate(
        &self,
        querier: &QuerierWrapper,
        denom_in: &str,
        denom_out: &str,
    ) -> SwapperResult<()> {
        match self {
            SwapperRoute::Astro(route) => route.validate(querier, denom_in, denom_out),
            SwapperRoute::Osmo(route) => route.validate(querier, denom_in, denom_out),
        }
    }

    fn build_exact_in_swap_msg(
        &self,
        querier: &QuerierWrapper,
        env: &Env,
        coin_in: &Coin,
        slippage: Decimal,
    ) -> SwapperResult<CosmosMsg> {
        let max_slippage = Decimal::percent(MAX_SLIPPAGE_PERCENTAGE);
        if slippage > max_slippage {
            return Err(SwapperError::MaxSlippageExceeded {
                max_slippage,
                slippage,
            });
        }

        match self {
            SwapperRoute::Astro(route) => {
                route.build_exact_in_swap_msg(querier, env, coin_in, slippage)
            }
            SwapperRoute::Osmo(route) => {
                route.build_exact_in_swap_msg(querier, env, coin_in, slippage)
            }
        }
    }

    fn estimate_exact_in_swap(
        &self,
        querier: &QuerierWrapper,
        env: &Env,
        coin_in: &Coin,
    ) -> SwapperResult<EstimateExactInSwapResponse> {
        match self {
            SwapperRoute::Astro(route) => route.estimate_exact_in_swap(querier, env, coin_in),
            SwapperRoute::Osmo(route) => route.estimate_exact_in_swap(querier, env, coin_in),
        }
    }
}
