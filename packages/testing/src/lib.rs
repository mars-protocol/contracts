extern crate core;

/// cosmwasm_std::testing overrides and custom test helpers
#[cfg(not(target_arch = "wasm32"))]
mod helpers;
#[cfg(not(target_arch = "wasm32"))]
mod incentives_querier;
#[cfg(not(target_arch = "wasm32"))]
mod mars_mock_querier;
#[cfg(not(target_arch = "wasm32"))]
mod mock_address_provider;
#[cfg(not(target_arch = "wasm32"))]
mod mocks;
#[cfg(not(target_arch = "wasm32"))]
mod oracle_querier;
#[cfg(not(target_arch = "wasm32"))]
mod osmosis_querier;
#[cfg(not(target_arch = "wasm32"))]
mod red_bank_querier;

#[cfg(not(target_arch = "wasm32"))]
pub use helpers::*;
#[cfg(not(target_arch = "wasm32"))]
pub use mars_mock_querier::MarsMockQuerier;
#[cfg(not(target_arch = "wasm32"))]
pub use mocks::*;

pub mod integration;
