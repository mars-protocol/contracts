#![cfg(not(target_arch = "wasm32"))]

extern crate core;

pub mod astroport_incentives_querier;
#[cfg(feature = "astroport")]
pub mod astroport_swapper;
mod cosmwasm_pool_querier;
/// cosmwasm_std::testing overrides and custom test helpers
mod helpers;
mod incentives_querier;
mod mars_mock_querier;
mod mock_address_provider;
mod mocks;
pub mod multitest;
mod oracle_querier;
mod osmosis_querier;
mod params_querier;
mod pyth_querier;
mod red_bank_querier;
mod redemption_rate_querier;
pub mod test_runner;
#[cfg(feature = "astroport")]
pub mod wasm_oracle;

pub use helpers::*;
pub use mars_mock_querier::MarsMockQuerier;
pub use mocks::*;

pub mod integration;
