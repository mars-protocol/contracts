use std::fmt;

use astroport::{asset::AssetInfo, router::SwapOperation};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, Coin, CosmosMsg, Decimal, Empty, Env, QuerierWrapper, QueryRequest, StdResult,
    Uint128, WasmMsg, WasmQuery,
};
use mars_oracle::PriceResponse;
use mars_swapper::msgs::EstimateExactInSwapResponse;
use mars_swapper_base::{ContractError, ContractResult, Route};

use crate::helpers::hashset;

#[cw_serde]
pub struct AstroportRoute {
    /// The swap operations of the route
    pub operations: Vec<SwapOperation>,
    /// The astroport router contract address
    pub router: String,
    /// The astroport factory contract address
    pub factory: String,
    /// The mars wasm oracle contract address
    pub oracle: String,
}

impl fmt::Display for AstroportRoute {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s =
            self.operations.iter().map(|step| format!("{:?}", step)).collect::<Vec<_>>().join("|");
        write!(f, "{s}")
    }
}

impl AstroportRoute {
    pub fn ask(&self) -> AssetInfo {
        match self.operations.last().unwrap() {
            SwapOperation::NativeSwap {
                offer_denom: _,
                ask_denom,
            } => AssetInfo::NativeToken {
                denom: ask_denom.to_string(),
            },
            SwapOperation::AstroSwap {
                offer_asset_info: _,
                ask_asset_info,
            } => ask_asset_info.clone(),
        }
    }

    pub fn offer(&self) -> AssetInfo {
        match self.operations.first().unwrap() {
            SwapOperation::NativeSwap {
                offer_denom,
                ask_denom: _,
            } => AssetInfo::NativeToken {
                denom: offer_denom.to_string(),
            },
            SwapOperation::AstroSwap {
                offer_asset_info,
                ask_asset_info: _,
            } => offer_asset_info.clone(),
        }
    }

    pub fn query_oracle_price(
        &self,
        querier: &QuerierWrapper,
        denom: AssetInfo,
    ) -> StdResult<Decimal> {
        querier
            .query::<PriceResponse>(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: self.oracle.clone(),
                msg: to_binary(&mars_oracle::QueryMsg::Price {
                    denom: denom.to_string(),
                })?,
            }))
            .map(|res| res.price)
    }

    pub fn estimate_out_amount(
        &self,
        querier: &QuerierWrapper,
        coin_in: &Coin,
    ) -> ContractResult<Uint128> {
        // Query oracle for prices
        let offer_price = self.query_oracle_price(querier, self.offer())?;
        let ask_price = self.query_oracle_price(querier, self.ask())?;

        // Calculate the minimum amount of output tokens to receive
        Ok((ask_price / offer_price) * coin_in.amount)
    }
}

