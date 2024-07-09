use std::cmp::min;

use cosmwasm_std::{
    attr, ensure_eq, to_json_binary, BankMsg, Coin, CosmosMsg, Deps, DepsMut, Env, Event,
    MessageInfo, Order, Response, StdError, StdResult, Uint128, WasmMsg,
};
use mars_types::{
    adapters::{account_nft::AccountNftBase, health::HealthContractBase, oracle::OracleBase},
    credit_manager::{self, Action, ActionAmount, ActionCoin, ConfigResponse, Positions, QueryMsg},
    health::AccountKind,
    oracle::ActionKind,
};

use crate::{
    error::ContractError,
    msg::UnlockState,
    performance_fee::PerformanceFeeConfig,
    state::{
        BASE_TOKEN, COOLDOWN_PERIOD, CREDIT_MANAGER, OWNER, PERFORMANCE_FEE_CONFIG,
        PERFORMANCE_FEE_STATE, UNLOCKS, VAULT_ACC_ID, VAULT_TOKEN,
    },
    vault_token::{calculate_base_tokens, calculate_vault_tokens},
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

    // check if contract owner is the owner of account id
    let owner = OWNER.current(deps.storage)?.ok_or(ContractError::NoOwner {})?;
    assert_account_ownership(deps.as_ref(), &credit_manager, &account_id, owner.as_str())?;

    VAULT_ACC_ID.save(deps.storage, &account_id)?;

    let event = Event::new("bind_credit_manager_account")
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
    let base_token = BASE_TOKEN.load(deps.storage)?.to_string();
    let vault_token = VAULT_TOKEN.load(deps.storage)?;

    // check that only the expected base token was sent
    let amount = cw_utils::must_pay(info, &base_token)?;

    // calculate vault tokens
    let total_base_tokens = total_base_tokens_in_account(deps.as_ref())?;
    let vault_token_supply = vault_token.query_total_supply(deps.as_ref())?;

    let mut performance_fee_state = PERFORMANCE_FEE_STATE.load(deps.storage)?;
    let performance_fee_config = PERFORMANCE_FEE_CONFIG.load(deps.storage)?;
    performance_fee_state.update_fee_and_pnl(
        env.block.time.seconds(),
        total_base_tokens,
        &performance_fee_config,
    )?;
    performance_fee_state.update_base_tokens_after_deposit(total_base_tokens, amount)?;
    PERFORMANCE_FEE_STATE.save(deps.storage, &performance_fee_state)?;
    let total_base_tokens_without_fee =
        total_base_tokens.checked_sub(performance_fee_state.accumulated_fee)?;

    let vault_tokens =
        calculate_vault_tokens(amount, total_base_tokens_without_fee, vault_token_supply)?;

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

    let event = Event::new("deposit").add_attributes(vec![
        attr("action", "mint_vault_tokens"),
        attr("recipient", vault_token_recipient.to_string()),
        attr("vault_tokens_minted", vault_tokens),
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
    let vault_token = VAULT_TOKEN.load(deps.storage)?;

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
    UNLOCKS.save(
        deps.storage,
        (info.sender.as_str(), current_time),
        &UnlockState {
            created_at: current_time,
            cooldown_end,
            vault_tokens: amount,
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "unlock")
        .add_attribute("vault_tokens_unlocked", amount)
        .add_attribute("created_at", current_time.to_string())
        .add_attribute("cooldown_end", cooldown_end.to_string()))
}

pub fn redeem(
    deps: DepsMut,
    env: Env,
    info: &MessageInfo,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let Some(vault_acc_id) = VAULT_ACC_ID.may_load(deps.storage)? else {
        // bind credit manager account first
        return Err(ContractError::VaultAccountNotFound {});
    };

    // unwrap recipient or use caller's address
    let recipient = recipient.map_or(Ok(info.sender.clone()), |x| deps.api.addr_validate(&x))?;

    // load state
    let base_token = BASE_TOKEN.load(deps.storage)?;
    let vault_token = VAULT_TOKEN.load(deps.storage)?;

    // check that only the expected vault token was sent
    let mut vault_tokens = cw_utils::must_pay(info, &vault_token.to_string())?;

    let unlocks = UNLOCKS
        .prefix(recipient.as_str())
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (_created_at, unlock) = item?;
            Ok(unlock)
        })
        .collect::<StdResult<Vec<UnlockState>>>()?;

    // find all unlocked positions
    let current_time = env.block.time.seconds();
    let (unlocked, _unlocking): (Vec<_>, Vec<_>) =
        unlocks.into_iter().partition(|us| us.cooldown_end <= current_time);

    // cannot withdraw when there is zero unlocked positions
    if unlocked.is_empty() {
        return Err(ContractError::UnlockedPositionsNotFound {});
    }

    // remove unlocked positions
    for unlock in &unlocked {
        UNLOCKS.remove(deps.storage, (recipient.as_str(), unlock.created_at));
    }

    // compute the total vault tokens to be withdrawn
    let total_unlocked_vault_tokens =
        unlocked.into_iter().map(|us| us.vault_tokens).sum::<Uint128>();

    // check that the the provided vault tokens is equal or greater than total unlocked vault tokens
    if vault_tokens < total_unlocked_vault_tokens {
        return Err(ContractError::InvalidAmount {
            reason: "provided vault tokens is less than total unlocked amount".to_string(),
        });
    }

    let refund_vault_tokens = vault_tokens - total_unlocked_vault_tokens;
    vault_tokens = min(vault_tokens, total_unlocked_vault_tokens);

    let total_base_tokens = total_base_tokens_in_account(deps.as_ref())?;

    let mut performance_fee_state = PERFORMANCE_FEE_STATE.load(deps.storage)?;
    let performance_fee_config = PERFORMANCE_FEE_CONFIG.load(deps.storage)?;
    performance_fee_state.update_fee_and_pnl(
        env.block.time.seconds(),
        total_base_tokens,
        &performance_fee_config,
    )?;

    let total_base_tokens_without_fee =
        total_base_tokens.checked_sub(performance_fee_state.accumulated_fee)?;

    // calculate base tokens based on the given amount of vault tokens
    let vault_token_supply = vault_token.query_total_supply(deps.as_ref())?;
    let base_tokens_to_redeem =
        calculate_base_tokens(vault_tokens, total_base_tokens_without_fee, vault_token_supply)?;

    performance_fee_state
        .update_base_tokens_after_redeem(total_base_tokens, base_tokens_to_redeem)?;

    PERFORMANCE_FEE_STATE.save(deps.storage, &performance_fee_state)?;

    let withdraw_from_cm = prepare_credit_manager_msg(
        deps.as_ref(),
        base_token,
        base_tokens_to_redeem,
        recipient.to_string(),
        vault_acc_id,
    )?;

    let mut response = vault_token.burn(deps, &env, vault_tokens)?.add_message(withdraw_from_cm);

    let mut event = Event::new("redeem").add_attributes(vec![
        attr("action", "burn_vault_tokens"),
        attr("recipient", recipient.clone()),
        attr("vault_tokens_burned", vault_tokens),
        attr("base_tokens_redeemed", base_tokens_to_redeem),
    ]);

    if !refund_vault_tokens.is_zero() {
        let transfer_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![Coin {
                denom: vault_token.to_string(),
                amount: refund_vault_tokens,
            }],
        });
        response = response.add_message(transfer_msg);
        event = event.add_attribute("vault_tokens_refunded", refund_vault_tokens);
    }

    Ok(response.add_event(event))
}

