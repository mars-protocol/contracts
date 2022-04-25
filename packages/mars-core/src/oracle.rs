use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Api, StdResult, Uint128};

use crate::math::decimal::Decimal;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PriceSource<A> {
    /// Returns a fixed value; used for UST
    Fixed { price: Decimal },
    /// Native Terra stablecoins transaction rate quoted in UST
    Native { denom: String },
    /// Astroport spot price quoted in UST
    ///
    /// NOTE: `pair_address` must point to an astroport pair consists of the asset of intereset and UST
    AstroportSpot {
        /// Address of the Astroport pair
        pair_address: A,
    },
    /// Astroport TWAP price quoted in UST
    ///
    /// NOTE: `pair_address` must point to an astroport pair consists of the asset of intereset and UST
    AstroportTwap {
        /// Address of the Astroport pair
        pair_address: A,
        /// Address of the asset of interest
        ///
        /// NOTE: Spot price in intended for CW20 tokens. Terra native tokens should use Fixed or
        /// Native price sources.
        window_size: u64,
        /// When calculating averaged price, we take the most recent TWAP snapshot and find a second
        /// snapshot in the range of window_size +/- tolerance. For example, if window size is 5 minutes
        /// and tolerance is 1 minute, we look for snapshots that are 4 - 6 minutes back in time from
        /// the most recent snapshot.
        ///
        /// If there are multiple snapshots within the range, we take the one that is closest to the
        /// desired window size.
        tolerance: u64,
    },
    /// Astroport liquidity token
    ///
    /// NOTE: Astroport's pair contract does not have a query command to check the address of the LP
    /// token associated with a pair. Therefore, we can't implement relevant checks in the contract.
    /// The owner must make sure the addresses supplied are accurate
    AstroportLiquidityToken {
        /// Address of the asset of interest
        pair_address: A,
    },
    /// stLuna price calculated from stLuna/Luna exchange rate from Lido hub contract and Luna price from current price source
    Stluna { hub_address: A },
}

impl<A> fmt::Display for PriceSource<A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let label = match self {
            PriceSource::Fixed { .. } => "fixed",
            PriceSource::Native { .. } => "native",
            PriceSource::AstroportSpot { .. } => "astroport_spot",
            PriceSource::AstroportTwap { .. } => "astroport_twap",
            PriceSource::AstroportLiquidityToken { .. } => "astroport_liquidity_token",
            PriceSource::Stluna { .. } => "stluna",
        };
        write!(f, "{}", label)
    }
}

pub type PriceSourceUnchecked = PriceSource<String>;
pub type PriceSourceChecked = PriceSource<Addr>;

impl PriceSourceUnchecked {
    pub fn to_checked(&self, api: &dyn Api) -> StdResult<PriceSourceChecked> {
        Ok(match self {
            PriceSourceUnchecked::Fixed { price } => PriceSourceChecked::Fixed { price: *price },
            PriceSourceUnchecked::Native { denom } => PriceSourceChecked::Native {
                denom: denom.clone(),
            },
            PriceSourceUnchecked::AstroportSpot { pair_address } => {
                PriceSourceChecked::AstroportSpot {
                    pair_address: api.addr_validate(pair_address)?,
                }
            }
            PriceSourceUnchecked::AstroportTwap {
                pair_address,
                window_size,
                tolerance,
            } => PriceSourceChecked::AstroportTwap {
                pair_address: api.addr_validate(pair_address)?,
                window_size: *window_size,
                tolerance: *tolerance,
            },
            PriceSourceUnchecked::AstroportLiquidityToken { pair_address } => {
                PriceSourceChecked::AstroportLiquidityToken {
                    pair_address: api.addr_validate(pair_address)?,
                }
            }
            PriceSourceUnchecked::Stluna { hub_address } => PriceSourceChecked::Stluna {
                hub_address: api.addr_validate(hub_address)?,
            },
        })
    }
}

/// Contract global configuration
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AstroportTwapSnapshot {
    /// Timestamp of the most recent TWAP data update
    pub timestamp: u64,
    /// Cumulative price of the asset retrieved by the most recent TWAP data update
    pub price_cumulative: Uint128,
}

pub mod msg {
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    use super::PriceSourceUnchecked;
    use crate::asset::Asset;

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    pub struct InstantiateMsg {
        pub owner: String,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum ExecuteMsg {
        /// Update contract config
        UpdateConfig { owner: Option<String> },
        /// Specify parameters to query asset price
        SetAsset {
            asset: Asset,
            price_source: PriceSourceUnchecked,
        },
        /// Fetch cumulative prices from Astroport pairs and record in contract storage
        RecordTwapSnapshots { assets: Vec<Asset> },
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum QueryMsg {
        /// Query contract config. Returns `Config`
        Config {},
        /// Get asset's price source. Returns `AssetConfigChecked`
        AssetPriceSource { asset: Asset },
        /// Query asset price given an asset; returns `mars_core::math::decimal::Decimal`
        AssetPrice { asset: Asset },
        /// Query asset price given it's internal reference; returns `mars_core::math::decimal::Decimal`
        ///
        /// NOTE: meant to be used by protocol contracts only
        AssetPriceByReference { asset_reference: Vec<u8> },
    }
}

pub mod helpers {
    use cosmwasm_std::{to_binary, Addr, QuerierWrapper, QueryRequest, StdResult, WasmQuery};

    use crate::asset::AssetType;
    use crate::math::decimal::Decimal;

    use super::msg::QueryMsg;

    pub fn query_price(
        querier: QuerierWrapper,
        oracle_address: Addr,
        asset_label: &str,
        asset_reference: Vec<u8>,
        asset_type: AssetType,
    ) -> StdResult<Decimal> {
        // For UST, we skip the query and just return 1 to save gas
        if asset_type == AssetType::Native && asset_label == "uusd" {
            Ok(Decimal::one())
        } else {
            querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: oracle_address.into(),
                msg: to_binary(&QueryMsg::AssetPriceByReference { asset_reference })?,
            }))
        }
    }
}
