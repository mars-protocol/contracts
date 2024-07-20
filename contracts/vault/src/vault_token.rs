use cosmwasm_std::{StdError, Uint128};

pub const DEFAULT_VAULT_TOKENS_PER_STAKED_BASE_TOKEN: Uint128 = Uint128::new(1_000_000);

pub fn calculate_vault_tokens(
    base_tokens: Uint128,
    total_base_tokens: Uint128,
    vault_token_supply: Uint128,
) -> Result<Uint128, StdError> {
    let vault_tokens = if total_base_tokens.is_zero() {
        base_tokens.checked_mul(DEFAULT_VAULT_TOKENS_PER_STAKED_BASE_TOKEN)?
    } else {
        vault_token_supply.multiply_ratio(base_tokens, total_base_tokens)
    };

    Ok(vault_tokens)
}

pub fn calculate_base_tokens(
    vault_tokens: Uint128,
    total_base_tokens: Uint128,
    vault_token_supply: Uint128,
) -> Result<Uint128, StdError> {
    let base_tokens = if vault_token_supply.is_zero() {
        vault_tokens.checked_div(DEFAULT_VAULT_TOKENS_PER_STAKED_BASE_TOKEN)?
    } else {
        total_base_tokens.multiply_ratio(vault_tokens, vault_token_supply)
    };

    Ok(base_tokens)
}
