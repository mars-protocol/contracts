use std::cmp::min;

use cosmwasm_std::{Coin, DepsMut, Env, Response, Uint128};

use mars_rover::adapters::vault::{
    UnlockingChange, UnlockingPositions, UpdateType, Vault, VaultPositionAmount,
    VaultPositionUpdate,
};
use mars_rover::error::ContractResult;

use crate::liquidate_coin::{calculate_liquidation, repay_debt};
use crate::state::VAULT_POSITIONS;
use crate::utils::update_balance_msg;
use crate::vault::update_vault_position;

pub fn liquidate_vault(
    deps: DepsMut,
    env: Env,
    liquidator_account_id: &str,
    liquidatee_account_id: &str,
    debt_coin: Coin,
    request_vault: Vault,
) -> ContractResult<Response> {
    let liquidatee_position = VAULT_POSITIONS.load(
        deps.storage,
        (liquidatee_account_id, request_vault.address.clone()),
    )?;

    match liquidatee_position {
        VaultPositionAmount::Unlocked(a) => liquidate_unlocked(
            deps,
            env,
            liquidator_account_id,
            liquidatee_account_id,
            debt_coin,
            request_vault,
            a.total(),
        ),
        VaultPositionAmount::Locking(ref a) => {
            // A locking vault can have two different positions: LOCKED & UNLOCKING
            // Priority goes to force withdrawing the unlocking buckets
            if !a.unlocking.positions().is_empty() {
                liquidate_unlocking(
                    deps,
                    env,
                    liquidator_account_id,
                    liquidatee_account_id,
                    debt_coin,
                    request_vault,
                    liquidatee_position.unlocking(),
                )
            } else {
                liquidate_locked(
                    deps,
                    env,
                    liquidator_account_id,
                    liquidatee_account_id,
                    debt_coin,
                    request_vault,
                    a.locked.total(),
                )
            }
        }
    }
}

fn liquidate_unlocked(
    deps: DepsMut,
    env: Env,
    liquidator_account_id: &str,
    liquidatee_account_id: &str,
    debt_coin: Coin,
    request_vault: Vault,
    amount: Uint128,
) -> ContractResult<Response> {
    let vault_info = request_vault.query_info(&deps.querier)?;

    let (debt, request) = calculate_liquidation(
        &deps,
        &env,
        liquidatee_account_id,
        &debt_coin,
        &vault_info.vault_token,
        amount,
    )?;

    let repay_msg = repay_debt(
        deps.storage,
        &env,
        liquidator_account_id,
        liquidatee_account_id,
        &debt,
    )?;

    update_vault_position(
        deps.storage,
        liquidatee_account_id,
        &request_vault.address,
        VaultPositionUpdate::Unlocked(UpdateType::Decrement(request.amount)),
    )?;

    let vault_withdraw_msg = request_vault.withdraw_msg(&deps.querier, request.amount, false)?;

    let update_coin_balance_msg = update_balance_msg(
        &deps.querier,
        &env.contract.address,
        liquidator_account_id,
        &vault_info.base_token,
    )?;

    Ok(Response::new()
        .add_message(repay_msg)
        .add_message(vault_withdraw_msg)
        .add_message(update_coin_balance_msg)
        .add_attribute("action", "rover/credit-manager/liquidate_vault/unlocked")
        .add_attribute("liquidatee_account_id", liquidatee_account_id)
        .add_attribute("debt_repaid_denom", debt.denom)
        .add_attribute("debt_repaid_amount", debt.amount)
        .add_attribute("vault_coin_denom", request.denom)
        .add_attribute("vault_coin_liquidated", request.amount))
}

fn liquidate_unlocking(
    deps: DepsMut,
    env: Env,
    liquidator_account_id: &str,
    liquidatee_account_id: &str,
    debt_coin: Coin,
    request_vault: Vault,
    unlocking_positions: UnlockingPositions,
) -> ContractResult<Response> {
    let vault_info = request_vault.query_info(&deps.querier)?;

    let (debt, request) = calculate_liquidation(
        &deps,
        &env,
        liquidatee_account_id,
        &debt_coin,
        &vault_info.base_token,
        unlocking_positions.total(),
    )?;

    let repay_msg = repay_debt(
        deps.storage,
        &env,
        liquidator_account_id,
        liquidatee_account_id,
        &debt,
    )?;

    let mut total_to_liquidate = request.amount;
    let mut vault_withdraw_msgs = vec![];

    for u in unlocking_positions.positions() {
        let amount = min(u.coin.amount, total_to_liquidate);

        if amount.is_zero() {
            break;
        }

        update_vault_position(
            deps.storage,
            liquidatee_account_id,
            &request_vault.address,
            VaultPositionUpdate::Unlocking(UnlockingChange::Decrement { id: u.id, amount }),
        )?;

        let msg = request_vault.force_withdraw_unlocking_msg(u.id, Some(amount))?;
        vault_withdraw_msgs.push(msg);

        total_to_liquidate = total_to_liquidate.checked_sub(amount)?;
    }

    let update_coin_balance_msg = update_balance_msg(
        &deps.querier,
        &env.contract.address,
        liquidator_account_id,
        &vault_info.base_token,
    )?;

    Ok(Response::new()
        .add_message(repay_msg)
        .add_messages(vault_withdraw_msgs)
        .add_message(update_coin_balance_msg)
        .add_attribute("action", "rover/credit-manager/liquidate_vault/unlocking")
        .add_attribute("liquidatee_account_id", liquidatee_account_id)
        .add_attribute("debt_repaid_denom", debt.denom)
        .add_attribute("debt_repaid_amount", debt.amount)
        .add_attribute("vault_coin_denom", request.denom)
        .add_attribute("vault_coin_liquidated", request.amount))
}

fn liquidate_locked(
    deps: DepsMut,
    env: Env,
    liquidator_account_id: &str,
    liquidatee_account_id: &str,
    debt_coin: Coin,
    request_vault: Vault,
    amount: Uint128,
) -> ContractResult<Response> {
    let vault_info = request_vault.query_info(&deps.querier)?;

    let (debt, request) = calculate_liquidation(
        &deps,
        &env,
        liquidatee_account_id,
        &debt_coin,
        &vault_info.vault_token,
        amount,
    )?;

    let repay_msg = repay_debt(
        deps.storage,
        &env,
        liquidator_account_id,
        liquidatee_account_id,
        &debt,
    )?;

    update_vault_position(
        deps.storage,
        liquidatee_account_id,
        &request_vault.address,
        VaultPositionUpdate::Locked(UpdateType::Decrement(request.amount)),
    )?;

    let vault_withdraw_msg = request_vault.withdraw_msg(&deps.querier, request.amount, true)?;

    let update_coin_balance_msg = update_balance_msg(
        &deps.querier,
        &env.contract.address,
        liquidator_account_id,
        &vault_info.base_token,
    )?;

    Ok(Response::new()
        .add_message(repay_msg)
        .add_message(vault_withdraw_msg)
        .add_message(update_coin_balance_msg)
        .add_attribute("action", "rover/credit-manager/liquidate_vault/locked")
        .add_attribute("liquidatee_account_id", liquidatee_account_id)
        .add_attribute("debt_repaid_denom", debt.denom)
        .add_attribute("debt_repaid_amount", debt.amount)
        .add_attribute("vault_coin_denom", request.denom)
        .add_attribute("vault_coin_liquidated", request.amount))
}
