use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;

/// Contract global configuration
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
}

pub mod msg {
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    use crate::asset::Asset;

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    pub struct InstantiateMsg {
        pub owner: String,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum ExecuteMsg<T> {
        /// Update contract config
        UpdateConfig { owner: Option<String> },
        /// Specify parameters to query asset price
        SetAsset { asset: Asset, price_source: T },
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum QueryMsg {
        /// Query contract config. Returns `Config`
        Config {},
        /// Get asset's price source. Returns `AssetConfigChecked`
        AssetPriceSource { asset: Asset },
        /// Query asset price given an asset; returns `Decimal`
        AssetPrice { asset: Asset },
        /// Query asset price given it's internal reference; returns `Decimal`
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
        oracle_address: Addr,
        _asset_label: &str,
        asset_reference: Vec<u8>,
        _asset_type: AssetType,
    ) -> StdResult<Decimal> {
        querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: oracle_address.into(),
            msg: to_binary(&QueryMsg::AssetPriceByReference { asset_reference })?,
        }))
    }
}
