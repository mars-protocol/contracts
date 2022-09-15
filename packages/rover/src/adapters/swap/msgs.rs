use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config<T> {
    /// The contract's owner, who can update config
    pub owner: T,
}

pub type InstantiateMsg = Config<String>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg<Route> {
    /// Update contract config
    UpdateConfig { owner: Option<String> },
    /// Configure the route for swapping an asset
    ///
    /// This is chain-specific, and can include parameters such as slippage tolerance and the routes
    /// for multi-step swaps
    SetRoute {
        denom_in: String,
        denom_out: String,
        route: Route,
    },
    /// Perform a swapper with an exact-in amount. Requires slippage allowance %.
    SwapExactIn {
        coin_in: Coin,
        denom_out: String,
        slippage: Decimal,
    },
    /// Send swapper results back to swapper. Also refunds extra if sent more than needed. Internal use only.
    TransferResult {
        recipient: Addr,
        denom_in: String,
        denom_out: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Query contract config. Returns `Config<String>`
    Config {},
    /// Get route for swapping an input denom into an output denom; response: `RouteResponse`
    Route { denom_in: String, denom_out: String },
    /// Enumerate all swapper routes; response: `RoutesResponse`
    Routes {
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    },
    /// Return current spot price swapping In for Out
    /// Warning: Do not use this as an oracle price feed. Use Mars-Oracle for pricing.
    /// Returns `EstimateExactInSwapResponse`
    EstimateExactInSwap { coin_in: Coin, denom_out: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct RouteResponse<Route> {
    pub denom_in: String,
    pub denom_out: String,
    pub route: Route,
}

pub type RoutesResponse<Route> = Vec<RouteResponse<Route>>;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct EstimateExactInSwapResponse {
    pub amount: Uint128,
}
