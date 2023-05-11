#![cfg(not(target_arch = "wasm32"))]

extern crate core;

#[cfg(feature = "astroport")]
pub mod astroport_swapper;
/// cosmwasm_std::testing overrides and custom test helpers
mod helpers;
mod mars_mock_querier;
mod mocks;
mod oracle_querier;
mod osmosis_querier;
mod pyth_querier;
mod redemption_rate_querier;
pub mod test_runner;
#[cfg(feature = "astroport")]
pub mod wasm_oracle;

pub use helpers::*;
pub use mars_mock_querier::MarsMockQuerier;
pub use mocks::*;
