use cosmwasm_std::{
    attr, ensure_eq, to_json_binary, Coin, CosmosMsg, Deps, DepsMut, Env, Event, MessageInfo,
    Response, StdError, Uint128, WasmMsg,
};
use mars_types::{
    credit_manager::{self, Action, ActionAmount, ActionCoin, Positions, QueryMsg},
    health::AccountKind,
    oracle::ActionKind,
};

use crate::{
    contract::Vault,
    error::ContractError,
    msg::{PerformanceFeeConfig, UnlockState},
    state::{
        ACCOUNT_NFT, COOLDOWN_PERIOD, CREDIT_MANAGER, HEALTH, ORACLE, PERFORMANCE_FEE_CONFIG,
        PERFORMANCE_FEE_STATE, UNLOCKS, VAULT_ACC_ID,
    },
};

pub fn bind_credit_manager_account(
    deps: DepsMut,
    info: &MessageInfo,
    account_id: String,
) -> Result<Response, ContractError> {
    let credit_manager = CREDIT_MANAGER.load(deps.storage)?;
    ensure_eq!(info.sender, credit_manager, ContractError::NotCreditManager {});

    // only one binding allowed
    let vault_acc_id = VAULT_ACC_ID.may_load(deps.storage)?;
    if vault_acc_id.is_some() {
        return Err(ContractError::VaultAccountExists {});
    }

    VAULT_ACC_ID.save(deps.storage, &account_id)?;

    let event = Event::new("vault/bind_credit_manager_account")
        .add_attributes(vec![attr("account_id", account_id)]);
    Ok(Response::new().add_event(event))
}

pub fn deposit(
    deps: DepsMut,
    env: Env,
    info: &MessageInfo,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let cm_addr = CREDIT_MANAGER.load(deps.storage)?;
    let Some(vault_acc_id) = VAULT_ACC_ID.may_load(deps.storage)? else {
        // bind credit manager account first
        return Err(ContractError::VaultAccountNotFound {});
    };

    // unwrap recipient or use caller's address
    let vault_token_recipient =
        recipient.map_or(Ok(info.sender.clone()), |r| deps.api.addr_validate(&r))?;

    // load state
    let vault = Vault::default();
    let base_token = vault.base_token.load(deps.storage)?.to_string();
    let vault_token = vault.vault_token.load(deps.storage)?;

    // check that only the expected base token was sent
    let amount = cw_utils::must_pay(info, &base_token)?;

    // calculate vault tokens
    let total_staked_amount = total_base_token_in_account(deps.as_ref())?;
    let vault_token_supply = vault_token.query_total_supply(deps.as_ref())?;

    let mut performance_fee_state = PERFORMANCE_FEE_STATE.load(deps.storage)?;
    let performance_fee_config = PERFORMANCE_FEE_CONFIG.load(deps.storage)?;
    performance_fee_state.update_fee_and_pnl(
        env.block.time.seconds(),
        total_staked_amount,
        &performance_fee_config,
    )?;
    performance_fee_state.update_by_deposit(total_staked_amount, amount)?;
    PERFORMANCE_FEE_STATE.save(deps.storage, &performance_fee_state)?;
    let total_staked_amount_without_fee =
        total_staked_amount.checked_sub(performance_fee_state.accumulated_fee)?;

    let vault_tokens = vault.calculate_vault_tokens(
        amount,
        total_staked_amount_without_fee,
        vault_token_supply,
    )?;

    let coin_deposited = Coin {
        denom: base_token,
        amount,
    };

    let deposit_to_cm = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cm_addr.to_string(),
        msg: to_json_binary(&credit_manager::ExecuteMsg::UpdateCreditAccount {
            account_id: Some(vault_acc_id.clone()),
            account_kind: None,
            actions: vec![Action::Deposit(coin_deposited.clone())],
        })?,
        funds: vec![coin_deposited],
    });

    let event = Event::new("vault/deposit").add_attributes(vec![
        attr("action", "mint_vault_tokens"),
        attr("recipient", vault_token_recipient.to_string()),
        attr("mint_amount", vault_tokens),
    ]);

    Ok(vault_token
        .mint(deps, &env, &vault_token_recipient, vault_tokens)?
        .add_message(deposit_to_cm)
        .add_event(event))
}

