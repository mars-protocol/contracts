use cosmwasm_std::{Decimal, DepsMut, Order, Response, StdResult};
use cw2::{assert_contract_version, set_contract_version};
use mars_oracle_base::ContractError;
use mars_types::oracle::V2Updates;
use osmosis_std::types::osmosis::downtimedetector::v1beta1::Downtime;

use crate::{
    contract::{OsmosisOracle, CONTRACT_NAME, CONTRACT_VERSION},
    DowntimeDetector, OsmosisPriceSourceChecked,
};

const FROM_VERSION: &str = "1.1.0";

/// Use only PriceSource types which are currently configured in the Osmosis oracle
pub mod v1_state {
    use cosmwasm_std::{Addr, Decimal};
    use cw_storage_plus::Map;
    use pyth_sdk_cw::PriceIdentifier;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    pub const PRICE_SOURCES: Map<&str, OsmosisPriceSourceChecked> = Map::new("price_sources");

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum Downtime {
        Duration30m = 8,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub struct DowntimeDetector {
        pub downtime: Downtime,
        pub recovery: u64,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum OsmosisPriceSource<T> {
        Fixed {
            price: Decimal,
        },
        XykLiquidityToken {
            pool_id: u64,
        },
        StakedGeometricTwap {
            transitive_denom: String,
            pool_id: u64,
            window_size: u64,
            downtime_detector: Option<DowntimeDetector>,
        },
        Pyth {
            contract_addr: T,
            price_feed_id: PriceIdentifier,
            max_staleness: u64,
            denom_decimals: u8,
        },
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub struct GeometricTwap {
        pub pool_id: u64,
        pub window_size: u64,
        pub downtime_detector: Option<DowntimeDetector>,
    }

    pub type OsmosisPriceSourceUnchecked = OsmosisPriceSource<String>;
    pub type OsmosisPriceSourceChecked = OsmosisPriceSource<Addr>;
}

pub fn migrate(deps: DepsMut, msg: V2Updates) -> Result<Response, ContractError> {
    // make sure we're migrating the correct contract and from the correct version
    assert_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;

    let price_sources = v1_state::PRICE_SOURCES
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    v1_state::PRICE_SOURCES.clear(deps.storage);
    let osmosis_oracle = OsmosisOracle::default();
    for (denom, ps) in price_sources.into_iter() {
        osmosis_oracle.price_sources.save(
            deps.storage,
            &denom,
            &from_v1_to_v2(ps, msg.max_confidence, msg.max_deviation),
        )?;
    }

    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", CONTRACT_VERSION))
}

fn from_v1_to_v2(
    value: v1_state::OsmosisPriceSourceChecked,
    max_confidence: Decimal,
    max_deviation: Decimal,
) -> OsmosisPriceSourceChecked {
    match value {
        v1_state::OsmosisPriceSource::Fixed {
            price,
        } => OsmosisPriceSourceChecked::Fixed {
            price,
        },
        v1_state::OsmosisPriceSource::XykLiquidityToken {
            pool_id,
        } => OsmosisPriceSourceChecked::XykLiquidityToken {
            pool_id,
        },
        v1_state::OsmosisPriceSource::StakedGeometricTwap {
            transitive_denom,
            pool_id,
            window_size,
            downtime_detector,
        } => OsmosisPriceSourceChecked::StakedGeometricTwap {
            transitive_denom,
            pool_id,
            window_size,
            downtime_detector: downtime_detector.map(|dd| dd.into()),
        },
        v1_state::OsmosisPriceSource::Pyth {
            contract_addr,
            price_feed_id,
            max_staleness,
            denom_decimals,
        } => OsmosisPriceSourceChecked::Pyth {
            contract_addr,
            price_feed_id,
            max_staleness,
            max_confidence,
            max_deviation,
            denom_decimals,
        },
    }
}

/// Use Downtime from osmosis_std which wasn't available before (there was some issue with serialization)
/// It changes Downtime value from lower case to upper case.
/// V1: duration30m
/// V2: Duration30m
impl From<v1_state::DowntimeDetector> for DowntimeDetector {
    fn from(value: v1_state::DowntimeDetector) -> Self {
        let downtime = match value.downtime {
            v1_state::Downtime::Duration30m => Downtime::Duration30m,
        };
        DowntimeDetector {
            downtime,
            recovery: value.recovery,
        }
    }
}
