use std::collections::HashMap;

use cosmwasm_std::{to_json_binary, Binary, ContractResult, QuerierResult};
use ica_oracle::msg::{QueryMsg, RedemptionRateResponse};

#[derive(Default)]
pub struct RedemptionRateQuerier {
    pub redemption_rates: HashMap<String, RedemptionRateResponse>,
}

impl RedemptionRateQuerier {
    pub fn handle_query(&self, query: QueryMsg) -> QuerierResult {
        let res: ContractResult<Binary> = match query {
            QueryMsg::RedemptionRate {
                denom,
                params: _,
            } => {
                let option_rr = self.redemption_rates.get(&denom);

                if let Some(rr) = option_rr {
                    to_json_binary(rr).into()
                } else {
                    Err(format!("[mock]: could not find redemption rate for denom {}", denom))
                        .into()
                }
            }

            _ => Err("[mock]: Unsupported Stride query").into(),
        };

        Ok(res).into()
    }
}
