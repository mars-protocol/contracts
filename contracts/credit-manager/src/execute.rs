use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdResult, WasmMsg,
};
use mars_account_nft::msg::ExecuteMsg as NftExecuteMsg;
use mars_rover::{
    coins::Coins,
    error::{ContractError, ContractResult},
    msg::execute::{Action, CallbackMsg, LiquidateRequest},
};

use crate::{
    borrow::borrow,
    deposit::deposit,
    health::{assert_max_ltv, query_health},
    lend::lend,
    liquidate_deposit::liquidate_deposit,
    liquidate_lend::liquidate_lend,
    reclaim::reclaim,
    refund::refund_coin_balances,
    repay::{repay, repay_for_recipient},
    state::ACCOUNT_NFT,
    swap::swap_exact_in,
    update_coin_balances::update_coin_balance,
    utils::{assert_is_token_owner, assert_not_contract_in_config},
    vault::{
        enter_vault, exit_vault, exit_vault_unlocked, liquidate_vault, request_vault_unlock,
        update_vault_coin_balance,
    },
    withdraw::withdraw,
    zap::{provide_liquidity, withdraw_liquidity},
};

pub fn create_credit_account(deps: DepsMut, user: Addr) -> ContractResult<Response> {
    let contract_addr = ACCOUNT_NFT.load(deps.storage)?;

    let nft_mint_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract_addr.to_string(),
        funds: vec![],
        msg: to_binary(&NftExecuteMsg::Mint {
            user: user.to_string(),
        })?,
    });

    Ok(Response::new().add_message(nft_mint_msg).add_attribute("action", "create_credit_account"))
}

