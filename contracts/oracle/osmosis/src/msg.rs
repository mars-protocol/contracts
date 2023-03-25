use mars_oracle as oracle;

use crate::OsmosisPriceSource;

pub type ExecuteMsg = oracle::ExecuteMsg<OsmosisPriceSource>;
pub type PriceSourceResponse = oracle::PriceSourceResponse<OsmosisPriceSource>;
