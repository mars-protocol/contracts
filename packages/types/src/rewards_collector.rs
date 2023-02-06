use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Api, Decimal, StdResult, Uint128};
use mars_owner::OwnerUpdate;
use mars_utils::{
    error::ValidationError,
    helpers::{decimal_param_le_one, integer_param_gt_zero, validate_native_denom},
};

const MAX_SLIPPAGE_TOLERANCE_PERCENTAGE: u64 = 50;

#[cw_serde]
pub struct InstantiateMsg {
    /// The contract's owner
    pub owner: String,
    /// Address provider returns addresses for all protocol contracts
    pub address_provider: String,
    /// Percentage of fees that are sent to the safety fund
    pub safety_tax_rate: Decimal,
    /// The asset to which the safety fund share is converted
    pub safety_fund_denom: String,
    /// The asset to which the fee collector share is converted
    pub fee_collector_denom: String,
    /// The channel ID of the mars hub
    pub channel_id: String,
    /// Number of seconds after which an IBC transfer is to be considered failed, if no acknowledgement is received
    pub timeout_seconds: u64,
    /// Maximum percentage of price movement (minimum amount you accept to receive during swap)
    pub slippage_tolerance: Decimal,
}

#[cw_serde]
pub struct Config {
    /// Address provider returns addresses for all protocol contracts
    pub address_provider: Addr,
    /// Percentage of fees that are sent to the safety fund
    pub safety_tax_rate: Decimal,
    /// The asset to which the safety fund share is converted
    pub safety_fund_denom: String,
    /// The asset to which the fee collector share is converted
    pub fee_collector_denom: String,
    /// The channel ID of the mars hub
    pub channel_id: String,
    /// Number of seconds after which an IBC transfer is to be considered failed, if no acknowledgement is received
    pub timeout_seconds: u64,
    /// Maximum percentage of price movement (minimum amount you accept to receive during swap)
    pub slippage_tolerance: Decimal,
}

impl Config {
    pub fn validate(&self) -> Result<(), ValidationError> {
        decimal_param_le_one(self.safety_tax_rate, "safety_tax_rate")?;

        integer_param_gt_zero(self.timeout_seconds, "timeout_seconds")?;

        if self.slippage_tolerance > Decimal::percent(MAX_SLIPPAGE_TOLERANCE_PERCENTAGE) {
            return Err(ValidationError::InvalidParam {
                param_name: "slippage_tolerance".to_string(),
                invalid_value: self.slippage_tolerance.to_string(),
                predicate: format!("<= {}", Decimal::percent(MAX_SLIPPAGE_TOLERANCE_PERCENTAGE)),
            });
        }

        validate_native_denom(&self.safety_fund_denom)?;
        validate_native_denom(&self.fee_collector_denom)?;

        Ok(())
    }
}

impl Config {
    pub fn checked(api: &dyn Api, msg: InstantiateMsg) -> StdResult<Config> {
        Ok(Config {
            address_provider: api.addr_validate(&msg.address_provider)?,
            safety_tax_rate: msg.safety_tax_rate,
            safety_fund_denom: msg.safety_fund_denom,
            fee_collector_denom: msg.fee_collector_denom,
            channel_id: msg.channel_id,
            timeout_seconds: msg.timeout_seconds,
            slippage_tolerance: msg.slippage_tolerance,
        })
    }
}

#[cw_serde]
#[derive(Default)]
pub struct UpdateConfig {
    /// Address provider returns addresses for all protocol contracts
    pub address_provider: Option<String>,
    /// Percentage of fees that are sent to the safety fund
    pub safety_tax_rate: Option<Decimal>,
    /// The asset to which the safety fund share is converted
    pub safety_fund_denom: Option<String>,
    /// The asset to which the fee collector share is converted
    pub fee_collector_denom: Option<String>,
    /// The channel id of the mars hub
    pub channel_id: Option<String>,
    /// Number of seconds after which an IBC transfer is to be considered failed, if no acknowledgement is received
    pub timeout_seconds: Option<u64>,
    /// Maximum percentage of price movement (minimum amount you accept to receive during swap)
    pub slippage_tolerance: Option<Decimal>,
}

#[cw_serde]
pub enum ExecuteMsg<Route> {
    /// Manages admin role state
    UpdateOwner(OwnerUpdate),

    /// Update contract config
    UpdateConfig {
        new_cfg: UpdateConfig,
    },

    /// Configure the route for swapping an asset
    ///
    /// This is chain-specific, and can include parameters such as slippage tolerance and the routes
    /// for multi-step swaps
    SetRoute {
        denom_in: String,
        denom_out: String,
        route: Route,
    },

    /// Withdraw coins from the red bank
    WithdrawFromRedBank {
        denom: String,
        amount: Option<Uint128>,
    },

    /// Distribute the accrued protocol income between the safety fund and the fee modules on mars hub,
    /// according to the split set in config.
    /// Callable by any address.
    DistributeRewards {
        denom: String,
        amount: Option<Uint128>,
    },

    /// Swap any asset on the contract
    SwapAsset {
        denom: String,
        amount: Option<Uint128>,
    },

    /// Claim rewards in incentives contract.
    ///
    /// We wanted to leave protocol rewards in the red-bank so they continue to work as liquidity (until the bot invokes WithdrawFromRedBank).
    /// As an side effect to this, if the market is incentivised with MARS tokens, the contract will also accrue MARS token incentives.
    ClaimIncentiveRewards {},
}

#[cw_serde]
pub struct ConfigResponse {
    /// The contract's owner
    pub owner: Option<String>,
    /// The contract's proposed owner
    pub proposed_new_owner: Option<String>,
    /// Address provider returns addresses for all protocol contracts
    pub address_provider: String,
    /// Percentage of fees that are sent to the safety fund
    pub safety_tax_rate: Decimal,
    /// The asset to which the safety fund share is converted
    pub safety_fund_denom: String,
    /// The asset to which the fee collector share is converted
    pub fee_collector_denom: String,
    /// The channel ID of the mars hub
    pub channel_id: String,
    /// Number of seconds after which an IBC transfer is to be considered failed, if no acknowledgement is received
    pub timeout_seconds: u64,
    /// Maximum percentage of price movement (minimum amount you accept to receive during swap)
    pub slippage_tolerance: Decimal,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get config parameters
    #[returns(ConfigResponse)]
    Config {},
    /// Get routes for swapping an input denom into an output denom.
    ///
    /// NOTE: The response type of this query is chain-specific.
    #[returns(RouteResponse<String>)]
    Route {
        denom_in: String,
        denom_out: String,
    },
    /// Enumerate all swap routes.
    ///
    /// NOTE: The response type of this query is chain-specific.
    #[returns(Vec<RouteResponse<String>>)]
    Routes {
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct RouteResponse<Route> {
    pub denom_in: String,
    pub denom_out: String,
    pub route: Route,
}

pub type RoutesResponse<Route> = Vec<RouteResponse<Route>>;
