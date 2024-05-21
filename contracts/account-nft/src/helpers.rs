use crate::error::ContractError;

/// token_id follows Twitter username/handle rules:
/// - must be more than 4 characters long and can be up to 15 characters or less,
/// - can contain only letters, numbers, and underscores — no spaces are allowed.
/// We won't to overlap with automatic generation of token_id (via `next_id` variable), so we add
/// additional rule to ensure uniqueness of token_id:
/// - should contain at least one letter.
pub fn validate_token_id(token_id: &str) -> Result<(), ContractError> {
    if token_id.len() < 4 || token_id.len() > 15 {
        return Err(ContractError::InvalidTokenId {
            reason: "token_id length should be between 4 and 15 chars".to_string(),
        });
    }

    let mut contains_letter = false;
    let chars = token_id.chars();
    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '_') {
            return Err(ContractError::InvalidTokenId {
                reason: "token_id can contain only letters, numbers, and underscores".to_string(),
            });
        }

        if c.is_alphabetic() {
            contains_letter = true;
        }
    }

    if !contains_letter {
        return Err(ContractError::InvalidTokenId {
            reason: "token_id should contain at least one letter".to_string(),
        });
    }

    Ok(())
}
