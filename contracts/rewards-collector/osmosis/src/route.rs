use std::fmt;

use cosmwasm_std::{CosmosMsg, Decimal, Empty, Env, QuerierWrapper, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use mars_rewards_collector_base::{ContractError, ContractResult, Route};

use mars_osmosis::helpers::{has_denom, query_estimate_swap_out_amount, query_pool};
use osmosis_std::types::cosmos::base::v1beta1::Coin;
use osmosis_std::types::osmosis::gamm::v1beta1::{MsgSwapExactAmountIn, SwapAmountInRoute};

use crate::helpers::hashset;

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

        let first_swap = steps.first().ok_or(ContractError::InvalidRoute {
            reason: "the route must contain at least one step".to_string(),
        })?;

        let out_amount = query_estimate_swap_out_amount(
            querier,
            &env.contract.address,
            first_swap.pool_id,
            amount,
            steps,
        )?;
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
