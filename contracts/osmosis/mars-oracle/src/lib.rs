pub mod contract;
mod helpers;
mod price_source;

pub use price_source::OsmosisPriceSource;

pub type ExecuteMsg = mars_outpost::oracle::ExecuteMsg<OsmosisPriceSource>;
pub type PriceSourceResponse = mars_outpost::oracle::PriceSourceResponse<OsmosisPriceSource>;

#[cfg(test)]
mod testing;
