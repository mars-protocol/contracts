use std::fmt;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{BlockInfo, CosmosMsg, Decimal, Empty, Env, Fraction, QuerierWrapper, Uint128};
use mars_osmosis::helpers::{query_arithmetic_twap_price, query_pool, CommonPoolData};
use mars_swapper_base::{ContractError, ContractResult, Route};
use mars_types::swapper::{EstimateExactInSwapResponse, SwapperRoute};
use osmosis_std::types::osmosis::gamm::v1beta1::MsgSwapExactAmountIn;
pub use osmosis_std::types::osmosis::poolmanager::v1beta1::SwapAmountInRoute as OsmosisSwapAmountInRoute;

use crate::{config::OsmosisConfig, helpers::hashset};

/// 10 min in seconds (Risk Team recommendation)
const TWAP_WINDOW_SIZE_SECONDS: u64 = 600u64;

#[cw_serde]
pub struct OsmosisRoute(pub Vec<SwapAmountInRoute>);

/// SwapAmountInRoute instead of using `osmosis_std::types::osmosis::poolmanager::v1beta1::SwapAmountInRoute`
/// to keep consistency for pool_id representation as u64.
///
/// SwapAmountInRoute from osmosis package uses as_str serializer/deserializer, so it expects pool_id
/// as a String, but JSON schema doesn't correctly represent it.
#[cw_serde]
pub struct SwapAmountInRoute {
    pub pool_id: u64,
    pub token_out_denom: String,
}

impl fmt::Display for OsmosisRoute {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = self
            .0
            .iter()
            .map(|step| format!("{}:{}", step.pool_id, step.token_out_denom))
            .collect::<Vec<_>>()
            .join("|");
        write!(f, "{s}")
    }
}

impl Route<Empty, Empty, OsmosisConfig> for OsmosisRoute {
    fn from(route: SwapperRoute, _config: Option<OsmosisConfig>) -> ContractResult<Self> {
        match route {
            SwapperRoute::Astro(_) => Err(ContractError::InvalidRoute {
                reason: "AstroRoute not supported".to_string(),
            }),
            SwapperRoute::Osmo(route) => {
                let steps: Vec<_> = route
                    .swaps
                    .into_iter()
                    .map(|step| SwapAmountInRoute {
                        pool_id: step.pool_id,
                        token_out_denom: step.to,
                    })
                    .collect();
                Ok(Self(steps))
            }
        }
    }

    // Perform basic validation of the swap steps
    fn validate(
        &self,
        querier: &QuerierWrapper,
        denom_in: &str,
        denom_out: &str,
    ) -> ContractResult<()> {
        let steps = &self.0;

        // there must be at least one step
        if steps.is_empty() {
            return Err(ContractError::InvalidRoute {
                reason: "the route must contain at least one step".to_string(),
            });
        }

        // for each step:
        // - the pool must contain the input and output denoms
        // - the output denom must not be the same as the input denom of a previous step (i.e. the route must not contain a loop)
        let mut prev_denom_out = denom_in;
        let mut seen_denoms = hashset(&[denom_in]);
        for (i, step) in steps.iter().enumerate() {
            let pool = query_pool(querier, step.pool_id)?;
            let pool_denoms = pool.get_pool_denoms();

            if !pool_denoms.contains(&prev_denom_out.to_string()) {
                return Err(ContractError::InvalidRoute {
                    reason: format!(
                        "step {}: pool {} does not contain input denom {}",
                        i + 1,
                        step.pool_id,
                        prev_denom_out
                    ),
                });
            }

            if !pool_denoms.contains(&step.token_out_denom) {
                return Err(ContractError::InvalidRoute {
                    reason: format!(
                        "step {}: pool {} does not contain output denom {}",
                        i + 1,
                        step.pool_id,
                        &step.token_out_denom
                    ),
                });
            }

            if seen_denoms.contains(step.token_out_denom.as_str()) {
                return Err(ContractError::InvalidRoute {
                    reason: format!(
                        "route contains a loop: denom {} seen twice",
                        step.token_out_denom
                    ),
                });
            }

            prev_denom_out = &step.token_out_denom;
            seen_denoms.insert(&step.token_out_denom);
        }

        // the route's final output denom must match the desired output denom
        if prev_denom_out != denom_out {
            return Err(ContractError::InvalidRoute {
                reason: format!(
                    "the route's output denom {prev_denom_out} does not match the desired output {denom_out}",
                ),
            });
        }

        Ok(())
    }

    /// Build a CosmosMsg that swaps given an input denom and amount
    fn build_exact_in_swap_msg(
        &self,
        querier: &QuerierWrapper,
        env: &Env,
        coin_in: &cosmwasm_std::Coin,
        slippage: Decimal,
    ) -> ContractResult<CosmosMsg> {
        let steps = &self.0;

        steps.first().ok_or(ContractError::InvalidRoute {
            reason: "the route must contain at least one step".to_string(),
        })?;

        let out_amount = query_out_amount(querier, &env.block, coin_in, steps)?;
        let min_out_amount = (Decimal::one() - slippage) * out_amount;

        let routes: Vec<_> = steps
            .iter()
            .map(|step| OsmosisSwapAmountInRoute {
                pool_id: step.pool_id,
                token_out_denom: step.token_out_denom.clone(),
            })
            .collect();

        let swap_msg: CosmosMsg = MsgSwapExactAmountIn {
            sender: env.contract.address.to_string(),
            routes,
            token_in: Some(osmosis_std::types::cosmos::base::v1beta1::Coin {
                denom: coin_in.denom.clone(),
                amount: coin_in.amount.to_string(),
            }),
            token_out_min_amount: min_out_amount.to_string(),
        }
        .into();
        Ok(swap_msg)
    }

    fn estimate_exact_in_swap(
        &self,
        querier: &QuerierWrapper,
        env: &Env,
        coin_in: &cosmwasm_std::Coin,
    ) -> ContractResult<EstimateExactInSwapResponse> {
        let out_amount = query_out_amount(querier, &env.block, coin_in, &self.0)?;
        Ok(EstimateExactInSwapResponse {
            amount: out_amount,
        })
    }
}

/// Query how much amount of denom_out we get for denom_in.
///
/// Example calculation:
/// If we want to swap atom to usdc and configured routes are [pool_1 (atom/osmo), pool_69 (osmo/usdc)] (no direct pool of atom/usdc):
/// 1) query pool_1 to get price for atom/osmo
/// 2) query pool_69 to get price for osmo/usdc
/// 3) atom/usdc = (price for atom/osmo) * (price for osmo/usdc)
/// 4) usdc_out_amount = (atom amount) * (price for atom/usdc)
fn query_out_amount(
    querier: &QuerierWrapper,
    block: &BlockInfo,
    coin_in: &cosmwasm_std::Coin,
    steps: &[SwapAmountInRoute],
) -> ContractResult<Uint128> {
    let start_time = block.time.seconds() - TWAP_WINDOW_SIZE_SECONDS;

    let mut price = Decimal::one();
    let mut denom_in = coin_in.denom.clone();
    for step in steps {
        let step_price = query_arithmetic_twap_price(
            querier,
            step.pool_id,
            &denom_in,
            &step.token_out_denom,
            start_time,
        )?;
        price = price.checked_mul(step_price)?;
        denom_in = step.token_out_denom.clone();
    }

    let out_amount =
        coin_in.amount.checked_multiply_ratio(price.numerator(), price.denominator())?;
    Ok(out_amount)
}
