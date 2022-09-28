use std::fmt;
use std::ops::Sub;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, Addr, Coin, CosmosMsg, Decimal, Deps, Env, QuerierWrapper, QueryRequest, WasmQuery,
};
use osmo_bindings::{
    EstimatePriceResponse as OsmoResponse, OsmosisMsg, OsmosisQuery, PoolStateResponse, Step, Swap,
    SwapAmount, SwapAmountWithLimit,
};
use rover::adapters::swap::{EstimateExactInSwapResponse, QueryMsg};
use rover::traits::IntoDecimal;
use swapper_base::{ContractError, ContractResult, Route};

use crate::helpers::{hashset, GetValue, IntoUint128};

#[cw_serde]
pub struct OsmosisRoute {
    pub steps: Vec<Step>,
}

impl fmt::Display for OsmosisRoute {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = self
            .steps
            .iter()
            .map(|step| format!("{}:{}", step.pool_id, step.denom_out))
            .collect::<Vec<_>>()
            .join("|");
        write!(f, "{}", s)
    }
}

impl Route<OsmosisMsg, OsmosisQuery> for OsmosisRoute {
    // Perform basic validation of the swapper steps
    fn validate(
        &self,
        querier: &QuerierWrapper<OsmosisQuery>,
        denom_in: &str,
        denom_out: &str,
    ) -> ContractResult<()> {
        // there must be at least one step
        if self.steps.is_empty() {
            return Err(ContractError::InvalidRoute {
                reason: "the route must contain at least one step".to_string(),
            });
        }

        // for each step:
        // - the pool must contain the input and output denoms
        // - the output denom must not be the same as the input denom of a previous step (i.e. the route must not contain a loop)
        let mut prev_denom_out = denom_in;
        let mut seen_denoms = hashset(&[denom_in]);
        for (i, step) in self.steps.iter().enumerate() {
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
    fn build_exact_in_swap_msg(
        &self,
        querier: &QuerierWrapper<OsmosisQuery>,
        contract_addr: Addr,
        coin_in: &Coin,
        slippage: Decimal,
    ) -> ContractResult<CosmosMsg<OsmosisMsg>> {
        let last_step = self.steps.last().unwrap(); // Safe as contract guarantees at least one step
        let res: EstimateExactInSwapResponse =
            querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: contract_addr.to_string(),
                msg: to_binary(&QueryMsg::EstimateExactInSwap {
                    coin_in: coin_in.clone(),
                    denom_out: last_step.denom_out.clone(),
                })?,
            }))?;

        let swap_amount_with_slippage = SwapAmountWithLimit::ExactIn {
            input: coin_in.amount,
            min_output: Decimal::one()
                .sub(slippage)
                .checked_mul(res.amount.to_dec()?)?
                .uint128(),
        };

        let first_swap = self
            .steps
            .first()
            .map(|step| Swap::new(step.pool_id, coin_in.denom.clone(), &step.denom_out))
            .unwrap(); // Safe as contract guarantees at least one step

        Ok(CosmosMsg::Custom(OsmosisMsg::Swap {
            first: first_swap,
            route: self.steps[1..].to_vec(),
            amount: swap_amount_with_slippage,
        }))
    }

    fn estimate_exact_in_swap(
        &self,
        deps: Deps<OsmosisQuery>,
        env: Env,
        coin_in: Coin,
    ) -> ContractResult<EstimateExactInSwapResponse> {
        let first_step = self.steps.first().unwrap(); // Safe as contract guarantees at least one step
        let query = OsmosisQuery::EstimateSwap {
            sender: env.contract.address.to_string(),
            first: Swap {
                pool_id: first_step.pool_id,
                denom_in: coin_in.denom,
                denom_out: first_step.denom_out.clone(),
            },
            route: self.steps[1..].to_vec(),
            amount: SwapAmount::In(coin_in.amount),
        };

        let res: OsmoResponse = deps.querier.query(&QueryRequest::Custom(query))?;
        Ok(EstimateExactInSwapResponse {
            amount: res.amount.value(),
        })
    }
}