pub fn unlock(
    deps: DepsMut,
    env: Env,
    info: &MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    if VAULT_ACC_ID.may_load(deps.storage)?.is_none() {
        // bind credit manager account first
        return Err(ContractError::VaultAccountNotFound {});
    };

    // cannot unlock zero vault tokens
    if amount.is_zero() {
        return Err(ContractError::InvalidAmount {
            reason: "provided zero vault tokens".to_string(),
        });
    }

    // load state
    let vault = Vault::default();
    let vault_token = vault.vault_token.load(deps.storage)?;

    // Cannot unlock more than total vault token supply.
    let vault_token_supply = vault_token.query_total_supply(deps.as_ref())?;
    if amount > vault_token_supply {
        return Err(ContractError::InvalidAmount {
            reason: "amount exceeds total vault token supply".to_string(),
        });
    }

    // add new unlock request
    let current_time = env.block.time.seconds();
    let cooldown_period = COOLDOWN_PERIOD.load(deps.storage)?;
    let cooldown_end = current_time + cooldown_period;
    UNLOCKS.update(deps.storage, info.sender.to_string(), |maybe_unlocks| {
        let mut unlocks = maybe_unlocks.unwrap_or_default();

        unlocks.push(UnlockState {
            created_at: current_time,
            cooldown_end,
            vault_tokens: amount,
        });

        Ok::<Vec<UnlockState>, StdError>(unlocks)
    })?;

    Ok(Response::new()
        .add_attribute("action", "unlock")
        .add_attribute("vault_tokens", amount)
        .add_attribute("created_at", current_time.to_string())
        .add_attribute("cooldown_end", cooldown_end.to_string()))
}

pub fn redeem(
    mut deps: DepsMut,
    env: Env,
    info: &MessageInfo,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let cm_addr = CREDIT_MANAGER.load(deps.storage)?;
    let Some(vault_acc_id) = VAULT_ACC_ID.may_load(deps.storage)? else {
        // bind credit manager account first
        return Err(ContractError::VaultAccountNotFound {});
    };

    // unwrap recipient or use caller's address
    let recipient = recipient.map_or(Ok(info.sender.clone()), |x| deps.api.addr_validate(&x))?;

    // load state
    let vault = Vault::default();
    let base_token = vault.base_token.load(deps.storage)?;
    let vault_token = vault.vault_token.load(deps.storage)?;

    // check that only the expected base token was sent
    let vault_token_amount = cw_utils::must_pay(info, &vault_token.to_string())?;

    let unlocks = UNLOCKS.may_load(deps.storage, recipient.to_string())?.unwrap_or_default();

    // find all unlocked positions
    let current_time = env.block.time.seconds();
    let (unlocked, unlocking): (Vec<_>, Vec<_>) =
        unlocks.into_iter().partition(|us| us.cooldown_end <= current_time);

    // cannot withdraw when there is zero unlocked positions
    if unlocked.is_empty() {
        return Err(ContractError::UnlockedPositionsNotFound {});
    }

    // clear state if no more unlocking positions
    if unlocking.is_empty() {
        UNLOCKS.remove(deps.storage, recipient.to_string());
    } else {
        UNLOCKS.save(deps.storage, recipient.to_string(), &unlocking)?;
    }

    // compute the total vault tokens to be withdrawn
    let total_unlocked_vault_tokens =
        unlocked.into_iter().map(|us| us.vault_tokens).sum::<Uint128>();

    // check that the total unlocked vault tokens match the provided vault tokens
    if vault_token_amount != total_unlocked_vault_tokens {
        return Err(ContractError::InvalidAmount {
            reason: "provided vault tokens do not match total unlocked vault tokens".to_string(),
        });
    }

    let total_staked_amount = total_base_token_in_account(deps.as_ref())?;

    let mut performance_fee_state = PERFORMANCE_FEE_STATE.load(deps.storage)?;
    let performance_fee_config = PERFORMANCE_FEE_CONFIG.load(deps.storage)?;
    performance_fee_state.update_fee_and_pnl(
        env.block.time.seconds(),
        total_staked_amount,
        &performance_fee_config,
    )?;

    let total_staked_amount_without_fee =
        total_staked_amount.checked_sub(performance_fee_state.accumulated_fee)?;
    let (tokens_to_withdraw, burn_res) = vault.burn_vault_tokens_for_base_tokens(
        deps.branch(),
        &env,
        total_staked_amount_without_fee,
        vault_token_amount,
    )?;

    performance_fee_state.update_by_withdraw(total_staked_amount, tokens_to_withdraw)?;

    PERFORMANCE_FEE_STATE.save(deps.storage, &performance_fee_state)?;

    let mut actions =
        prepare_lend_and_borrow_actions(deps.as_ref(), base_token.clone(), tokens_to_withdraw)?;

    actions.push(Action::WithdrawToWallet {
        coin: ActionCoin {
            denom: base_token,
            amount: ActionAmount::Exact(tokens_to_withdraw),
        },
        recipient: recipient.to_string(),
    });

    let withdraw_from_cm = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cm_addr.to_string(),
        msg: to_json_binary(&credit_manager::ExecuteMsg::UpdateCreditAccount {
            account_id: Some(vault_acc_id.clone()),
            account_kind: None,
            actions,
        })?,
        funds: vec![],
    });

    let event = Event::new("vault/redeem").add_attributes(vec![
        attr("action", "burn_vault_tokens"),
        attr("recipient", recipient.clone()),
        attr("vault_token_amount", vault_token_amount),
        attr("lp_tokens_to_withdraw", tokens_to_withdraw),
    ]);

    Ok(burn_res.add_message(withdraw_from_cm).add_event(event))
}

