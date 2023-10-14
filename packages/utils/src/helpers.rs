use cosmwasm_std::{coins, Addr, Api, BankMsg, CosmosMsg, Decimal, StdResult, Uint128};

use crate::error::ValidationError;

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

pub fn decimal_param_lt_one(param_value: Decimal, param_name: &str) -> Result<(), ValidationError> {
    if !param_value.lt(&Decimal::one()) {
        Err(ValidationError::InvalidParam {
            param_name: param_name.to_string(),
            invalid_value: param_value.to_string(),
            predicate: "< 1".to_string(),
        })
    } else {
        Ok(())
    }
}

pub fn decimal_param_le_one(param_value: Decimal, param_name: &str) -> Result<(), ValidationError> {
    if !param_value.le(&Decimal::one()) {
        Err(ValidationError::InvalidParam {
            param_name: param_name.to_string(),
            invalid_value: param_value.to_string(),
            predicate: "<= 1".to_string(),
        })
    } else {
        Ok(())
    }
}

pub fn integer_param_gt_zero(param_value: u64, param_name: &str) -> Result<(), ValidationError> {
    if !param_value.gt(&0) {
        Err(ValidationError::InvalidParam {
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

/// follows cosmos SDK validation logic where denoms can be 3 - 128 characters long
/// and starts with a letter, followed but either a letter, number, or separator ( ‘/' , ‘:' , ‘.’ , ‘_’ , or '-')
/// reference: https://github.com/cosmos/cosmos-sdk/blob/7728516abfab950dc7a9120caad4870f1f962df5/types/coin.go#L865-L867
pub fn validate_native_denom(denom: &str) -> Result<(), ValidationError> {
    if denom.len() < 3 || denom.len() > 128 {
        return Err(ValidationError::InvalidDenom {
            reason: "Invalid denom length".to_string(),
        });
    }

    let mut chars = denom.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() {
        return Err(ValidationError::InvalidDenom {
            reason: "First character is not ASCII alphabetic".to_string(),
        });
    }

    let set = ['/', ':', '.', '_', '-'];
    for c in chars {
        if !(c.is_ascii_alphanumeric() || set.contains(&c)) {
            return Err(ValidationError::InvalidDenom {
                reason: "Not all characters are ASCII alphanumeric or one of:  /  :  .  _  -"
                    .to_string(),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{error::ValidationError::InvalidDenom, helpers::validate_native_denom};

    #[test]
    fn length_below_three() {
        let res = validate_native_denom("su");
        assert_eq!(
            res,
            Err(InvalidDenom {
                reason: "Invalid denom length".to_string()
            }),
        )
    }

    #[test]
    fn length_above_128() {
        let res =
        validate_native_denom("fadjkvnrufbaalkefoi2934095sfonalf89o234u2sadsafsdbvsdrgweqraefsdgagqawfaf104hqflkqehf98348qfhdsfave3r23152wergfaefegqsacasfasfadvcadfsdsADsfaf324523");
        assert_eq!(
            res,
            Err(InvalidDenom {
                reason: "Invalid denom length".to_string()
            }),
        )
    }

    #[test]
    fn first_char_not_alphabetical() {
        let res = validate_native_denom("7asdkjnfe7");
        assert_eq!(
            res,
            Err(InvalidDenom {
                reason: "First character is not ASCII alphabetic".to_string()
            }),
        )
    }

    #[test]
    fn invalid_character() {
        let res = validate_native_denom("fakjfh&asd!#");
        assert_eq!(
            res,
            Err(InvalidDenom {
                reason: "Not all characters are ASCII alphanumeric or one of:  /  :  .  _  -"
                    .to_string()
            }),
        )
    }

    #[test]
    fn correct_denom() {
        let res = validate_native_denom("umars");
        assert_eq!(res, Ok(()));

        let res = validate_native_denom(
            "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
        );
        assert_eq!(res, Ok(()));
    }
}
