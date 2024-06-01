use cosmwasm_std::{Addr, Deps, Env};

use crate::{
    contract::Vault,
    error::ContractResult,
    execute::total_base_token_in_account,
    msg::VaultUnlock,
    state::{PerformanceFeeState, PERFORMANCE_FEE_CONFIG, PERFORMANCE_FEE_STATE, UNLOCKS},
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

pub fn performance_fee(deps: Deps, env: Env) -> ContractResult<PerformanceFeeState> {
    let total_staked_amount = total_base_token_in_account(deps)?;

    let mut performance_fee_state = PERFORMANCE_FEE_STATE.load(deps.storage)?;
    let performance_fee_config = PERFORMANCE_FEE_CONFIG.load(deps.storage)?;
    performance_fee_state.update_fee_and_pnl(
        env.block.time.seconds(),
        total_staked_amount,
        &performance_fee_config,
    )?;

    let updated_liquidity = total_staked_amount - performance_fee_state.accumulated_fee;

    Ok(PerformanceFeeState {
        updated_at: performance_fee_state.updated_at,
        liquidity: updated_liquidity,
        accumulated_pnl: performance_fee_state.accumulated_pnl,
        accumulated_fee: performance_fee_state.accumulated_fee,
    })
}
