use std::cmp::min;

use cosmwasm_std::{
    to_binary, Coin, CosmosMsg, DepsMut, Env, QuerierWrapper, Response, Storage, Uint128, WasmMsg,
};

use rover::adapters::vault::{
    Total, UnlockingChange, UpdateType, Vault, VaultPositionAmount, VaultPositionUpdate,
};
use rover::error::ContractResult;
use rover::msg::execute::CallbackMsg;
use rover::msg::ExecuteMsg;
use rover::traits::Denoms;

use crate::liquidate_coin::calculate_liquidation;
use crate::state::VAULT_POSITIONS;
use crate::update_coin_balances::query_balances;
use crate::utils::{decrement_coin_balance, increment_coin_balance};
use crate::vault::update_vault_position;

pub fn liquidate_vault(
    deps: DepsMut,
    env: Env,
    liquidator_account_id: &str,
    liquidatee_account_id: &str,
    debt_coin: Coin,
    request_vault: Vault,
) -> ContractResult<Response> {
    let vault_info = request_vault.query_info(&deps.querier)?;
    let liquidatee_position = VAULT_POSITIONS.load(
        deps.storage,
        (liquidatee_account_id, request_vault.address.clone()),
    )?;
    let (debt, request) = calculate_liquidation(
        &deps,
        &env,
        liquidatee_account_id,
        &debt_coin,
        &vault_info.token_denom,
        liquidatee_position.total(),
    )?;

    // Transfer debt coin from liquidator's coin balance to liquidatee
    // Will be used to pay off the debt via CallbackMsg::Repay {}
    decrement_coin_balance(deps.storage, liquidator_account_id, &debt)?;
    increment_coin_balance(deps.storage, liquidatee_account_id, &debt)?;
    let repay_msg = (CallbackMsg::Repay {
        account_id: liquidatee_account_id.to_string(),
        coin: debt.clone(),
    })
    .into_cosmos_msg(&env.contract.address)?;

    let vault_withdraw_msgs = get_vault_withdraw_msgs(
        deps.storage,
        &deps.querier,
        liquidatee_account_id,
        &request_vault,
        &liquidatee_position,
        request.amount,
    )?;

    // Update coin balances of liquidator after withdraws have been made
    let coins_from_withdraw = request_vault.query_preview_redeem(&deps.querier, request.amount)?;
    let previous_balances = query_balances(
        deps.as_ref(),
        &env.contract.address,
        &coins_from_withdraw.to_denoms(),
    )?;
    let update_coin_balance_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        funds: vec![],
        msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::UpdateCoinBalances {
            account_id: liquidator_account_id.to_string(),
            previous_balances,
        }))?,
    });

    Ok(Response::new()
        .add_message(repay_msg)
        .add_messages(vault_withdraw_msgs)
        .add_message(update_coin_balance_msg)
        .add_attribute("action", "rover/credit_manager/liquidate_vault")
        .add_attribute("liquidatee_account_id", liquidatee_account_id)
        .add_attribute("debt_repaid_denom", debt.denom)
        .add_attribute("debt_repaid_amount", debt.amount)
        .add_attribute("vault_coin_denom", request.denom)
        .add_attribute("vault_coin_liquidated", request.amount))
}

/// Generates Cosmos msgs for Vault withdraws & updates Rover credit account balances
fn get_vault_withdraw_msgs(
    storage: &mut dyn Storage,
    querier: &QuerierWrapper,
    liquidatee_account_id: &str,
    request_vault: &Vault,
    liquidatee_position: &VaultPositionAmount,
    amount: Uint128,
) -> ContractResult<Vec<CosmosMsg>> {
    let mut total_to_liquidate = amount;
    let mut vault_withdraw_msgs = vec![];

    match liquidatee_position {
        VaultPositionAmount::Unlocked(_) => {
            update_vault_position(
                storage,
                liquidatee_account_id,
                &request_vault.address,
                VaultPositionUpdate::Unlocked(UpdateType::Decrement(total_to_liquidate)),
            )?;

            let msg = request_vault.withdraw_msg(querier, total_to_liquidate, false)?;
            vault_withdraw_msgs.push(msg);
        }
        VaultPositionAmount::Locking(amount) => {
            // A locking vault can have two different positions: LOCKED & UNLOCKING
            // Priority goes to force withdrawing the unlocking buckets
            for u in amount.unlocking.positions() {
                let amount = min(u.amount, total_to_liquidate);

                update_vault_position(
                    storage,
                    liquidatee_account_id,
                    &request_vault.address,
                    VaultPositionUpdate::Unlocking(UnlockingChange::Decrement { id: u.id, amount }),
                )?;

                let msg = request_vault.force_withdraw_unlocking_msg(u.id, Some(amount))?;
                vault_withdraw_msgs.push(msg);

                total_to_liquidate = total_to_liquidate.checked_sub(amount)?;
            }

            // If unlocking positions have been exhausted, liquidate from LOCKED bucket
            if !total_to_liquidate.is_zero() {
                update_vault_position(
                    storage,
                    liquidatee_account_id,
                    &request_vault.address,
                    VaultPositionUpdate::Locked(UpdateType::Decrement(total_to_liquidate)),
                )?;

                let msg = request_vault.withdraw_msg(querier, total_to_liquidate, true)?;
                vault_withdraw_msgs.push(msg);
            }
        }
    }

    Ok(vault_withdraw_msgs)
}
