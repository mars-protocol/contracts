use cosmwasm_std::{CosmosMsg, QuerierWrapper, QueryRequest, Uint128};
use osmo_bindings::{OsmosisMsg, OsmosisQuery, PoolStateResponse, Step, Swap, SwapAmountWithLimit};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{ContractError, ContractResult};
use crate::helpers::hashset;

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
                reason: "the route must contain at least one step".to_string(),
            });
        }

        // for each step:
        // - the pool must contain the input and output denoms
        // - the output denom must not be the same as the input denom of a previous step (i.e. the route must not contain a loop)
        let mut prev_denom_out = denom_in;
        let mut seen_denoms = hashset(&[denom_in]);
        for (i, step) in steps.iter().enumerate() {
            let pool_state: PoolStateResponse =
                querier.query(&QueryRequest::Custom(OsmosisQuery::PoolState {
                    id: step.pool_id,
                }))?;

            if !pool_state.has_denom(prev_denom_out) {
                return Err(ContractError::InvalidSwapRoute {
                    reason: format!(
                        "step {}: pool {} does not contain input denom {}",
                        i + 1,
                        step.pool_id,
                        prev_denom_out
                    ),
                });
            }

            if !pool_state.has_denom(&step.denom_out) {
                return Err(ContractError::InvalidSwapRoute {
                    reason: format!(
                        "step {}: pool {} does not contain output denom {}",
                        i + 1,
                        step.pool_id,
                        &step.denom_out
                    ),
                });
            }

            if seen_denoms.contains(step.denom_out.as_str()) {
                return Err(ContractError::InvalidSwapRoute {
                    reason: format!("route contains a loop: denom {} seen twice", step.denom_out),
                });
            }

            prev_denom_out = &step.denom_out;
            seen_denoms.insert(&step.denom_out);
        }

        // the route's final output denom must match the desired output denom
        if prev_denom_out != denom_out {
            return Err(ContractError::InvalidSwapRoute {
                reason: format!(
                    "the route's output denom {} does not match the desired output {}",
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
