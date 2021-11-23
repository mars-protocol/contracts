use crate::math::decimal::Decimal;
use cosmwasm_std::{to_binary, Binary, ContractResult, QuerierResult, Uint128};
use std::collections::HashMap;
use terra_cosmwasm::{
    ExchangeRateItem, ExchangeRatesResponse, TaxCapResponse, TaxRateResponse, TerraQuery,
    TerraRoute,
};

pub struct NativeQuerier {
    /// maps denom to exchange rates
    pub exchange_rates: HashMap<String, HashMap<String, Decimal>>,
    /// maps denom to tax caps
    pub tax_caps: HashMap<String, Uint128>,
    pub tax_rate: Decimal,
}

impl Default for NativeQuerier {
    fn default() -> Self {
        NativeQuerier {
            exchange_rates: HashMap::new(),
            tax_caps: HashMap::new(),
            tax_rate: Decimal::zero(),
        }
    }
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

            TerraRoute::Treasury => {
                let ret: ContractResult<Binary> = match query_data {
                    TerraQuery::TaxRate {} => {
                        let res = TaxRateResponse {
                            rate: self.tax_rate.to_std_decimal(),
                        };
                        to_binary(&res).into()
                    }

                    TerraQuery::TaxCap { denom } => match self.tax_caps.get(denom) {
                        Some(cap) => {
                            let res = TaxCapResponse { cap: *cap };
                            to_binary(&res).into()
                        }
                        None => Err(format!(
                            "no tax cap available for provided denom: {}",
                            denom
                        ))
                        .into(),
                    },

                    _ => Err(format!(
                        "[mock]: Unsupported query data for QueryRequest::Custom : {:?}",
                        query_data
                    ))
                    .into(),
                };

                Ok(ret).into()
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
