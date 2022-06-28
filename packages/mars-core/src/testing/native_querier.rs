use crate::math::decimal::Decimal;
use cosmwasm_std::{to_binary, Binary, ContractResult, QuerierResult};
use std::collections::HashMap;
use terra_cosmwasm::{ExchangeRateItem, ExchangeRatesResponse, TerraQuery, TerraRoute};

#[derive(Default)]
pub struct NativeQuerier {
    /// maps denom to exchange rates
    pub exchange_rates: HashMap<String, HashMap<String, Decimal>>,
}

impl NativeQuerier {
    pub fn handle_query(&self, route: &TerraRoute, query_data: &TerraQuery) -> QuerierResult {
        match route {
            TerraRoute::Oracle => {
                if let TerraQuery::ExchangeRates {
                    base_denom,
                    quote_denoms,
                } = query_data
                {
                    return self.query_oracle(base_denom, quote_denoms);
                }
                let err: ContractResult<Binary> = Err(format!(
                    "[mock]: Unsupported query data for QueryRequest::Custom : {:?}",
                    query_data
                ))
                .into();

                Ok(err).into()
            }

            _ => {
                let err: ContractResult<Binary> = Err(format!(
                    "[mock]: Unsupported query data for QueryRequest::Custom : {:?}",
                    query_data
                ))
                .into();

                Ok(err).into()
            }
        }
    }

    fn query_oracle(&self, base_denom: &str, quote_denoms: &[String]) -> QuerierResult {
        let base_exchange_rates = match self.exchange_rates.get(base_denom) {
            Some(res) => res,
            None => {
                let err: ContractResult<Binary> = Err(format!(
                    "no exchange rates available for provided base denom: {}",
                    base_denom
                ))
                .into();
                return Ok(err).into();
            }
        };

        let exchange_rate_items: Result<Vec<ExchangeRateItem>, String> = quote_denoms
            .iter()
            .map(|denom| {
                let exchange_rate = match base_exchange_rates.get(denom) {
                    Some(rate) => rate,
                    None => return Err(format!("no exchange rate available for {}", denom)),
                };

                Ok(ExchangeRateItem {
                    quote_denom: denom.into(),
                    exchange_rate: exchange_rate.to_std_decimal(),
                })
            })
            .collect();

        let res = ExchangeRatesResponse {
            base_denom: base_denom.into(),
            exchange_rates: exchange_rate_items.unwrap(),
        };
        let cr: ContractResult<Binary> = to_binary(&res).into();
        Ok(cr).into()
    }
}
