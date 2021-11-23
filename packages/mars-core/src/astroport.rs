// Type definitions of relevant Astroport contracts. We have to define them here because Astroport
// has not uploaded their package to crates.io. Once they've uploaded, we can remove this
pub mod asset {
    use std::fmt;

    use cosmwasm_std::{Addr, Uint128};
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    pub struct Asset {
        pub info: AssetInfo,
        pub amount: Uint128,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum AssetInfo {
        Token { contract_addr: Addr },
        NativeToken { denom: String },
    }

    impl fmt::Display for AssetInfo {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                AssetInfo::NativeToken { denom } => write!(f, "{}", denom),
                AssetInfo::Token { contract_addr } => write!(f, "{}", contract_addr),
            }
        }
    }
}

pub mod pair {
    use cosmwasm_std::{Addr, Decimal, Uint128};
    use cw20::Cw20ReceiveMsg;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    use super::{
        asset::{Asset, AssetInfo},
        factory::PairType,
    };

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub struct PairInfo {
        pub asset_infos: [AssetInfo; 2],
        pub contract_addr: Addr,
        pub liquidity_token: Addr,
        pub pair_type: PairType,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum QueryMsg {
        Pool {},
        Simulation { offer_asset: Asset },
        CumulativePrices {},
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    pub struct PoolResponse {
        pub assets: [Asset; 2],
        pub total_share: Uint128,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    pub struct SimulationResponse {
        pub return_amount: Uint128,
        pub spread_amount: Uint128,
        pub commission_amount: Uint128,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    pub struct CumulativePricesResponse {
        pub assets: [Asset; 2],
        pub total_share: Uint128,
        pub price0_cumulative_last: Uint128,
        pub price1_cumulative_last: Uint128,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum ExecuteMsg {
        Receive(Cw20ReceiveMsg),
        /// Post initialize step to allow user to set controlled contract address after creating it
        PostInitialize {},
        /// ProvideLiquidity a user provides pool liquidity
        ProvideLiquidity {
            assets: [Asset; 2],
            slippage_tolerance: Option<Decimal>,
            auto_stack: Option<bool>,
        },
        /// Swap an offer asset to the other
        Swap {
            offer_asset: Asset,
            belief_price: Option<Decimal>,
            max_spread: Option<Decimal>,
            to: Option<String>,
        },
        UpdateConfig {
            amp: Option<u64>,
        },
    }
}

pub mod factory {
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    use super::asset::AssetInfo;

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum PairType {
        Xyk {},
        Stable {},
        Custom { pair_type: String },
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum QueryMsg {
        Config {},
        Pair {
            asset_infos: [AssetInfo; 2],
        },
        Pairs {
            start_after: Option<[AssetInfo; 2]>,
            limit: Option<u32>,
        },
        FeeInfo {
            pair_type: PairType,
        },
    }
}

pub mod querier {
    use cosmwasm_std::{to_binary, Addr, QuerierWrapper, QueryRequest, StdResult, WasmQuery};

    use super::{asset::AssetInfo, factory::QueryMsg as FactoryQueryMsg, pair::PairInfo};

    pub fn query_pair_info(
        querier: &QuerierWrapper,
        factory_contract: Addr,
        asset_infos: &[AssetInfo; 2],
    ) -> StdResult<PairInfo> {
        querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: factory_contract.to_string(),
            msg: to_binary(&FactoryQueryMsg::Pair {
                asset_infos: asset_infos.clone(),
            })?,
        }))
    }
}
