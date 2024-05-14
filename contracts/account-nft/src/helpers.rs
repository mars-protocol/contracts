use crate::error::ContractError;

/// token_id follows Twitter username/handle rules:
/// - must be more than 4 characters long and can be up to 15 characters or less,
/// - can contain only letters, numbers, and underscores — no spaces are allowed.
pub fn validate_token_id(token_id: &str) -> Result<(), ContractError> {
    if token_id.len() < 4 || token_id.len() > 15 {
        return Err(ContractError::InvalidTokenId {
            reason: "token_id length should be between 4 and 15 chars".to_string(),
        });
    }

    let chars = token_id.chars();
    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '_') {
            return Err(ContractError::InvalidTokenId {
                reason: "token_id can contain only letters, numbers, and underscores".to_string(),
            });
        }
    }

    Ok(())
}