pub fn dispatch_actions(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account_id: &str,
    actions: &[Action],
) -> ContractResult<Response> {
    assert_is_token_owner(&deps, &info.sender, account_id)?;
    assert_not_contract_in_config(&deps.as_ref(), &info.sender)?;

    let mut response = Response::new();
    let mut callbacks: Vec<CallbackMsg> = vec![];
    let mut received_coins = Coins::try_from(info.funds)?;
    let prev_health = query_health(deps.as_ref(), account_id)?;

    for action in actions {
        match action {
            Action::Deposit(coin) => {
                response = deposit(deps.storage, response, account_id, coin, &mut received_coins)?;
            }
            Action::Withdraw(coin) => callbacks.push(CallbackMsg::Withdraw {
                account_id: account_id.to_string(),
                coin: coin.clone(),
                recipient: info.sender.clone(),
            }),
            Action::Borrow(coin) => callbacks.push(CallbackMsg::Borrow {
                account_id: account_id.to_string(),
                coin: coin.clone(),
            }),
            Action::Repay {
                recipient_account_id,
                coin,
            } => {
                if let Some(recipient) = recipient_account_id {
                    callbacks.push(CallbackMsg::RepayForRecipient {
                        benefactor_account_id: account_id.to_string(),
                        recipient_account_id: recipient.clone(),
                        coin: coin.clone(),
                    })
                } else {
                    callbacks.push(CallbackMsg::Repay {
                        account_id: account_id.to_string(),
                        coin: coin.clone(),
                    })
                }
            }
            Action::Lend(coin) => callbacks.push(CallbackMsg::Lend {
                account_id: account_id.to_string(),
                coin: coin.clone(),
            }),
            Action::Reclaim(coin) => callbacks.push(CallbackMsg::Reclaim {
                account_id: account_id.to_string(),
                coin: coin.clone(),
            }),
            Action::EnterVault {
                vault,
                coin,
            } => callbacks.push(CallbackMsg::EnterVault {
                account_id: account_id.to_string(),
                vault: vault.check(deps.api)?,
                coin: coin.clone(),
            }),
            Action::Liquidate {
                liquidatee_account_id,
                debt_coin,
                request,
            } => match request {
                LiquidateRequest::Deposit(denom) => callbacks.push(CallbackMsg::Liquidate {
                    liquidator_account_id: account_id.to_string(),
                    liquidatee_account_id: liquidatee_account_id.to_string(),
                    debt_coin: debt_coin.clone(),
                    request: LiquidateRequest::Deposit(denom.clone()),
                }),
                LiquidateRequest::Lend(denom) => callbacks.push(CallbackMsg::Liquidate {
                    liquidator_account_id: account_id.to_string(),
                    liquidatee_account_id: liquidatee_account_id.to_string(),
                    debt_coin: debt_coin.clone(),
                    request: LiquidateRequest::Lend(denom.clone()),
                }),
                LiquidateRequest::Vault {
                    request_vault,
                    position_type,
                } => callbacks.push(CallbackMsg::Liquidate {
                    liquidator_account_id: account_id.to_string(),
                    liquidatee_account_id: liquidatee_account_id.to_string(),
                    debt_coin: debt_coin.clone(),
                    request: LiquidateRequest::Vault {
                        request_vault: request_vault.check(deps.api)?,
                        position_type: position_type.clone(),
                    },
                }),
            },
            Action::SwapExactIn {
                coin_in,
                denom_out,
                slippage,
            } => callbacks.push(CallbackMsg::SwapExactIn {
                account_id: account_id.to_string(),
                coin_in: coin_in.clone(),
                denom_out: denom_out.clone(),
                slippage: *slippage,
            }),
            Action::ExitVault {
                vault,
                amount,
            } => callbacks.push(CallbackMsg::ExitVault {
                account_id: account_id.to_string(),
                vault: vault.check(deps.api)?,
                amount: *amount,
            }),
            Action::RequestVaultUnlock {
                vault,
                amount,
            } => callbacks.push(CallbackMsg::RequestVaultUnlock {
                account_id: account_id.to_string(),
                vault: vault.check(deps.api)?,
                amount: *amount,
            }),
            Action::ExitVaultUnlocked {
                id,
                vault,
            } => callbacks.push(CallbackMsg::ExitVaultUnlocked {
                account_id: account_id.to_string(),
                vault: vault.check(deps.api)?,
                position_id: *id,
            }),
            Action::ProvideLiquidity {
                coins_in,
                lp_token_out,
                minimum_receive,
            } => callbacks.push(CallbackMsg::ProvideLiquidity {
                account_id: account_id.to_string(),
                lp_token_out: lp_token_out.clone(),
                coins_in: coins_in.clone(),
                minimum_receive: *minimum_receive,
            }),
            Action::WithdrawLiquidity {
                lp_token,
            } => callbacks.push(CallbackMsg::WithdrawLiquidity {
                account_id: account_id.to_string(),
                lp_token: lp_token.clone(),
            }),
            Action::RefundAllCoinBalances {} => {
                callbacks.push(CallbackMsg::RefundAllCoinBalances {
                    account_id: account_id.to_string(),
                })
            }
        }
    }

    // after all deposits have been handled, we assert that the `received_natives` list is empty
    // this way, we ensure that the user does not send any extra fund which will get lost in the contract
    if !received_coins.is_empty() {
        return Err(ContractError::ExtraFundsReceived(received_coins));
    }

    callbacks.extend([
        // after user selected actions, we assert LTV is either:
        // - Healthy, if prior to actions MaxLTV health factor >= 1 or None
        // - Not further weakened, if prior to actions MaxLTV health factor < 1
        // Else, throw error and revert all actions
        CallbackMsg::AssertMaxLTV {
            account_id: account_id.to_string(),
            prev_max_ltv_health_factor: prev_health.max_ltv_health_factor,
        },
    ]);

    let callback_msgs = callbacks
        .iter()
        .map(|callback| callback.into_cosmos_msg(&env.contract.address))
        .collect::<StdResult<Vec<CosmosMsg>>>()?;

    Ok(response
        .add_messages(callback_msgs)
        .add_attribute("action", "rover/execute/update_credit_account")
        .add_attribute("account_id", account_id.to_string()))
}

