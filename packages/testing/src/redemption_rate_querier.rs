use std::collections::HashMap;

use cosmwasm_std::{to_binary, Binary, ContractResult, QuerierResult};
use mars_oracle_osmosis::stride::{Price, RedemptionRateRequest, RedemptionRateResponse};

#[derive(Default)]
pub struct RedemptionRateQuerier {
    pub redemption_rates: HashMap<Price, RedemptionRateResponse>,
}

impl RedemptionRateQuerier {
    pub fn handle_query(&self, req: RedemptionRateRequest) -> QuerierResult {
        let res: ContractResult<Binary> = {
            let option_rr = self.redemption_rates.get(&req.price);

            if let Some(rr) = option_rr {
                to_binary(rr).into()
            } else {
                Err(format!(
                    "[mock]: could not find redemption rate for denom {} and base_denom {}",
                    req.price.denom, req.price.base_denom
                ))
                .into()
            }
        };

        Ok(res).into()
    }
}
