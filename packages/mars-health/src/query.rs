use cosmwasm_std::{Addr, Decimal, QuerierWrapper, StdResult};
use mars_outpost::oracle::{PriceResponse, QueryMsg as OracleQueryMsg};
use mars_outpost::red_bank::{Market, QueryMsg as RedBankQueryMsg};

pub struct MarsQuerier<'a> {
    querier: &'a QuerierWrapper<'a>,
    oracle_addr: Addr,
    red_bank_addr: Addr,
}

impl<'a> MarsQuerier<'a> {
    pub fn new(querier: &'a QuerierWrapper, oracle_addr: Addr, red_bank_addr: Addr) -> Self {
        MarsQuerier {
            querier,
            oracle_addr,
            red_bank_addr,
        }
    }

    pub fn query_market(&self, denom: &str) -> StdResult<Market> {
        self.querier.query_wasm_smart(
            self.red_bank_addr.clone(),
            &RedBankQueryMsg::Market {
                denom: denom.to_string(),
            },
        )
    }

    pub fn query_price(&self, denom: &str) -> StdResult<Decimal> {
        let PriceResponse {
            price,
            ..
        } = self.querier.query_wasm_smart(
            self.oracle_addr.clone(),
            &OracleQueryMsg::Price {
                denom: denom.to_string(),
            },
        )?;
        Ok(price)
    }
}
