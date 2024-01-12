use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use mars_owner::OwnerUpdate;

#[cw_serde]
pub enum SwapperRoute {
    Astro(AstroportRoute),
    Osmo(OsmosisRoute),
}

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

#[cw_serde]
pub struct SwapOperation {
    /// Asset to swap from
    pub from: String,
    /// Asset to swap to
    pub to: String,
}

#[cw_serde]
pub struct OsmosisRoute(pub Vec<SwapAmountInRoute>);

#[cw_serde]
pub struct SwapAmountInRoute {
    pub pool_id: u64,
    pub token_out_denom: String,
}

#[cw_serde]
pub struct InstantiateMsg {
    /// The contract's owner, who can update config
    pub owner: String,
}

#[cw_serde]
pub enum ExecuteMsg<Route> {
    /// Manges owner role state
    UpdateOwner(OwnerUpdate),
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
        route: SwapperRoute,
    },
    /// Send swapper results back to swapper. Also refunds extra if sent more than needed. Internal use only.
    TransferResult {
        recipient: Addr,
        denom_in: String,
        denom_out: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Query contract owner config
    #[returns(mars_owner::OwnerResponse)]
    Owner {},
    /// Get route for swapping an input denom into an output denom
    #[returns(RouteResponse<cosmwasm_std::Empty>)]
    Route {
        denom_in: String,
        denom_out: String,
    },
    /// Enumerate all swapper routes
    #[returns(RoutesResponse<cosmwasm_std::Empty>)]
    Routes {
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    },
    /// Return current spot price swapping In for Out
    /// Warning: Do not use this as an oracle price feed. Use Mars-Oracle for pricing.
    #[returns(EstimateExactInSwapResponse)]
    EstimateExactInSwap {
        coin_in: Coin,
        denom_out: String,
        route: SwapperRoute,
    },
}

#[cw_serde]
pub struct RouteResponse<Route> {
    pub denom_in: String,
    pub denom_out: String,
    pub route: Route,
}

pub type RoutesResponse<Route> = Vec<RouteResponse<Route>>;

#[cw_serde]
pub struct EstimateExactInSwapResponse {
    pub amount: Uint128,
}
