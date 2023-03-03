use std::collections::HashMap;

use cosmwasm_std::{to_binary, Addr, Binary, ContractResult, QuerierResult};
use pyth_sdk_cw::{PriceFeedResponse, PriceIdentifier, QueryMsg};

#[derive(Default)]
pub struct PythQuerier {
    pub prices: HashMap<PriceIdentifier, PriceFeedResponse>,
}

impl PythQuerier {
    pub fn handle_query(&self, _contract_addr: &Addr, query: QueryMsg) -> QuerierResult {
        let res: ContractResult<Binary> = match query {
            QueryMsg::PriceFeed {
                id,
            } => {
                let option_price = self.prices.get(&id);

                if let Some(price) = option_price {
                    to_binary(price).into()
                } else {
                    Err(format!("[mock]: could not find Pyth price for {id}")).into()
                }
            }

            _ => Err("[mock]: Unsupported Pyth query").into(),
        };

        Ok(res).into()
    }
}
