use std::cmp::min;

use cosmwasm_std::{Coin, Decimal, DepsMut, Env, Response, Uint128};
use cw_vault_standard::VaultInfoResponse;
use mars_rover::{
    adapters::vault::{
        UnlockingChange, UnlockingPositions, UpdateType, Vault, VaultPositionAmount,
        VaultPositionType, VaultPositionUpdate,
    },
    error::{ContractError, ContractResult},
};

use crate::{
    liquidate::calculate_liquidation, liquidate_deposit::repay_debt, state::VAULT_POSITIONS,
    utils::update_balance_after_vault_liquidation_msg, vault::update_vault_position,
};

pub fn liquidate_vault(
    deps: DepsMut,
    env: Env,
    liquidator_account_id: &str,
    liquidatee_account_id: &str,
    debt_coin: Coin,
    request_vault: Vault,
    position_type: VaultPositionType,
) -> ContractResult<Response> {
    let liquidatee_position = VAULT_POSITIONS
        .load(deps.storage, (liquidatee_account_id, request_vault.address.clone()))?;

    match liquidatee_position {
        VaultPositionAmount::Unlocked(a) => match position_type {
            VaultPositionType::UNLOCKED => liquidate_unlocked(
                deps,
                env,
                liquidator_account_id,
                liquidatee_account_id,
                debt_coin,
                request_vault,
                a.total(),
            ),
            _ => Err(ContractError::MismatchedVaultType),
        },
        VaultPositionAmount::Locking(ref a) => match position_type {
            VaultPositionType::LOCKED => liquidate_locked(
                deps,
                env,
                liquidator_account_id,
                liquidatee_account_id,
                debt_coin,
                request_vault,
                a.locked.total(),
            ),
            VaultPositionType::UNLOCKING => liquidate_unlocking(
                deps,
                env,
                liquidator_account_id,
                liquidatee_account_id,
                debt_coin,
                request_vault,
                liquidatee_position.unlocking(),
            ),
            _ => Err(ContractError::MismatchedVaultType),
        },
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

    let (debt, liquidator_request, liquidatee_request) = calculate_vault_liquidation(
        &deps,
        liquidatee_account_id,
        &debt_coin,
        &request_vault,
        amount,
        &vault_info,
    )?;

    let repay_msg =
        repay_debt(deps.storage, &env, liquidator_account_id, liquidatee_account_id, &debt)?;

    update_vault_position(
        deps.storage,
        liquidatee_account_id,
        &request_vault.address,
        VaultPositionUpdate::Unlocked(UpdateType::Decrement(liquidatee_request.amount)),
    )?;

    let vault_withdraw_msg =
        request_vault.withdraw_msg(&deps.querier, liquidatee_request.amount)?;

    let protocol_fee = liquidatee_request.amount.checked_sub(liquidator_request.amount)?;
    let protocol_fee_percentage =
        Decimal::checked_from_ratio(protocol_fee, liquidatee_request.amount)?;

    let update_coin_balance_msg = update_balance_after_vault_liquidation_msg(
        &deps.querier,
        &env.contract.address,
        liquidator_account_id,
        &vault_info.base_token,
        protocol_fee_percentage,
    )?;

    Ok(Response::new()
        .add_message(repay_msg)
        .add_message(vault_withdraw_msg)
        .add_message(update_coin_balance_msg)
        .add_attribute("action", "liquidate_vault/unlocked")
        .add_attribute("account_id", liquidator_account_id)
        .add_attribute("liquidatee_account_id", liquidatee_account_id)
        .add_attribute("coin_debt_repaid", debt.to_string())
        .add_attribute("coin_liquidated", liquidatee_request.to_string())
        .add_attribute(
            "protocol_fee_coin",
            Coin::new(protocol_fee.u128(), liquidatee_request.denom).to_string(),
        ))
}

/// Converts vault coins to their underlying value. This allows for pricing and liquidation
/// values to be determined. Afterward, the final amount is converted back into vault coins.
fn calculate_vault_liquidation(
    deps: &DepsMut,
    liquidatee_account_id: &str,
    debt_coin: &Coin,
    request_vault: &Vault,
    amount: Uint128,
    vault_info: &VaultInfoResponse,
) -> ContractResult<(Coin, Coin, Coin)> {
    let total_underlying = request_vault.query_preview_redeem(&deps.querier, amount)?;
    let (debt, mut liquidator_request, mut liquidatee_request) = calculate_liquidation(
        deps,
        liquidatee_account_id,
        debt_coin,
        &vault_info.base_token,
        total_underlying,
    )?;
    liquidatee_request.denom = vault_info.vault_token.clone();
    liquidatee_request.amount =
        amount.checked_multiply_ratio(liquidatee_request.amount, total_underlying)?;
    liquidator_request.denom = vault_info.vault_token.clone();
    liquidator_request.amount =
        amount.checked_multiply_ratio(liquidator_request.amount, total_underlying)?;
    Ok((debt, liquidator_request, liquidatee_request))
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

    let (debt, liquidator_request, liquidatee_request) = calculate_liquidation(
        &deps,
        liquidatee_account_id,
        &debt_coin,
        &vault_info.base_token,
        unlocking_positions.total(),
    )?;

    let repay_msg =
        repay_debt(deps.storage, &env, liquidator_account_id, liquidatee_account_id, &debt)?;

    let mut total_to_liquidate = liquidatee_request.amount;
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
            VaultPositionUpdate::Unlocking(UnlockingChange::Decrement {
                id: u.id,
                amount,
            }),
        )?;

        let msg = request_vault.force_withdraw_unlocking_msg(u.id, Some(amount))?;
        vault_withdraw_msgs.push(msg);

        total_to_liquidate = total_to_liquidate.checked_sub(amount)?;
    }

    let protocol_fee = liquidatee_request.amount.checked_sub(liquidator_request.amount)?;
    let protocol_fee_percentage =
        Decimal::checked_from_ratio(protocol_fee, liquidatee_request.amount)?;

    let update_coin_balance_msg = update_balance_after_vault_liquidation_msg(
        &deps.querier,
        &env.contract.address,
        liquidator_account_id,
        &vault_info.base_token,
        protocol_fee_percentage,
    )?;

    Ok(Response::new()
        .add_message(repay_msg)
        .add_messages(vault_withdraw_msgs)
        .add_message(update_coin_balance_msg)
        .add_attribute("action", "liquidate_vault/unlocking")
        .add_attribute("account_id", liquidator_account_id)
        .add_attribute("liquidatee_account_id", liquidatee_account_id)
        .add_attribute("coin_debt_repaid", debt.to_string())
        .add_attribute("coin_liquidated", liquidatee_request.to_string())
        .add_attribute(
            "protocol_fee_coin",
            Coin::new(protocol_fee.u128(), liquidatee_request.denom).to_string(),
        ))
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

    let (debt, liquidator_request, liquidatee_request) = calculate_vault_liquidation(
        &deps,
        liquidatee_account_id,
        &debt_coin,
        &request_vault,
        amount,
        &vault_info,
    )?;

    let repay_msg =
        repay_debt(deps.storage, &env, liquidator_account_id, liquidatee_account_id, &debt)?;

    update_vault_position(
        deps.storage,
        liquidatee_account_id,
        &request_vault.address,
        VaultPositionUpdate::Locked(UpdateType::Decrement(liquidatee_request.amount)),
    )?;

    let vault_withdraw_msg =
        request_vault.force_withdraw_locked_msg(&deps.querier, liquidatee_request.amount)?;

    let protocol_fee = liquidatee_request.amount.checked_sub(liquidator_request.amount)?;
    let protocol_fee_percentage =
        Decimal::checked_from_ratio(protocol_fee, liquidatee_request.amount)?;

    let update_coin_balance_msg = update_balance_after_vault_liquidation_msg(
        &deps.querier,
        &env.contract.address,
        liquidator_account_id,
        &vault_info.base_token,
        protocol_fee_percentage,
    )?;

    Ok(Response::new()
        .add_message(repay_msg)
        .add_message(vault_withdraw_msg)
        .add_message(update_coin_balance_msg)
        .add_attribute("action", "liquidate_vault/locked")
        .add_attribute("account_id", liquidator_account_id)
        .add_attribute("liquidatee_account_id", liquidatee_account_id)
        .add_attribute("coin_debt_repaid", debt.to_string())
        .add_attribute("coin_liquidated", liquidatee_request.to_string())
        .add_attribute(
            "protocol_fee_coin",
            Coin::new(protocol_fee.u128(), liquidatee_request.denom).to_string(),
        ))
}
