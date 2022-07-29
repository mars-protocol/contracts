use cosmwasm_std::{CosmosMsg, QuerierWrapper, QueryRequest, Uint128};
use osmo_bindings::{OsmosisMsg, OsmosisQuery, PoolStateResponse, Step, Swap, SwapAmountWithLimit};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{error::ContractResult, ContractError};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SwapInstruction(pub Vec<Step>);

impl SwapInstruction {
    // Perform basic validation of the swap steps
    pub fn validate(
        &self,
        querier: &QuerierWrapper<OsmosisQuery>,
        denom_in: &str,
        denom_out: &str,
    ) -> ContractResult<()> {
        let steps = self.steps();

        // there must be at least one step
        if steps.len() == 0 {
            return Err(ContractError::InvalidSwapRoute {
                steps: steps.to_vec(),
                reason: "the route must contain at least one step".to_string(),
            });
        }

        // for each step, the pool must contain the input and output denoms
        let mut prev_denom_out = denom_in;
        for (i, step) in steps.iter().enumerate() {
            let pool_state: PoolStateResponse =
                querier.query(&QueryRequest::Custom(OsmosisQuery::PoolState {
                    id: step.pool_id,
                }))?;

            if !pool_state.has_denom(prev_denom_out) {
                return Err(ContractError::InvalidSwapRoute {
                    steps: steps.to_vec(),
                    reason: format!(
                        "step {}: pool {} does not contain input denom {}",
                        i, step.pool_id, prev_denom_out
                    ),
                });
            }

            if !pool_state.has_denom(&step.denom_out) {
                return Err(ContractError::InvalidSwapRoute {
                    steps: steps.to_vec(),
                    reason: format!(
                        "step {}: pool {} does not contain output denom {}",
                        i, step.pool_id, &step.denom_out
                    ),
                });
            }

            prev_denom_out = &step.denom_out;
        }

        // the final output denom must not be the same from the initial input denom
        if denom_in == prev_denom_out {
            return Err(ContractError::InvalidSwapRoute {
                steps: steps.to_vec(),
                reason: format!("input and output denom cannot both be {}", denom_in),
            });
        }

        // the route's output denom must match the desired output denom
        if prev_denom_out != denom_out {
            return Err(ContractError::InvalidSwapRoute {
                steps: steps.to_vec(),
                reason: format!(
                    "the route output denom {} does not match the desired output {}",
                    prev_denom_out, denom_out
                ),
            });
        }

        Ok(())
    }

    /// Return a referenece to the swap steps
    pub fn steps(&self) -> &[Step] {
        &self.0
    }

    /// Build a CosmosMsg that swaps given an input denom and amount
    pub fn build_swap_msg(
        &self,
        denom_in: &str,
        amount: Uint128,
    ) -> ContractResult<CosmosMsg<OsmosisMsg>> {
        let steps = self.steps();

        let first_swap = steps
            .first()
            .map(|step| Swap::new(step.pool_id, denom_in, &step.denom_out))
            .ok_or(ContractError::InvalidSwapRoute {
                steps: steps.to_vec(),
                reason: "the route must contain at least one step".to_string(),
            })?;

        Ok(CosmosMsg::Custom(OsmosisMsg::Swap {
            first: first_swap,
            route: steps[1..].to_vec(),
            amount: SwapAmountWithLimit::ExactIn {
                input: amount,
                min_output: Uint128::zero(), // TODO: implement slippage tolerance
            },
        }))
    }
}
