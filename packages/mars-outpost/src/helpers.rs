use std::convert::TryInto;

use cosmwasm_std::{
    coins, to_binary, Addr, Api, BankMsg, CosmosMsg, Decimal, QuerierWrapper, QueryRequest,
    StdError, StdResult, Uint128, WasmQuery,
};
use cw20::{BalanceResponse, Cw20QueryMsg, TokenInfoResponse};

use crate::error::MarsError;

// CW20
pub fn cw20_get_balance(
    querier: &QuerierWrapper,
    token_address: Addr,
    balance_address: Addr,
) -> StdResult<Uint128> {
    let res: BalanceResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: token_address.into(),
        msg: to_binary(&Cw20QueryMsg::Balance {
            address: balance_address.into(),
        })?,
    }))?;

    Ok(res.balance)
}

pub fn cw20_get_total_supply(querier: &QuerierWrapper, token_address: Addr) -> StdResult<Uint128> {
    let res: TokenInfoResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: token_address.into(),
        msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
    }))?;
    Ok(res.total_supply)
}

pub fn build_send_coin_msg(recipient_addr: &Addr, denom: &str, amount: Uint128) -> CosmosMsg {
    CosmosMsg::Bank(BankMsg::Send {
        to_address: recipient_addr.into(),
        amount: coins(amount.u128(), denom),
    })
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
        Err(err) => Err(StdError::generic_err(format!("Error converting slice to array: {}", err))),
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

pub fn decimal_param_le_one(param_value: Decimal, param_name: &str) -> Result<(), MarsError> {
    if !param_value.le(&Decimal::one()) {
        Err(MarsError::InvalidParam {
            param_name: param_name.to_string(),
            invalid_value: param_value.to_string(),
            predicate: "<= 1".to_string(),
        })
    } else {
        Ok(())
    }
}

pub fn zero_address() -> Addr {
    Addr::unchecked("")
}