/// Prepare lend and borrow actions to redeem the desired base token balance.
/// If there is enough base token deposited, no actions are needed.
/// If there is not enough base token deposited, it will try to reclaim the lent base token first.
/// If there is still not enough base token deposited, it will borrow the remaining amount.
fn prepare_lend_and_borrow_actions(
    deps: Deps,
    base_token: String,
    withraw_amount: Uint128,
) -> Result<Vec<Action>, ContractError> {
    let cm_addr = CREDIT_MANAGER.load(deps.storage)?;
    let vault_acc_id = VAULT_ACC_ID.load(deps.storage)?;

    let positions: Positions = deps.querier.query_wasm_smart(
        cm_addr,
        &QueryMsg::Positions {
            account_id: vault_acc_id,
        },
    )?;
    let base_token_deposit = positions
        .deposits
        .iter()
        .filter(|d| d.denom == base_token)
        .map(|d| d.amount)
        .sum::<Uint128>();

    if withraw_amount <= base_token_deposit {
        return Ok(vec![]);
    }

    let base_token_lend =
        positions.lends.iter().filter(|d| d.denom == base_token).map(|d| d.amount).sum::<Uint128>();

    let mut actions = vec![];

    let mut left_amount_to_withdraw = withraw_amount - base_token_deposit;
    if !base_token_lend.is_zero() {
        if left_amount_to_withdraw <= base_token_lend {
            actions.push(Action::Reclaim(ActionCoin {
                denom: base_token.clone(),
                amount: credit_manager::ActionAmount::Exact(left_amount_to_withdraw),
            }));

            return Ok(actions);
        } else {
            // reclaim all lent base token
            actions.push(Action::Reclaim(ActionCoin {
                denom: base_token.clone(),
                amount: credit_manager::ActionAmount::Exact(base_token_lend),
            }));

            left_amount_to_withdraw -= base_token_lend
        }
    }

    actions.push(Action::Borrow(Coin {
        denom: base_token.clone(),
        amount: left_amount_to_withdraw,
    }));

    Ok(actions)
}

