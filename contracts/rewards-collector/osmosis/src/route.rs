use std::fmt;

use cosmwasm_std::{
    Addr, CosmosMsg, Decimal, Env, QuerierWrapper, QueryRequest, StdError, StdResult, Uint128,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use mars_rewards_collector_base::{ContractError, ContractResult, Route};

use osmo_bindings::{
    OsmosisMsg, OsmosisQuery, PoolStateResponse, Step, Swap, SwapAmount, SwapAmountWithLimit,
    SwapResponse,
};

use crate::helpers::hashset;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OsmosisRoute(pub Vec<Step>);

impl fmt::Display for OsmosisRoute {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = self
            .0
            .iter()
            .map(|step| format!("{}:{}", step.pool_id, step.denom_out))
            .collect::<Vec<_>>()
            .join("|");
        write!(f, "{}", s)
    }
}

impl Route<OsmosisMsg, OsmosisQuery> for OsmosisRoute {
    // Perform basic validation of the swap steps
    fn validate(
        &self,
        querier: &QuerierWrapper<OsmosisQuery>,
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
            let pool_state: PoolStateResponse =
                querier.query(&QueryRequest::Custom(OsmosisQuery::PoolState {
                    id: step.pool_id,
                }))?;

            if !pool_state.has_denom(prev_denom_out) {
                return Err(ContractError::InvalidRoute {
                    reason: format!(
                        "step {}: pool {} does not contain input denom {}",
                        i + 1,
                        step.pool_id,
                        prev_denom_out
                    ),
                });
            }

            if !pool_state.has_denom(&step.denom_out) {
                return Err(ContractError::InvalidRoute {
                    reason: format!(
                        "step {}: pool {} does not contain output denom {}",
                        i + 1,
                        step.pool_id,
                        &step.denom_out
                    ),
                });
            }

            if seen_denoms.contains(step.denom_out.as_str()) {
                return Err(ContractError::InvalidRoute {
                    reason: format!("route contains a loop: denom {} seen twice", step.denom_out),
                });
            }

            prev_denom_out = &step.denom_out;
            seen_denoms.insert(&step.denom_out);
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
        querier: &QuerierWrapper<OsmosisQuery>,
        denom_in: &str,
        amount: Uint128,
        slippage_tolerance: Decimal,
    ) -> ContractResult<CosmosMsg<OsmosisMsg>> {
        let steps = &self.0;

        let first_swap = steps
            .first()
            .map(|step| Swap::new(step.pool_id, denom_in, &step.denom_out))
            .ok_or(ContractError::InvalidRoute {
                reason: "the route must contain at least one step".to_string(),
            })?;

        let out_amount =
            estimate_swap_out_amount(querier, &env.contract.address, &first_swap, steps, amount)?;
        let min_out_amount = (Decimal::one() - slippage_tolerance) * out_amount;

        Ok(CosmosMsg::Custom(OsmosisMsg::Swap {
            first: first_swap,
            route: steps[1..].to_vec(),
            amount: SwapAmountWithLimit::ExactIn {
                input: amount,
                min_output: min_out_amount,
            },
        }))
    }
}

fn estimate_swap_out_amount(
    querier: &QuerierWrapper<OsmosisQuery>,
    contract_addr: &Addr,
    first_swap: &Swap,
    steps: &[Step],
    amount: Uint128,
) -> StdResult<Uint128> {
    let query = OsmosisQuery::EstimateSwap {
        sender: contract_addr.into(),
        first: first_swap.clone(),
        route: steps[1..].to_vec(),
        amount: SwapAmount::In(amount),
    };

    let res: SwapResponse = querier.query(&QueryRequest::Custom(query))?;
    match res.amount {
        SwapAmount::Out(out_amount) => Ok(out_amount),
        _ => Err(StdError::GenericErr {
            msg: "Can't be `SwapAmount::In if query is built with this type".to_string(),
        }),
    }
}