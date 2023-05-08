use mars_oracle as oracle;

use crate::price_source::{OsmosisPriceSourceChecked, OsmosisPriceSourceUnchecked};

pub type ExecuteMsg = oracle::ExecuteMsg<OsmosisPriceSourceUnchecked>;
pub type PriceSourceResponse = oracle::PriceSourceResponse<OsmosisPriceSourceChecked>;
