use std::collections::HashMap;

use cosmwasm_std::{to_binary, Addr, Binary, ContractResult, Decimal, QuerierResult};
use mars_oracle::msg::{PriceResponse, QueryMsg};

#[derive(Default)]
pub struct OracleQuerier {
    pub prices: HashMap<String, Decimal>,
}

impl OracleQuerier {
    pub fn handle_query(&self, _contract_addr: &Addr, query: QueryMsg) -> QuerierResult {
        let ret: ContractResult<Binary> = match query {
            QueryMsg::Price {
                denom,
            } => {
                let option_price = self.prices.get(&denom);

                if let Some(price) = option_price {
                    to_binary(&PriceResponse {
                        denom,
                        price: *price,
                    })
                    .into()
                } else {
                    Err(format!("[mock]: could not find oracle price for {denom}")).into()
                }
            }

            _ => Err("[mock]: Unsupported oracle query").into(),
        };

        Ok(ret).into()
    }
}
