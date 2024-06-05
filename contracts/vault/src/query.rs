use cosmwasm_std::{Addr, Deps, Uint128};

use crate::{
    error::ContractResult,
    execute::total_base_tokens_in_account,
    msg::{VaultInfoResponseExt, VaultUnlock},
    state::{
        BASE_TOKEN, COOLDOWN_PERIOD, CREDIT_MANAGER, DESCRIPTION, PERFORMANCE_FEE_CONFIG, SUBTITLE,
        TITLE, UNLOCKS, VAULT_ACC_ID, VAULT_TOKEN,
    },
    vault_token::{calculate_base_tokens, calculate_vault_tokens},
};

pub fn query_vault_info(deps: Deps) -> ContractResult<VaultInfoResponseExt> {
    Ok(VaultInfoResponseExt {
        base_token: BASE_TOKEN.load(deps.storage)?,
        vault_token: VAULT_TOKEN.load(deps.storage)?.to_string(),
        title: TITLE.may_load(deps.storage)?,
        subtitle: SUBTITLE.may_load(deps.storage)?,
        description: DESCRIPTION.may_load(deps.storage)?,
        credit_manager: CREDIT_MANAGER.load(deps.storage)?,
        vault_account_id: VAULT_ACC_ID.may_load(deps.storage)?,
        cooldown_period: COOLDOWN_PERIOD.load(deps.storage)?,
        performance_fee_config: PERFORMANCE_FEE_CONFIG.load(deps.storage)?,
    })
}

pub fn query_user_unlocks(deps: Deps, user_addr: Addr) -> ContractResult<Vec<VaultUnlock>> {
    let vault_token_supply = VAULT_TOKEN.load(deps.storage)?.query_total_supply(deps)?;
    let total_staked_amount = total_base_tokens_in_account(deps)?;

    let unlocks = UNLOCKS.may_load(deps.storage, user_addr.to_string())?.unwrap_or_default();
    unlocks
        .into_iter()
        .map(|unlock| {
            let base_tokens = calculate_base_tokens(
                unlock.vault_tokens,
                total_staked_amount,
                vault_token_supply,
            )?;
            Ok(VaultUnlock {
                created_at: unlock.created_at,
                cooldown_end: unlock.cooldown_end,
                vault_tokens: unlock.vault_tokens,
                base_tokens,
            })
        })
        .collect()
}

pub fn convert_to_vault_tokens(deps: Deps, amount: Uint128) -> ContractResult<Uint128> {
    let vault_token_supply = VAULT_TOKEN.load(deps.storage)?.query_total_supply(deps)?;
    let total_staked_amount = total_base_tokens_in_account(deps)?;
    Ok(calculate_vault_tokens(amount, total_staked_amount, vault_token_supply)?)
}

pub fn convert_to_base_tokens(deps: Deps, amount: Uint128) -> ContractResult<Uint128> {
    let vault_token_supply = VAULT_TOKEN.load(deps.storage)?.query_total_supply(deps)?;
    let total_staked_amount = total_base_tokens_in_account(deps)?;
    Ok(calculate_base_tokens(amount, total_staked_amount, vault_token_supply)?)
}
