use cosmwasm_std::{Addr, QuerierWrapper, Uint128};

use crate::{ContractError, ContractResult};

/// For a denom with an optional Uint128 amount,
/// - if the amount is provided, assert that it is no larger than the available balance;
/// - if not provided, use the available balance as default.
pub fn unwrap_option_amount(
    querier: &QuerierWrapper<impl cosmwasm_std::CustomQuery>,
    addr: &Addr,
    denom: &str,
    amount: Option<Uint128>,
) -> ContractResult<Uint128> {
    let balance = querier.query_balance(addr, denom)?.amount;
    if let Some(amount) = amount {
        if amount > balance {
            return Err(ContractError::AmountToDistributeTooLarge {
                amount,
                balance,
            });
        }
        Ok(amount)
    } else {
        Ok(balance)
    }
}

/// Convert an optional Uint128 amount to string. If the amount is undefined, return `undefined`
pub fn stringify_option_amount(amount: Option<Uint128>) -> String {
    amount.map_or_else(|| "undefined".to_string(), |amount| amount.to_string())
}
