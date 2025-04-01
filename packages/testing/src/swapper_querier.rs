use std::collections::HashMap;

use cosmwasm_std::{to_json_binary, Addr, Decimal, QuerierResult};
use mars_types::swapper::{EstimateExactInSwapResponse, QueryMsg};

#[derive(Default)]
pub struct SwapperQuerier {
    pub swap_prices: HashMap<String, Decimal>,
}

impl SwapperQuerier {
    pub fn handle_query(&self, _contract_addr: &Addr, query: QueryMsg) -> QuerierResult {
        let ret = match query {
            QueryMsg::EstimateExactInSwap {
                coin_in,
                denom_out,
                route: _,
            } => {
                let denom_in = coin_in.denom.clone();
                let denom_in_price = self.swap_prices.get(&denom_in).unwrap();
                let denom_out_price = self.swap_prices.get(&denom_out).unwrap();

                let price = denom_in_price / denom_out_price;
                let amount = coin_in.amount * price;
                to_json_binary(&EstimateExactInSwapResponse {
                    amount,
                })
                .into()
            }
            _ => Err("[mock]: Unsupported swapper query").into(),
        };

        Ok(ret).into()
    }
}
