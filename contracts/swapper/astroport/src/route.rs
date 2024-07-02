use std::{fmt, str::FromStr};

use astroport_v5::{asset::AssetInfo, pair::MAX_ALLOWED_SLIPPAGE, router::SwapOperation};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, Coin, CosmosMsg, Decimal, Empty, Env, QuerierWrapper, QueryRequest, StdError,
    StdResult, Uint128, WasmMsg, WasmQuery,
};
use mars_swapper_base::{ContractError, ContractResult, Route};
use mars_types::{
    oracle::PriceResponse,
    swapper::{EstimateExactInSwapResponse, SwapperRoute},
};

use crate::{config::AstroportConfig, helpers::hashset};

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
    pub fn ask(&self) -> StdResult<AssetInfo> {
        match self.operations.last() {
            Some(step) => Ok(step.ask()),
            None => Err(StdError::generic_err("failed to get last step of AstroportRoute")),
        }
    }

    pub fn offer(&self) -> StdResult<AssetInfo> {
        match self.operations.first() {
            Some(step) => Ok(step.offer()),
            None => Err(StdError::generic_err("failed to get first step of AstroportRoute")),
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
                msg: to_json_binary(&mars_types::oracle::QueryMsg::Price {
                    denom: denom.to_string(),
                    kind: None,
                })?,
            }))
            .map(|res| res.price)
    }

    pub fn estimate_out_amount(
        &self,
        querier: &QuerierWrapper,
        coin_in: &Coin,
    ) -> ContractResult<Uint128> {
        // Validate the input coin
        match self.offer()? {
            AssetInfo::NativeToken {
                denom,
            } => {
                if coin_in.denom != denom {
                    Err(ContractError::InvalidRoute {
                        reason: format!(
                            "invalid offer denom: expected {}, got {}",
                            denom, coin_in.denom
                        ),
                    })
                } else {
                    Ok(())
                }
            }
            token => Err(ContractError::InvalidRoute {
                reason: format!("invalid offer denom: expected {}, got {}", token, coin_in.denom),
            }),
        }?;

        // Query oracle for prices
        let base_denom_per_offer_unit = self.query_oracle_price(querier, self.offer()?)?;
        let base_denom_per_ask_unit = self.query_oracle_price(querier, self.ask()?)?;

        // Calculate the minimum amount of output tokens to receive
        Ok(coin_in
            .amount
            .checked_mul_floor(base_denom_per_offer_unit.checked_div(base_denom_per_ask_unit)?)?)
    }
}

impl Route<Empty, Empty, AstroportConfig> for AstroportRoute {
    fn from(route: SwapperRoute, config: Option<AstroportConfig>) -> ContractResult<Self> {
        let Some(config) = config else {
            return Err(ContractError::InvalidRoute {
                reason: "AstroportConfig not set".to_string(),
            });
        };
        match route {
            SwapperRoute::Astro(route) => {
                let operations: Vec<_> = route
                    .swaps
                    .into_iter()
                    .map(|op| SwapOperation::AstroSwap {
                        offer_asset_info: AssetInfo::NativeToken {
                            denom: op.from,
                        },
                        ask_asset_info: AssetInfo::NativeToken {
                            denom: op.to,
                        },
                    })
                    .collect();
                Ok(Self {
                    operations,
                    router: config.router,
                    factory: config.factory,
                    oracle: config.oracle,
                })
            }
            SwapperRoute::Osmo(_) => Err(ContractError::InvalidRoute {
                reason: "OsmosisRoute not supported".to_string(),
            }),
        }
    }

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
        let mut prev_denom_out = AssetInfo::NativeToken {
            denom: denom_in.to_string(),
        };
        let mut seen_denoms = hashset(&[prev_denom_out.clone()]);
        for step in steps.iter() {
            let offer = step.offer();
            let ask = step.ask();

            if offer != prev_denom_out {
                return Err(ContractError::InvalidRoute {
                    reason: format!(
                        "the route's offer denom {offer} does not match the previous step's ask {prev_denom_out}",
                    ),
                });
            }

            if seen_denoms.contains(&ask) {
                return Err(ContractError::InvalidRoute {
                    reason: format!("route contains a loop: denom {} seen twice", ask),
                });
            }

            prev_denom_out = ask.clone();
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

        // Calculate the minimum amount of output tokens to receive
        let out_amount = self.estimate_out_amount(querier, coin_in)?;
        let minimum_receive = Some((Decimal::one() - slippage) * out_amount);

        let swap_msg: CosmosMsg = WasmMsg::Execute {
            contract_addr: self.router.clone(),
            msg: to_json_binary(&astroport_v5::router::ExecuteMsg::ExecuteSwapOperations {
                operations: self.operations.clone(),
                minimum_receive,
                to: None,
                // If we set max_spread to None, Astroport will unwrap it as 0.5%, so instead we
                // set it to the max allowed since we control slippage via minimum_receive instead.
                max_spread: Some(Decimal::from_str(MAX_ALLOWED_SLIPPAGE)?),
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
                ..
            } => unimplemented!("NativeSwap not implemented"),
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
                ..
            } => unimplemented!("NativeSwap not implemented"),
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
    fn swap_operation_ask_and_offer() {
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
    #[should_panic]
    fn swap_operation_ask_native_swap() {
        let op = SwapOperation::NativeSwap {
            offer_denom: "uosmo".to_string(),
            ask_denom: "uusd".to_string(),
        };

        op.ask();
    }

    #[test]
    #[should_panic]
    fn swap_operation_offer_native_swap() {
        let op = SwapOperation::NativeSwap {
            offer_denom: "uosmo".to_string(),
            ask_denom: "uusd".to_string(),
        };

        op.offer();
    }

    #[test]
    fn astroport_route_offer_and_ask() {
        let route = AstroportRoute {
            operations: vec![
                SwapOperation::AstroSwap {
                    offer_asset_info: AssetInfo::NativeToken {
                        denom: "uosmo".to_string(),
                    },
                    ask_asset_info: AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
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
            route.ask().unwrap(),
            AssetInfo::NativeToken {
                denom: "uatom".to_string()
            }
        );
        assert_eq!(
            route.offer().unwrap(),
            AssetInfo::NativeToken {
                denom: "uosmo".to_string()
            }
        );
    }

    #[test]
    #[should_panic]
    fn astroport_route_native_swap_offer() {
        let route = AstroportRoute {
            operations: vec![SwapOperation::NativeSwap {
                offer_denom: "uosmo".to_string(),
                ask_denom: "uusd".to_string(),
            }],
            router: "router".to_string(),
            oracle: "oracle".to_string(),
            factory: "factory".to_string(),
        };

        route.offer().unwrap();
    }

    #[test]
    #[should_panic]
    fn astroport_route_native_swap_ask() {
        let route = AstroportRoute {
            operations: vec![SwapOperation::NativeSwap {
                offer_denom: "uosmo".to_string(),
                ask_denom: "uusd".to_string(),
            }],
            router: "router".to_string(),
            oracle: "oracle".to_string(),
            factory: "factory".to_string(),
        };

        route.ask().unwrap();
    }
}