pub fn execute_callback(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    callback: CallbackMsg,
) -> ContractResult<Response> {
    if info.sender != env.contract.address {
        return Err(ContractError::ExternalInvocation);
    }
    match callback {
        CallbackMsg::Withdraw {
            account_id,
            coin,
            recipient,
        } => withdraw(deps, &account_id, coin, recipient),
        CallbackMsg::Borrow {
            coin,
            account_id,
        } => borrow(deps, env, &account_id, coin),
        CallbackMsg::Repay {
            account_id,
            coin,
        } => repay(deps, env, &account_id, &coin),
        CallbackMsg::RepayForRecipient {
            benefactor_account_id,
            recipient_account_id,
            coin,
        } => repay_for_recipient(deps, env, &benefactor_account_id, &recipient_account_id, coin),
        CallbackMsg::Lend {
            account_id,
            coin,
        } => lend(deps, env, &account_id, coin),
        CallbackMsg::Reclaim {
            account_id,
            coin,
        } => reclaim(deps, env, &account_id, &coin),
        CallbackMsg::AssertMaxLTV {
            account_id,
            prev_max_ltv_health_factor,
        } => assert_max_ltv(deps.as_ref(), env, &account_id, &prev_max_ltv_health_factor),
        CallbackMsg::EnterVault {
            account_id,
            vault,
            coin,
        } => enter_vault(deps, &env.contract.address, &account_id, vault, &coin),
        CallbackMsg::UpdateVaultCoinBalance {
            vault,
            account_id,
            previous_total_balance,
        } => update_vault_coin_balance(
            deps,
            vault,
            &account_id,
            previous_total_balance,
            &env.contract.address,
        ),
        CallbackMsg::Liquidate {
            liquidator_account_id,
            liquidatee_account_id,
            debt_coin,
            request,
        } => match request {
            LiquidateRequest::Deposit(request_coin_denom) => liquidate_deposit(
                deps,
                env,
                &liquidator_account_id,
                &liquidatee_account_id,
                debt_coin,
                &request_coin_denom,
            ),
            LiquidateRequest::Lend(request_coin_denom) => liquidate_lend(
                deps,
                env,
                &liquidator_account_id,
                &liquidatee_account_id,
                debt_coin,
                &request_coin_denom,
            ),
            LiquidateRequest::Vault {
                request_vault,
                position_type,
            } => liquidate_vault(
                deps,
                env,
                &liquidator_account_id,
                &liquidatee_account_id,
                debt_coin,
                request_vault,
                position_type,
            ),
        },
        CallbackMsg::SwapExactIn {
            account_id,
            coin_in,
            denom_out,
            slippage,
        } => swap_exact_in(deps, env, &account_id, &coin_in, &denom_out, slippage),
        CallbackMsg::UpdateCoinBalance {
            account_id,
            previous_balance,
        } => update_coin_balance(deps, env, &account_id, &previous_balance),
        CallbackMsg::ExitVault {
            account_id,
            vault,
            amount,
        } => exit_vault(deps, env, &account_id, vault, amount),
        CallbackMsg::RequestVaultUnlock {
            account_id,
            vault,
            amount,
        } => request_vault_unlock(deps, &account_id, vault, amount),
        CallbackMsg::ExitVaultUnlocked {
            account_id,
            vault,
            position_id,
        } => exit_vault_unlocked(deps, env, &account_id, vault, position_id),
        CallbackMsg::ProvideLiquidity {
            account_id,
            coins_in,
            lp_token_out,
            minimum_receive,
        } => provide_liquidity(deps, env, &account_id, coins_in, &lp_token_out, minimum_receive),
        CallbackMsg::WithdrawLiquidity {
            account_id,
            lp_token,
        } => withdraw_liquidity(deps, env, &account_id, &lp_token),
        CallbackMsg::RefundAllCoinBalances {
            account_id,
        } => refund_coin_balances(deps, env, &account_id),
    }
}
