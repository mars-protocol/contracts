use crate::error::MarsHealthResult;
use cosmwasm_std::{Addr, Decimal, QuerierWrapper};
use mars_outpost::oracle::{PriceResponse, QueryMsg as OracleQueryMsg};
use mars_outpost::red_bank::{Market, QueryMsg as RedBankQueryMsg};

pub struct MarsQuerier<'a> {
    querier: &'a QuerierWrapper<'a>,
    oracle_addr: Addr,
    redbank_addr: Addr,
}

impl<'a> MarsQuerier<'a> {
    pub fn new(querier: &'a QuerierWrapper, oracle_addr: Addr, redbank_addr: Addr) -> Self {
        MarsQuerier {
            querier,
            oracle_addr,
            redbank_addr,
        }
    }

    pub fn query_market(&self, denom: &str) -> MarsHealthResult<Market> {
        Ok(self.querier.query_wasm_smart(
            self.redbank_addr.clone(),
            &RedBankQueryMsg::Market {
                denom: denom.to_string(),
            },
        )?)
    }

    pub fn query_price(&self, denom: &str) -> MarsHealthResult<Decimal> {
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
