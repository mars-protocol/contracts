use cosmwasm_std::{attr, Attribute, DepsMut, Env, Response};
use mars_oracle_base::{ContractError, ContractResult};
use mars_types::oracle::AstroportTwapSnapshot;

use crate::{
    contract::WasmOracle, helpers::query_astroport_cumulative_price,
    state::ASTROPORT_TWAP_SNAPSHOTS, WasmPriceSourceChecked,
};

pub trait ExecuteTwapSnapshots {
    fn execute_record_astroport_twap_snapshots(
        &self,
        deps: DepsMut,
        env: Env,
        denoms: Vec<String>,
    ) -> ContractResult<Response>;
}

impl ExecuteTwapSnapshots for WasmOracle<'_> {
    fn execute_record_astroport_twap_snapshots(
        &self,
        deps: DepsMut,
        env: Env,
        denoms: Vec<String>,
    ) -> ContractResult<Response> {
        let timestamp = env.block.time.seconds();
        let mut attrs: Vec<Attribute> = vec![];

        for denom in denoms {
            let price_source = self.price_sources.load(deps.storage, &denom)?;

            // Asset must be configured to use TWAP price source
            let (pair_address, window_size, tolerance) = match price_source {
                WasmPriceSourceChecked::AstroportTwap {
                    pair_address,
                    window_size,
                    tolerance,
                } => (pair_address, window_size, tolerance),
                WasmPriceSourceChecked::Lsd {
                    transitive_denom: _,
                    twap,
                    redemption_rate: _,
                } => (twap.pair_address, twap.window_size, twap.tolerance),
                _ => {
                    return Err(ContractError::PriceSourceNotTwap {});
                }
            };

            // Load existing snapshots. If there's none, we initialize an empty vector
            let mut snapshots =
                ASTROPORT_TWAP_SNAPSHOTS.load(deps.storage, &denom).unwrap_or_else(|_| vec![]);

            // A potential attack is to repeatly call `RecordTwapSnapshots` so that `snapshots` becomes a
            // very big vector, so that calculating the average price becomes extremely gas expensive.
            // To deter this, we reject a new snapshot if the most recent snapshot is less than `tolerance`
            // seconds ago.
            if let Some(latest_snapshot) = snapshots.last() {
                if timestamp - latest_snapshot.timestamp < tolerance {
                    continue;
                }
            }

            // Query new price data
            let price_cumulative =
                query_astroport_cumulative_price(&deps.querier, &pair_address, &denom)?;

            // Purge snapshots that are too old, i.e. more than (window_size + tolerance) away from the
            // current timestamp. These snapshots will never be used in the future for calculating
            // average prices
            snapshots.retain(|snapshot| timestamp - snapshot.timestamp <= window_size + tolerance);

            snapshots.push(AstroportTwapSnapshot {
                timestamp,
                price_cumulative,
            });

            ASTROPORT_TWAP_SNAPSHOTS.save(deps.storage, &denom, &snapshots)?;

            attrs.extend(vec![attr("denom", denom), attr("price_cumulative", price_cumulative)]);
        }

        Ok(Response::new()
            .add_attribute("action", "record_twap_snapshots")
            .add_attribute("timestamp", timestamp.to_string())
            .add_attributes(attrs))
    }
}
