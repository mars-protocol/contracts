use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Addr, QuerierWrapper, QueryRequest, StdResult, WasmQuery};
use ica_oracle::msg::{QueryMsg, RedemptionRateResponse};

use crate::{ContractError, ContractError::InvalidPrice};

#[cw_serde]
pub struct RedemptionRate<T> {
    /// Contract addr
    pub contract_addr: T,

    /// The maximum number of seconds since the last price was by an oracle, before
    /// rejecting the price as too stale
    pub max_staleness: u64,
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
) -> StdResult<RedemptionRateResponse> {
    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract_addr.into_string(),
        msg: to_json_binary(&QueryMsg::RedemptionRate {
            denom,
            params: None,
        })?,
    }))
}

/// Redemption rate comes from different chain (Stride) and it can be greater than the current block time due to differences in block generation times,
/// network latency, and the asynchronous nature of cross-chain data updates. We accept such case as valid RR.
pub fn assert_rr_not_too_old(
    current_time: u64,
    rr_res: &RedemptionRateResponse,
    rr_config: &RedemptionRate<Addr>,
) -> Result<(), ContractError> {
    if rr_res.update_time + rr_config.max_staleness < current_time {
        return Err(InvalidPrice {
            reason: format!(
                "redemption rate update time is too old/stale. last updated: {}, now: {}",
                rr_res.update_time, current_time
            ),
        });
    }
    Ok(())
}
