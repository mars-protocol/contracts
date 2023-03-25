#![cfg(not(target_arch = "wasm32"))]

extern crate core;

/// cosmwasm_std::testing overrides and custom test helpers
mod helpers;
mod mars_mock_querier;
mod mocks;
mod oracle_querier;
mod osmosis_querier;

pub use helpers::*;
pub use mars_mock_querier::MarsMockQuerier;
pub use mocks::*;
