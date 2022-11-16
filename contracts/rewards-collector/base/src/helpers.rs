use cosmwasm_std::{Addr, QuerierWrapper, Uint128};

use crate::{ContractError, ContractResult};

/// For a denom with an optional Uint128 amount,
/// - if the amount is provided, assert that it is no larger than the available balance;
/// - if not provided, use the available balance as default.
pub(crate) fn unwrap_option_amount(
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
pub(crate) fn stringify_option_amount(amount: Option<Uint128>) -> String {
    amount.map_or_else(|| "undefined".to_string(), |amount| amount.to_string())
}

/// follows cosmos SDK validation logic where denoms can be 3 - 128 characters long
/// and support letters, followed but either a letter, number, or separator ( ‘/' , ‘:' , ‘.’ , ‘_’ , or '-')
/// reference: https://github.com/cosmos/cosmos-sdk/blob/7728516abfab950dc7a9120caad4870f1f962df5/types/coin.go#L865-L867
pub(crate) fn validate_native_denom_contract(denom: &str) -> Result<(), ContractError> {
    if denom.len() < 3 || denom.len() > 128 {
        return Err(ContractError::InvalidDenom {
            reason: "Invalid denom length".to_string(),
        });
    }

    let mut chars = denom.chars();
    let first = chars.next().ok_or(ContractError::InvalidDenom {
        reason: "Cannot retrieve first character".to_string(),
    })?;
    if !first.is_ascii_alphabetic() {
        return Err(ContractError::InvalidDenom {
            reason: "First character is not ASCII alphabetic".to_string(),
        });
    }

    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '/' || c == ':' || c == '.' || c == '_' || c == '-')
        {
            return Err(ContractError::InvalidDenom {
                reason: "Not all characters are ASCII alphanumeric or one of:  /  :  .  _  -"
                    .to_string(),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::coin;
    use cosmwasm_std::testing::{mock_dependencies_with_balance, MOCK_CONTRACT_ADDR};

    #[test]
    fn unwrapping_option_amount() {
        let deps = mock_dependencies_with_balance(&[
            coin(88888, "uatom"),
            coin(1234, "uusdc"),
            coin(8964, "umars"),
        ]);

        assert_eq!(
            unwrap_option_amount(
                &deps.as_ref().querier,
                &Addr::unchecked(MOCK_CONTRACT_ADDR),
                "uatom",
                None
            ),
            Ok(Uint128::new(88888))
        );
        assert_eq!(
            unwrap_option_amount(
                &deps.as_ref().querier,
                &Addr::unchecked(MOCK_CONTRACT_ADDR),
                "uatom",
                Some(Uint128::new(12345))
            ),
            Ok(Uint128::new(12345))
        );
        assert_eq!(
            unwrap_option_amount(
                &deps.as_ref().querier,
                &Addr::unchecked(MOCK_CONTRACT_ADDR),
                "uatom",
                Some(Uint128::new(99999))
            ),
            Err(ContractError::AmountToDistributeTooLarge {
                amount: Uint128::new(99999),
                balance: Uint128::new(88888),
            })
        );
    }

    #[test]
    fn stringifying_option_amount() {
        assert_eq!(stringify_option_amount(Some(Uint128::new(42069))), "42069".to_string());
        assert_eq!(stringify_option_amount(None), "undefined".to_string());
    }
}