pub fn total_base_token_in_account(deps: Deps) -> Result<Uint128, ContractError> {
    let cm_addr = CREDIT_MANAGER.load(deps.storage)?;
    let vault_acc_id = VAULT_ACC_ID.load(deps.storage)?;

    let cm_acc_kind: AccountKind = deps.querier.query_wasm_smart(
        cm_addr,
        &QueryMsg::AccountKind {
            account_id: vault_acc_id.clone(),
        },
    )?;

    let vault = Vault::default();
    let base_token = vault.base_token.load(deps.storage)?;

    let health = HEALTH.load(deps.storage)?;
    let health_values = health.query_health_values(
        &deps.querier,
        &vault_acc_id,
        cm_acc_kind,
        ActionKind::Default,
    )?;
    let net_value =
        health_values.total_collateral_value.checked_sub(health_values.total_debt_value)?;

    let oracle = ORACLE.load(deps.storage)?;
    let base_token_price =
        oracle.query_price(&deps.querier, &base_token, ActionKind::Default)?.price;

    let base_token_in_account = net_value.checked_div_floor(base_token_price)?;
    Ok(base_token_in_account)
}

pub fn withdraw_performance_fee(
    deps: DepsMut,
    env: Env,
    info: &MessageInfo,
    new_performance_fee_config: Option<PerformanceFeeConfig>,
) -> Result<Response, ContractError> {
    let cm_addr = CREDIT_MANAGER.load(deps.storage)?;
    let Some(vault_acc_id) = VAULT_ACC_ID.may_load(deps.storage)? else {
        // bind credit manager account first
        return Err(ContractError::VaultAccountNotFound {});
    };

    let acc_nft = ACCOUNT_NFT.load(deps.storage)?;
    let vault_acc_owner_addr = acc_nft.query_nft_token_owner(&deps.querier, &vault_acc_id)?;
    if vault_acc_owner_addr != info.sender {
        return Err(ContractError::NotTokenOwner {
            user: info.sender.to_string(),
            account_id: vault_acc_id,
        });
    }

    let total_staked_amount = total_base_token_in_account(deps.as_ref())?;

    let mut performance_fee_state = PERFORMANCE_FEE_STATE.load(deps.storage)?;
    let performance_fee_config = PERFORMANCE_FEE_CONFIG.load(deps.storage)?;
    performance_fee_state.update_fee_and_pnl(
        env.block.time.seconds(),
        total_staked_amount,
        &performance_fee_config,
    )?;
    let accumulated_performace_fee = performance_fee_state.accumulated_fee;
    performance_fee_state.update_by_manager(
        env.block.time.seconds(),
        total_staked_amount,
        &performance_fee_config,
    )?;

    PERFORMANCE_FEE_STATE.save(deps.storage, &performance_fee_state)?;

    // update performance fee config if new config is provided
    if let Some(new_config) = new_performance_fee_config {
        PERFORMANCE_FEE_CONFIG.save(deps.storage, &new_config)?;
    }

    let event = Event::new("withdraw_performance_fee").add_attributes(vec![
        attr("recipient", vault_acc_owner_addr.clone()),
        attr("amount", accumulated_performace_fee),
    ]);

    let vault = Vault::default();
    let base_token = vault.base_token.load(deps.storage)?;

    let mut actions = prepare_lend_and_borrow_actions(
        deps.as_ref(),
        base_token.clone(),
        accumulated_performace_fee,
    )?;

    actions.push(Action::WithdrawToWallet {
        coin: ActionCoin {
            denom: base_token,
            amount: ActionAmount::Exact(accumulated_performace_fee),
        },
        recipient: vault_acc_owner_addr.to_string(),
    });

    let withdraw_from_cm = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cm_addr.to_string(),
        msg: to_json_binary(&credit_manager::ExecuteMsg::UpdateCreditAccount {
            account_id: Some(vault_acc_id.clone()),
            account_kind: None,
            actions,
        })?,
        funds: vec![],
    });

    Ok(Response::new().add_message(withdraw_from_cm).add_event(event))
}
