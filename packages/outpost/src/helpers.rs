use cosmwasm_std::{coins, Addr, Api, BankMsg, CosmosMsg, Decimal, StdResult, Uint128};

use crate::error::MarsError;

pub fn build_send_asset_msg(recipient_addr: &Addr, denom: &str, amount: Uint128) -> CosmosMsg {
    CosmosMsg::Bank(BankMsg::Send {
        to_address: recipient_addr.into(),
        amount: coins(amount.u128(), denom),
    })
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

pub fn decimal_param_lt_one(param_value: Decimal, param_name: &str) -> Result<(), MarsError> {
    if !param_value.lt(&Decimal::one()) {
        Err(MarsError::InvalidParam {
            param_name: param_name.to_string(),
            invalid_value: param_value.to_string(),
            predicate: "< 1".to_string(),
        })
    } else {
        Ok(())
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

pub fn integer_param_gt_zero(param_value: u64, param_name: &str) -> Result<(), MarsError> {
    if !param_value.gt(&0) {
        Err(MarsError::InvalidParam {
            param_name: param_name.to_string(),
            invalid_value: param_value.to_string(),
            predicate: "> 0".to_string(),
        })
    } else {
        Ok(())
    }
}

pub fn zero_address() -> Addr {
    Addr::unchecked("")
}
