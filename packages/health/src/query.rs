use cosmwasm_std::{Addr, Decimal, QuerierWrapper, StdResult};
use mars_types::{
    oracle::{self, ActionKind, PriceResponse},
    params::AssetParams,
};

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
            &mars_types::params::QueryMsg::AssetParams {
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

    pub fn query_price_for_liquidate(&self, denom: &str) -> StdResult<Decimal> {
        let PriceResponse {
            price,
            ..
        } = self.querier.query_wasm_smart(
            self.oracle_addr,
            &oracle::QueryMsg::Price {
                denom: denom.to_string(),
                kind: Some(ActionKind::Liquidation),
            },
        )?;
        Ok(price)
    }
}
