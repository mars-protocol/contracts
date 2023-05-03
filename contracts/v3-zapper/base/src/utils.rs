use cosmwasm_std::{Coin, MessageInfo};

use crate::error::{ContractError, ContractResult};

/// Assert that fund of exactly the same type and amount was sent along with a message
pub fn assert_exact_funds_sent(info: &MessageInfo, expected: &[Coin]) -> ContractResult<()> {
    let same_quantity = info.funds.len() == expected.len();
    let all_expected_in_funds = expected.iter().all(|e| info.funds.iter().any(|i| e == i));

    if !same_quantity || !all_expected_in_funds {
        return Err(ContractError::FundsMismatch {
            expected: expected.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(", "),
            received: info.funds.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(", "),
        });
    }

    Ok(())
}