fn prepare_credit_manager_msg(
    deps: Deps,
    base_token: String,
    withdraw_amt: Uint128,
    withdraw_recipient: String,
    vault_acc_id: String,
) -> Result<CosmosMsg, ContractError> {
    let cm_addr = CREDIT_MANAGER.load(deps.storage)?;

    let mut actions = prepare_lend_and_borrow_actions(
        deps,
        base_token.clone(),
        withdraw_amt,
        cm_addr.clone(),
        vault_acc_id.clone(),
    )?;
    actions.push(Action::WithdrawToWallet {
        coin: ActionCoin {
            denom: base_token,
            amount: ActionAmount::Exact(withdraw_amt),
        },
        recipient: withdraw_recipient.to_string(),
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
    Ok(withdraw_from_cm)
}

/// Prepare lend and borrow actions to redeem the desired base token balance.
/// If there is enough base token deposited, no actions are needed.
/// If there is not enough base token deposited, it will try to reclaim the lent base token first.
/// If there is still not enough base token deposited, it will borrow the remaining amount.
fn prepare_lend_and_borrow_actions(
    deps: Deps,
    base_token: String,
    withraw_amount: Uint128,
    cm_addr: String,
    vault_acc_id: String,
) -> Result<Vec<Action>, ContractError> {
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

pub fn total_base_tokens_in_account(deps: Deps) -> Result<Uint128, ContractError> {
    let cm_addr = CREDIT_MANAGER.load(deps.storage)?;
    let vault_acc_id = VAULT_ACC_ID.load(deps.storage)?;

    let cm_acc_kind: AccountKind = deps.querier.query_wasm_smart(
        cm_addr.clone(),
        &QueryMsg::AccountKind {
            account_id: vault_acc_id.clone(),
        },
    )?;

    let base_token = BASE_TOKEN.load(deps.storage)?;

    let config: ConfigResponse = deps.querier.query_wasm_smart(cm_addr, &QueryMsg::Config {})?;

    let health = HealthContractBase::new(deps.api.addr_validate(&config.health_contract)?);
    let health_values = health.query_health_values(
        &deps.querier,
        &vault_acc_id,
        cm_acc_kind,
        ActionKind::Default,
    )?;
    let net_value =
        health_values.total_collateral_value.checked_sub(health_values.total_debt_value)?;

    let oracle = OracleBase::new(deps.api.addr_validate(&config.oracle)?);
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
    let Some(vault_acc_id) = VAULT_ACC_ID.may_load(deps.storage)? else {
        // bind credit manager account first
        return Err(ContractError::VaultAccountNotFound {});
    };

    let cm_addr = CREDIT_MANAGER.load(deps.storage)?;
    assert_account_ownership(deps.as_ref(), &cm_addr, &vault_acc_id, info.sender.as_str())?;
    let vault_acc_owner_addr = info.sender.to_string();

    let total_base_tokens = total_base_tokens_in_account(deps.as_ref())?;

    let mut performance_fee_state = PERFORMANCE_FEE_STATE.load(deps.storage)?;
    let performance_fee_config = PERFORMANCE_FEE_CONFIG.load(deps.storage)?;
    performance_fee_state.update_fee_and_pnl(
        env.block.time.seconds(),
        total_base_tokens,
        &performance_fee_config,
    )?;
    let accumulated_performace_fee = performance_fee_state.accumulated_fee;
    performance_fee_state.reset_state_by_manager(
        env.block.time.seconds(),
        total_base_tokens,
        &performance_fee_config,
    )?;

    PERFORMANCE_FEE_STATE.save(deps.storage, &performance_fee_state)?;

    // update performance fee config if new config is provided
    if let Some(new_config) = new_performance_fee_config {
        new_config.validate()?;
        PERFORMANCE_FEE_CONFIG.save(deps.storage, &new_config)?;
    }

    let event = Event::new("withdraw_performance_fee").add_attributes(vec![
        attr("recipient", vault_acc_owner_addr.clone()),
        attr("amount", accumulated_performace_fee),
    ]);

    let base_token = BASE_TOKEN.load(deps.storage)?;

    let withdraw_from_cm = prepare_credit_manager_msg(
        deps.as_ref(),
        base_token,
        accumulated_performace_fee,
        vault_acc_owner_addr,
        vault_acc_id,
    )?;

    Ok(Response::new().add_message(withdraw_from_cm).add_event(event))
}

fn assert_account_ownership(
    deps: Deps,
    cm_addr: &str,
    acc_id: &str,
    user_addr: &str,
) -> Result<(), ContractError> {
    let config: ConfigResponse = deps.querier.query_wasm_smart(cm_addr, &QueryMsg::Config {})?;
    let Some(acc_nft) = config.account_nft else {
        return Err(ContractError::Std(StdError::generic_err(
            "Account NFT contract address is not set in Credit Manager".to_string(),
        )));
    };
    let acc_nft = AccountNftBase::new(deps.api.addr_validate(&acc_nft)?);
    let acc_owner_addr = acc_nft.query_nft_token_owner(&deps.querier, acc_id)?;
    if acc_owner_addr != user_addr {
        return Err(ContractError::NotTokenOwner {
            user: user_addr.to_string(),
            account_id: acc_id.to_string(),
        });
    }
    Ok(())
}
