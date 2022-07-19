use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Api, Decimal, StdResult};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PriceSource {
    /// Returns a fixed value;
    Fixed { price: Decimal },
}

impl fmt::Display for PriceSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let label = match self {
            PriceSource::Fixed { .. } => "fixed",
        };
        write!(f, "{}", label)
    }
}

pub type PriceSourceUnchecked = PriceSource;
pub type PriceSourceChecked = PriceSource;

impl PriceSourceUnchecked {
    pub fn to_checked(&self, _api: &dyn Api) -> StdResult<PriceSourceChecked> {
        Ok(match self {
            PriceSourceUnchecked::Fixed { price } => PriceSourceChecked::Fixed { price: *price },
        })
    }
}

/// Contract global configuration
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
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

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    pub struct MigrateMsg {}
}

pub mod helpers {
    use cosmwasm_std::{
        to_binary, Addr, Decimal, QuerierWrapper, QueryRequest, StdResult, WasmQuery,
    };

    use crate::asset::AssetType;

    use super::msg::QueryMsg;

    pub fn query_price(
        querier: QuerierWrapper,
        oracle_address: &Addr,
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
