use mars_types::oracle;

use crate::OsmosisPriceSource;

pub type ExecuteMsg = oracle::ExecuteMsg<OsmosisPriceSource>;
pub type PriceSourceResponse = oracle::PriceSourceResponse<OsmosisPriceSource>;
