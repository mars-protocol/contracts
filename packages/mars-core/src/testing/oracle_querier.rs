use cosmwasm_std::{to_binary, Addr, Binary, ContractResult, QuerierResult};
use std::collections::HashMap;

use crate::math::decimal::Decimal;
use crate::oracle::msg::QueryMsg;

pub struct OracleQuerier {
    pub prices: HashMap<Vec<u8>, Decimal>,
}

impl Default for OracleQuerier {
    fn default() -> Self {
        OracleQuerier {
            prices: HashMap::new(),
        }
    }
}

impl OracleQuerier {
    pub fn handle_query(&self, contract_addr: &Addr, query: QueryMsg) -> QuerierResult {
        let oracle = Addr::unchecked("oracle");
        if *contract_addr != oracle {
            panic!(
                "[mock]: Oracle request made to {} shoud be {}",
                contract_addr, oracle
            );
        }

        let ret: ContractResult<Binary> = match query {
            QueryMsg::AssetPriceByReference { asset_reference } => {
                let option_price = self.prices.get(&asset_reference);

                if let Some(price) = option_price {
                    to_binary(price).into()
                } else {
                    Err(format!(
                        "[mock]: could not find oracle price for {}",
                        String::from_utf8(asset_reference).unwrap()
                    ))
                    .into()
                }
            }

            _ => Err("[mock]: Unsupported address provider query").into(),
        };

        Ok(ret).into()
    }
}
