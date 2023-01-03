use std::fmt;

use cosmwasm_std::{BlockInfo, CosmosMsg, Decimal, Empty, Env, Fraction, QuerierWrapper, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use mars_rewards_collector_base::{ContractError, ContractResult, Route};

use mars_osmosis::helpers::{has_denom, query_arithmetic_twap_price, query_pool};
use osmosis_std::types::cosmos::base::v1beta1::Coin;
use osmosis_std::types::osmosis::gamm::v1beta1::{MsgSwapExactAmountIn, SwapAmountInRoute};

use crate::helpers::hashset;

/// 10 min in seconds (Risk Team recommendation)
const TWAP_WINDOW_SIZE_SECONDS: u64 = 600u64;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct OsmosisRoute(pub Vec<SwapAmountInRoute>);

impl fmt::Display for OsmosisRoute {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = self
            .0
            .iter()
            .map(|step| format!("{}:{}", step.pool_id, step.token_out_denom))
            .collect::<Vec<_>>()
            .join("|");
        write!(f, "{}", s)
    }
}

impl Route<Empty, Empty> for OsmosisRoute {
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

            if !has_denom(prev_denom_out, &pool.pool_assets) {
                return Err(ContractError::InvalidRoute {
                    reason: format!(
                        "step {}: pool {} does not contain input denom {}",
                        i + 1,
                        step.pool_id,
                        prev_denom_out
                    ),
                });
            }

            if !has_denom(&step.token_out_denom, &pool.pool_assets) {
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
                    "the route's output denom {} does not match the desired output {}",
                    prev_denom_out, denom_out
                ),
            });
        }

        Ok(())
    }

    /// Build a CosmosMsg that swaps given an input denom and amount
    fn build_swap_msg(
        &self,
        env: &Env,
        querier: &QuerierWrapper,
        denom_in: &str,
        amount: Uint128,
        slippage_tolerance: Decimal,
    ) -> ContractResult<CosmosMsg> {
        let steps = &self.0;

        steps.first().ok_or(ContractError::InvalidRoute {
            reason: "the route must contain at least one step".to_string(),
        })?;

        let out_amount = query_out_amount(querier, &env.block, denom_in, amount, steps)?;
        let min_out_amount = (Decimal::one() - slippage_tolerance) * out_amount;

        let swap_msg: CosmosMsg = MsgSwapExactAmountIn {
            sender: env.contract.address.to_string(),
            routes: steps.to_vec(),
            token_in: Some(Coin {
                denom: denom_in.to_string(),
                amount: amount.to_string(),
            }),
            token_out_min_amount: min_out_amount.to_string(),
        }
        .into();
        Ok(swap_msg)
    }
}

/// Query how much amount of denom_out we get for denom_in.
///
/// Example calculation:
/// If we want to swap atom to usdc and configured routes are [pool_1 (atom/osmo), pool_69 (osmo/usdc)] (no direct pool of atom/usdc):
/// 1) query pool_1 to get price for atom/osmo
/// 2) query pool_69 to get price for osmo/usdc
/// 3) atom/usdc = (price for atom/osmo) * (price for osmo/usdc)
/// 4) out_amount = (atom amount) * (price for atom/usdc) = usdc amount
fn query_out_amount(
    querier: &QuerierWrapper,
    block: &BlockInfo,
    denom_in: &str,
    amount: Uint128,
    steps: &[SwapAmountInRoute],
) -> ContractResult<Uint128> {
    let start_time = block.time.seconds() - TWAP_WINDOW_SIZE_SECONDS;

    let mut price = Decimal::one();
    let mut denom_in = denom_in.to_string();
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

    let out_amount = amount.checked_multiply_ratio(price.numerator(), price.denominator())?;
    Ok(out_amount)
}
