use mars_red_bank_types::oracle;

use crate::price_source::{OsmosisPriceSourceChecked, OsmosisPriceSourceUnchecked};

pub type ExecuteMsg = oracle::ExecuteMsg<OsmosisPriceSourceUnchecked>;
pub type PriceSourceResponse = oracle::PriceSourceResponse<OsmosisPriceSourceChecked>;
