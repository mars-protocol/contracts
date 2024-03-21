use cosmwasm_std::{Decimal, DepsMut, Order, Response, StdResult};
use cw2::{assert_contract_version, set_contract_version};
use mars_oracle_base::ContractError;
use mars_types::oracle::V2Updates;

use crate::{
    contract::{WasmOracle, CONTRACT_NAME, CONTRACT_VERSION},
    WasmPriceSourceChecked,
};

const FROM_VERSION: &str = "1.2.1";

/// Use only PriceSource types which are currently configured in the Neutron oracle
pub mod v1_state {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{Addr, Decimal};
    use cw_storage_plus::Map;
    use pyth_sdk_cw::PriceIdentifier;

    pub const PRICE_SOURCES: Map<&str, WasmPriceSourceChecked> = Map::new("price_sources");

    #[cw_serde]
    pub enum WasmPriceSource<A> {
        Fixed {
            price: Decimal,
        },
        AstroportTwap {
            pair_address: A,
            window_size: u64,
            tolerance: u64,
        },
        Pyth {
            contract_addr: A,
            price_feed_id: PriceIdentifier,
            max_staleness: u64,
            denom_decimals: u8,
        },
    }

    pub type WasmPriceSourceUnchecked = WasmPriceSource<String>;
    pub type WasmPriceSourceChecked = WasmPriceSource<Addr>;
}

pub fn migrate(deps: DepsMut, msg: V2Updates) -> Result<Response, ContractError> {
    // make sure we're migrating the correct contract and from the correct version
    assert_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;

    let price_sources = v1_state::PRICE_SOURCES
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    v1_state::PRICE_SOURCES.clear(deps.storage);
    let wasm_oracle = WasmOracle::default();
    for (denom, ps) in price_sources.into_iter() {
        wasm_oracle.price_sources.save(
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
    value: v1_state::WasmPriceSourceChecked,
    max_confidence: Decimal,
    max_deviation: Decimal,
) -> WasmPriceSourceChecked {
    match value {
        v1_state::WasmPriceSource::Fixed {
            price,
        } => WasmPriceSourceChecked::Fixed {
            price,
        },
        v1_state::WasmPriceSource::AstroportTwap {
            pair_address,
            window_size,
            tolerance,
        } => WasmPriceSourceChecked::AstroportTwap {
            pair_address,
            window_size,
            tolerance,
        },
        v1_state::WasmPriceSource::Pyth {
            contract_addr,
            price_feed_id,
            max_staleness,
            denom_decimals,
        } => WasmPriceSourceChecked::Pyth {
            contract_addr,
            price_feed_id,
            max_staleness,
            max_confidence,
            max_deviation,
            denom_decimals,
        },
    }
}
