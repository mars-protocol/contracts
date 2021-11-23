use cosmwasm_std::{
    to_binary, Addr, Api, QuerierWrapper, QueryRequest, StdError, StdResult, Uint128, WasmQuery,
};

use crate::error::MarsError;
use cw20::{BalanceResponse, Cw20QueryMsg, TokenInfoResponse};
use std::convert::TryInto;

// CW20
pub fn cw20_get_balance(
    querier: &QuerierWrapper,
    token_address: Addr,
    balance_address: Addr,
) -> StdResult<Uint128> {
    let query: BalanceResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: token_address.into(),
        msg: to_binary(&Cw20QueryMsg::Balance {
            address: balance_address.into(),
        })?,
    }))?;

    Ok(query.balance)
}

pub fn cw20_get_total_supply(querier: &QuerierWrapper, token_address: Addr) -> StdResult<Uint128> {
    let query = cw20_get_info(querier, token_address)?;
    Ok(query.total_supply)
}

pub fn cw20_get_symbol(querier: &QuerierWrapper, token_address: Addr) -> StdResult<String> {
    let query = cw20_get_info(querier, token_address)?;
    Ok(query.symbol)
}

pub fn cw20_get_info(
    querier: &QuerierWrapper,
    token_address: Addr,
) -> StdResult<TokenInfoResponse> {
    let query: TokenInfoResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: token_address.into(),
        msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
    }))?;

    Ok(query)
}

pub fn read_be_u64(input: &[u8]) -> StdResult<u64> {
    let num_of_bytes = std::mem::size_of::<u64>();
    if input.len() != num_of_bytes {
        return Err(StdError::generic_err(format!(
            "Expected slice length to be {}, received length of {}",
            num_of_bytes,
            input.len()
        )));
    };
    let slice_to_array_result = input[0..num_of_bytes].try_into();

    match slice_to_array_result {
        Ok(array) => Ok(u64::from_be_bytes(array)),
        Err(err) => Err(StdError::generic_err(format!(
            "Error converting slice to array: {}",
            err
        ))),
    }
}

/// Used when unwrapping an optional address sent in a contract call by a user.
/// Validates addreess if present, otherwise uses a given default value.
pub fn option_string_to_addr(
    api: &dyn Api,
    option_string: Option<String>,
    default: Addr,
) -> StdResult<Addr> {
    match option_string {
        Some(input_addr) => api.addr_validate(&input_addr),
        None => Ok(default),
    }
}

/// Verify if all conditions are met. If not return list of invalid params.
pub fn all_conditions_valid(conditions_and_names: Vec<(bool, &str)>) -> Result<(), MarsError> {
    // All params which should meet criteria
    let param_names: Vec<_> = conditions_and_names.iter().map(|elem| elem.1).collect();
    // Filter params which don't meet criteria
    let invalid_params: Vec<_> = conditions_and_names
        .into_iter()
        .filter(|elem| !elem.0)
        .map(|elem| elem.1)
        .collect();
    if !invalid_params.is_empty() {
        return Err(MarsError::ParamsNotLessOrEqualOne {
            expected_params: param_names.join(", "),
            invalid_params: invalid_params.join(", "),
        });
    }

    Ok(())
}

pub fn zero_address() -> Addr {
    Addr::unchecked("")
}
