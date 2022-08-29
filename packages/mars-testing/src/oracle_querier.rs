use cosmwasm_std::{to_binary, Addr, Binary, ContractResult, Decimal, QuerierResult};
use std::collections::HashMap;

use mars_outpost::oracle::{PriceResponse, QueryMsg};

#[derive(Default)]
pub struct OracleQuerier {
    pub prices: HashMap<String, Decimal>,
}

impl OracleQuerier {
    pub fn handle_query(&self, contract_addr: &Addr, query: QueryMsg) -> QuerierResult {
        let oracle = Addr::unchecked("oracle");
        if *contract_addr != oracle {
            panic!("[mock]: Oracle request made to {} shoud be {}", contract_addr, oracle);
        }

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
                    Err(format!("[mock]: could not find oracle price for {}", denom)).into()
                }
            }

            _ => Err("[mock]: Unsupported oracle query").into(),
        };

        Ok(ret).into()
    }
}
