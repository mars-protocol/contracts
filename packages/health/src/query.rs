use cosmwasm_std::{Addr, Decimal, QuerierWrapper, StdResult};
use mars_params::types::AssetParams;
use mars_red_bank_types::oracle::{self, ActionKind, PriceResponse};

pub struct MarsQuerier<'a> {
    querier: &'a QuerierWrapper<'a>,
    oracle_addr: &'a Addr,
    params_addr: &'a Addr,
}

impl<'a> MarsQuerier<'a> {
    pub fn new(querier: &'a QuerierWrapper, oracle_addr: &'a Addr, params_addr: &'a Addr) -> Self {
        MarsQuerier {
            querier,
            oracle_addr,
            params_addr,
        }
    }

    pub fn query_asset_params(&self, denom: &str) -> StdResult<AssetParams> {
        self.querier.query_wasm_smart(
            self.params_addr,
            &mars_params::msg::QueryMsg::AssetParams {
                denom: denom.to_string(),
            },
        )
    }

    pub fn query_price(&self, denom: &str) -> StdResult<Decimal> {
        let PriceResponse {
            price,
            ..
        } = self.querier.query_wasm_smart(
            self.oracle_addr,
            &oracle::QueryMsg::Price {
                denom: denom.to_string(),
                kind: Some(ActionKind::Default),
            },
        )?;
        Ok(price)
    }
}
