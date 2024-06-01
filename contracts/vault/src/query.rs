use cosmwasm_std::{Addr, Deps};

use crate::{
    contract::Vault, error::ContractResult, execute::total_base_token_in_account, msg::VaultUnlock,
    state::UNLOCKS,
};

pub fn unlocks(deps: Deps, user_addr: Addr) -> ContractResult<Vec<VaultUnlock>> {
    let vault = Vault::default();
    let vault_token_supply = vault.vault_token.load(deps.storage)?.query_total_supply(deps)?;
    let total_staked_amount = total_base_token_in_account(deps)?;

    let unlocks = UNLOCKS.may_load(deps.storage, user_addr.to_string())?.unwrap_or_default();
    unlocks
        .into_iter()
        .map(|unlock| {
            let base_tokens = vault.calculate_base_tokens(
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
