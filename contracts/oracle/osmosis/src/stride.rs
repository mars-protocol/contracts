use cosmwasm_std::{to_binary, Addr, Decimal, QuerierWrapper, QueryRequest, StdResult, WasmQuery};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// TODO: should be updated once Stride open source their contract

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, JsonSchema)]
pub struct Price {
    pub denom: String,
    pub base_denom: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, JsonSchema)]
pub struct RedemptionRateRequest {
    pub price: Price,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, JsonSchema)]
pub struct RedemptionRateResponse {
    pub exchange_rate: Decimal,
    pub last_updated: u64,
}

/// How much base_denom we get for 1 denom
///
/// Example:
/// denom: stAtom, base_denom: Atom
/// exchange_rate: 1.0211
/// 1 stAtom = 1.0211 Atom
pub fn query_redemption_rate(
    querier: &QuerierWrapper,
    contract_addr: Addr,
    denom: String,
    base_denom: String,
) -> StdResult<RedemptionRateResponse> {
    let redemption_rate_response = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract_addr.into_string(),
        msg: to_binary(&RedemptionRateRequest {
            price: Price {
                denom,
                base_denom,
            },
        })?,
    }))?;
    Ok(redemption_rate_response)
}
