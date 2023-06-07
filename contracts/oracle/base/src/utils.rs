use crate::ContractError;

/// follows cosmos SDK validation logic where denoms can be 3 - 128 characters long
/// and starts with a letter, followed but either a letter, number, or separator ( ‘/' , ‘:' , ‘.’ , ‘_’ , or '-')
/// reference: https://github.com/cosmos/cosmos-sdk/blob/7728516abfab950dc7a9120caad4870f1f962df5/types/coin.go#L865-L867
pub fn validate_native_denom(denom: &str) -> Result<(), ContractError> {
    if denom.len() < 3 || denom.len() > 128 {
        return Err(ContractError::InvalidDenom {
            reason: "Invalid denom length".to_string(),
        });
    }

    let mut chars = denom.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() {
        return Err(ContractError::InvalidDenom {
            reason: "First character is not ASCII alphabetic".to_string(),
        });
    }

    let set = ['/', ':', '.', '_', '-'];
    for c in chars {
        if !(c.is_ascii_alphanumeric() || set.contains(&c)) {
            return Err(ContractError::InvalidDenom {
                reason: "Not all characters are ASCII alphanumeric or one of:  /  :  .  _  -"
                    .to_string(),
            });
        }
    }

    Ok(())
}