impl Route<Empty, Empty> for AstroportRoute {
    // Perform basic validation of the swap steps
    fn validate(
        &self,
        _querier: &QuerierWrapper,
        denom_in: &str,
        denom_out: &str,
    ) -> ContractResult<()> {
        let steps = &self.operations;

        // there must be at least one step
        if steps.is_empty() {
            return Err(ContractError::InvalidRoute {
                reason: "the route must contain at least one step".to_string(),
            });
        }

        // for each step:
        // - the pool must contain the input and output denoms
        // - the output denom must not be the same as the input denom of a previous step (i.e. the route must not contain a loop)
        let mut prev_denom_out = &AssetInfo::NativeToken {
            denom: denom_in.to_string(),
        };
        let mut seen_denoms = hashset(&[prev_denom_out.clone()]);
        for (_, step) in steps.iter().enumerate() {
            let (offer, ask) = match step {
                SwapOperation::NativeSwap {
                    ..
                } => Err(ContractError::InvalidRoute {
                    reason: "native swap not supported".to_string(),
                }),
                SwapOperation::AstroSwap {
                    offer_asset_info,
                    ask_asset_info,
                } => Ok((offer_asset_info, ask_asset_info)),
            }?;

            if offer != prev_denom_out {
                return Err(ContractError::InvalidRoute {
                    reason: format!(
                        "the route's offer denom {offer} does not match the previous step's ask {prev_denom_out}",
                    ),
                });
            }

            if seen_denoms.contains(&ask) {
                return Err(ContractError::InvalidRoute {
                    reason: format!("route contains a loop: denom {} seen twice", ask.to_string()),
                });
            }

            prev_denom_out = ask;
            seen_denoms.insert(ask.clone());
        }

        // the route's final output denom must match the desired output denom
        if prev_denom_out.to_string() != denom_out {
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
        _env: &Env,
        coin_in: &Coin,
        slippage: Decimal,
    ) -> ContractResult<CosmosMsg> {
        let steps = &self.operations;

        steps.first().ok_or(ContractError::InvalidRoute {
            reason: "the route must contain at least one step".to_string(),
        })?;

        // Query oracle for prices
        let offer_price = self.query_oracle_price(querier, self.offer())?;
        let ask_price = self.query_oracle_price(querier, self.ask())?;

        // Calculate the minimum amount of output tokens to receive
        let out_amount = (ask_price / offer_price) * coin_in.amount;
        let minimum_receive = Some((Decimal::one() - slippage) * out_amount);

        let swap_msg: CosmosMsg = WasmMsg::Execute {
            contract_addr: self.router.clone(),
            msg: to_binary(&astroport::router::ExecuteMsg::ExecuteSwapOperations {
                operations: self.operations.clone(),
                minimum_receive,
                to: None,
                max_spread: None,
            })?,
            funds: vec![coin_in.clone()],
        }
        .into();
        Ok(swap_msg)
    }

    fn estimate_exact_in_swap(
        &self,
        querier: &QuerierWrapper,
        _env: &Env,
        coin_in: &Coin,
    ) -> ContractResult<EstimateExactInSwapResponse> {
        let out_amount = self.estimate_out_amount(querier, coin_in)?;
        Ok(EstimateExactInSwapResponse {
            amount: out_amount,
        })
    }
}

pub trait Offer {
    fn offer(&self) -> AssetInfo;
}
pub trait Ask {
    fn ask(&self) -> AssetInfo;
}

impl Offer for SwapOperation {
    fn offer(&self) -> AssetInfo {
        match self {
            SwapOperation::NativeSwap {
                offer_denom,
                ask_denom: _,
            } => AssetInfo::NativeToken {
                denom: offer_denom.to_string(),
            },
            SwapOperation::AstroSwap {
                offer_asset_info,
                ask_asset_info: _,
            } => offer_asset_info.clone(),
        }
    }
}

impl Ask for SwapOperation {
    fn ask(&self) -> AssetInfo {
        match self {
            SwapOperation::NativeSwap {
                offer_denom: _,
                ask_denom,
            } => AssetInfo::NativeToken {
                denom: ask_denom.to_string(),
            },
            SwapOperation::AstroSwap {
                offer_asset_info: _,
                ask_asset_info,
            } => ask_asset_info.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swap_operation_ask_and_offer() {
        let op = SwapOperation::NativeSwap {
            offer_denom: "uosmo".to_string(),
            ask_denom: "uusd".to_string(),
        };

        assert_eq!(
            op.ask(),
            AssetInfo::NativeToken {
                denom: "uusd".to_string()
            }
        );
        assert_eq!(
            op.offer(),
            AssetInfo::NativeToken {
                denom: "uosmo".to_string()
            }
        );

        let op = SwapOperation::AstroSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: "uosmo".to_string(),
            },
            ask_asset_info: AssetInfo::NativeToken {
                denom: "uatom".to_string(),
            },
        };

        assert_eq!(
            op.ask(),
            AssetInfo::NativeToken {
                denom: "uatom".to_string()
            }
        );
        assert_eq!(
            op.offer(),
            AssetInfo::NativeToken {
                denom: "uosmo".to_string()
            }
        );
    }

    #[test]
    fn test_astroport_route_offer_and_ask() {
        let route = AstroportRoute {
            operations: vec![
                SwapOperation::NativeSwap {
                    offer_denom: "uosmo".to_string(),
                    ask_denom: "uusd".to_string(),
                },
                SwapOperation::NativeSwap {
                    offer_denom: "uusd".to_string(),
                    ask_denom: "uatom".to_string(),
                },
            ],
            router: "router".to_string(),
            oracle: "oracle".to_string(),
            factory: "factory".to_string(),
        };

        assert_eq!(
            route.ask(),
            AssetInfo::NativeToken {
                denom: "uatom".to_string()
            }
        );
        assert_eq!(
            route.offer(),
            AssetInfo::NativeToken {
                denom: "uosmo".to_string()
            }
        );

        let route = AstroportRoute {
            operations: vec![
                SwapOperation::NativeSwap {
                    offer_denom: "uosmo".to_string(),
                    ask_denom: "uusd".to_string(),
                },
                SwapOperation::AstroSwap {
                    offer_asset_info: AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    ask_asset_info: AssetInfo::NativeToken {
                        denom: "uatom".to_string(),
                    },
                },
            ],
            router: "router".to_string(),
            oracle: "oracle".to_string(),
            factory: "factory".to_string(),
        };

        assert_eq!(
            route.ask(),
            AssetInfo::NativeToken {
                denom: "uatom".to_string()
            }
        );
        assert_eq!(
            route.offer(),
            AssetInfo::NativeToken {
                denom: "uosmo".to_string()
            }
        );
    }
}
