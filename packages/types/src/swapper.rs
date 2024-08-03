use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Uint128};
use mars_owner::OwnerUpdate;

#[cw_serde]
pub enum SwapperRoute {
    Astro(AstroRoute),
    Osmo(OsmoRoute),
}

#[cw_serde]
pub struct AstroRoute {
    /// The swap operations of the route
    pub swaps: Vec<AstroSwap>,
}

#[cw_serde]
pub struct AstroSwap {
    /// Asset to swap from
    pub from: String,
    /// Asset to swap to
    pub to: String,
}

#[cw_serde]
pub struct OsmoRoute {
    pub swaps: Vec<OsmoSwap>,
}

/// Swap operation with pool id and asset to swap to.
/// "from" is another asset in the pool.
#[cw_serde]
pub struct OsmoSwap {
    /// Pool id
    pub pool_id: u64,
    /// Asset to swap to
    pub to: String,
}

#[cw_serde]
pub struct InstantiateMsg {
    /// The contract's owner, who can update config
    pub owner: String,
}

#[cw_serde]
pub enum ExecuteMsg<Route, C> {
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
    /// Perform a swapper with an exact-in amount
    SwapExactIn {
        coin_in: Coin,
        denom_out: String,
        min_receive: Uint128,
        route: Option<SwapperRoute>,
    },
    /// Send swapper results back to swapper. Also refunds extra if sent more than needed. Internal use only.
    TransferResult {
        recipient: Addr,
        denom_in: String,
        denom_out: String,
    },
    UpdateConfig {
        config: C,
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
        route: Option<SwapperRoute>,
    },
    /// Query contract config
    #[returns(cosmwasm_std::Empty)]
    Config {},
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

#[cw_serde]
pub enum MigrateMsg {
    V1_0_0ToV2_0_0 {},
    V2_0_2ToV2_0_3 {},
}
