use std::collections::BTreeSet;

use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdResult, WasmMsg,
};
use mars_account_nft::msg::ExecuteMsg as NftExecuteMsg;
use mars_red_bank_types::oracle::ActionKind;
use mars_rover::{
    coins::Coins,
    error::{ContractError, ContractResult},
    msg::execute::{Action, CallbackMsg, LiquidateRequest},
};
use mars_rover_health_types::AccountKind;

use crate::{
    borrow::borrow,
    claim_rewards::claim_rewards,
    deposit::{assert_deposit_caps, deposit},
    health::{assert_max_ltv, query_health_state},
    hls::assert_account_requirements,
    lend::lend,
    liquidate::assert_not_self_liquidation,
    liquidate_deposit::liquidate_deposit,
    liquidate_lend::liquidate_lend,
    reclaim::reclaim,
    refund::refund_coin_balances,
    repay::{repay, repay_for_recipient},
    state::{ACCOUNT_KINDS, ACCOUNT_NFT, REENTRANCY_GUARD},
    swap::swap_exact_in,
    update_coin_balances::{update_coin_balance, update_coin_balance_after_vault_liquidation},
    utils::assert_is_token_owner,
    vault::{
        enter_vault, exit_vault, exit_vault_unlocked, liquidate_vault, request_vault_unlock,
        update_vault_coin_balance,
    },
    withdraw::withdraw,
    zap::{provide_liquidity, withdraw_liquidity},
};

pub fn create_credit_account(
    deps: DepsMut,
    user: Addr,
    kind: AccountKind,
) -> ContractResult<Response> {
    let account_nft = ACCOUNT_NFT.load(deps.storage)?;

    let next_id = account_nft.query_next_id(&deps.querier)?;
    ACCOUNT_KINDS.save(deps.storage, &next_id, &kind)?;

    let nft_mint_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: account_nft.address().into(),
        funds: vec![],
        msg: to_binary(&NftExecuteMsg::Mint {
            user: user.to_string(),
        })?,
    });

    Ok(Response::new()
        .add_message(nft_mint_msg)
        .add_attribute("action", "create_credit_account")
        .add_attribute("kind", kind.to_string()))
}

pub fn dispatch_actions(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account_id: &str,
    actions: Vec<Action>,
) -> ContractResult<Response> {
    assert_is_token_owner(&deps, &info.sender, account_id)?;
    REENTRANCY_GUARD.try_lock(deps.storage)?;

    let mut response = Response::new();
    let mut callbacks: Vec<CallbackMsg> = vec![];
    let mut received_coins = Coins::try_from(info.funds)?;
    let prev_health_state = query_health_state(deps.as_ref(), account_id, ActionKind::Default)?;

    // We use a Set to record all denoms whose deposited amount may go up as the
    // result of any action. We invoke the AssertDepositCaps callback in the end
    // to make sure that none of the deposit cap is exceeded.
    //
    // Additionally, we use a BTreeSet (instead of a Vec or HashSet) to ensure
    // uniqueness and determininism.
    //
    // There are a few actions that may result in an asset's deposit amount
    // going up:
    // - Deposit: we check the deposited denom
    // - SwapExactIn: we check the output denom
    // - ClaimRewards: we don't check here; the reward amount is likely small so
    //   won't have much impact; this is also difficult to handle given that now
    //   we have multi-rewards
    // - ExitVault/ExitVaultUnlocked: we don't check here; it isn't reasonable
    //   to not allow a user to exit a vault because deposit cap will be exceeded
    //
    // Note that Borrow/Lend/Reclaim does not impact total deposit amount,
    // because they simply move assets between Red Bank and Rover. We don't
    // check these actions.
    let mut denoms_for_cap_check = BTreeSet::new();

    for action in actions {
        match action {
            Action::Deposit(coin) => {
                response = deposit(&mut deps, response, account_id, &coin, &mut received_coins)?;
                // check the deposit cap of the deposited denom
                denoms_for_cap_check.insert(coin.denom);
            }
            Action::Withdraw(coin) => callbacks.push(CallbackMsg::Withdraw {
                account_id: account_id.to_string(),
                coin,
                recipient: info.sender.clone(),
            }),
            Action::Borrow(coin) => callbacks.push(CallbackMsg::Borrow {
                account_id: account_id.to_string(),
                coin,
            }),
            Action::Repay {
                recipient_account_id,
                coin,
            } => {
                if let Some(recipient) = recipient_account_id {
                    callbacks.push(CallbackMsg::RepayForRecipient {
                        benefactor_account_id: account_id.to_string(),
                        recipient_account_id: recipient,
                        coin,
                    })
                } else {
                    callbacks.push(CallbackMsg::Repay {
                        account_id: account_id.to_string(),
                        coin,
                    })
                }
            }
            Action::Lend(coin) => callbacks.push(CallbackMsg::Lend {
                account_id: account_id.to_string(),
                coin,
            }),
            Action::Reclaim(coin) => callbacks.push(CallbackMsg::Reclaim {
                account_id: account_id.to_string(),
                coin,
            }),
            Action::ClaimRewards {} => callbacks.push(CallbackMsg::ClaimRewards {
                account_id: account_id.to_string(),
            }),
            Action::EnterVault {
                vault,
                coin,
            } => callbacks.push(CallbackMsg::EnterVault {
                account_id: account_id.to_string(),
                vault: vault.check(deps.api)?,
                coin,
            }),
            Action::Liquidate {
                liquidatee_account_id,
                debt_coin,
                request,
            } => match request {
                LiquidateRequest::Deposit(denom) => callbacks.push(CallbackMsg::Liquidate {
                    liquidator_account_id: account_id.to_string(),
                    liquidatee_account_id: liquidatee_account_id.to_string(),
                    debt_coin,
                    request: LiquidateRequest::Deposit(denom),
                }),
                LiquidateRequest::Lend(denom) => callbacks.push(CallbackMsg::Liquidate {
                    liquidator_account_id: account_id.to_string(),
                    liquidatee_account_id: liquidatee_account_id.to_string(),
                    debt_coin,
                    request: LiquidateRequest::Lend(denom),
                }),
                LiquidateRequest::Vault {
                    request_vault,
                    position_type,
                } => callbacks.push(CallbackMsg::Liquidate {
                    liquidator_account_id: account_id.to_string(),
                    liquidatee_account_id: liquidatee_account_id.to_string(),
                    debt_coin,
                    request: LiquidateRequest::Vault {
                        request_vault: request_vault.check(deps.api)?,
                        position_type,
                    },
                }),
            },
            Action::SwapExactIn {
                coin_in,
                denom_out,
                slippage,
            } => {
                callbacks.push(CallbackMsg::SwapExactIn {
                    account_id: account_id.to_string(),
                    coin_in,
                    denom_out: denom_out.clone(),
                    slippage,
                });
                // check the deposit cap of the swap output denom
                denoms_for_cap_check.insert(denom_out);
            }
            Action::ExitVault {
                vault,
                amount,
            } => callbacks.push(CallbackMsg::ExitVault {
                account_id: account_id.to_string(),
                vault: vault.check(deps.api)?,
                amount,
            }),
            Action::RequestVaultUnlock {
                vault,
                amount,
            } => callbacks.push(CallbackMsg::RequestVaultUnlock {
                account_id: account_id.to_string(),
                vault: vault.check(deps.api)?,
                amount,
            }),
            Action::ExitVaultUnlocked {
                id,
                vault,
            } => callbacks.push(CallbackMsg::ExitVaultUnlocked {
                account_id: account_id.to_string(),
                vault: vault.check(deps.api)?,
                position_id: id,
            }),
            Action::ProvideLiquidity {
                coins_in,
                lp_token_out,
                minimum_receive,
            } => callbacks.push(CallbackMsg::ProvideLiquidity {
                account_id: account_id.to_string(),
                lp_token_out,
                coins_in,
                minimum_receive,
            }),
            Action::WithdrawLiquidity {
                lp_token,
                minimum_receive,
            } => callbacks.push(CallbackMsg::WithdrawLiquidity {
                account_id: account_id.to_string(),
                lp_token,
                minimum_receive,
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
        // Ensures the account state abides by the rules of the account kind
        CallbackMsg::AssertAccountReqs {
            account_id: account_id.to_string(),
        },
        // After user selected actions, we assert LTV is either:
        // - Healthy, if prior to actions MaxLTV health factor >= 1 or None
        // - Not further weakened, if prior to actions MaxLTV health factor < 1
        // Else, throw error and revert all actions
        CallbackMsg::AssertMaxLTV {
            account_id: account_id.to_string(),
            prev_health_state,
        },
        // After user selected actions, we assert that the relevant deposit caps
        // are not exceeded.
        CallbackMsg::AssertDepositCaps {
            denoms: denoms_for_cap_check,
        },
        // Removes guard so that subsequent action dispatches can be made
        CallbackMsg::RemoveReentrancyGuard {},
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
        } => borrow(deps, &account_id, coin),
        CallbackMsg::Repay {
            account_id,
            coin,
        } => repay(deps, &account_id, &coin),
        CallbackMsg::RepayForRecipient {
            benefactor_account_id,
            recipient_account_id,
            coin,
        } => repay_for_recipient(deps, env, &benefactor_account_id, &recipient_account_id, coin),
        CallbackMsg::Lend {
            account_id,
            coin,
        } => lend(deps, &account_id, &coin),
        CallbackMsg::Reclaim {
            account_id,
            coin,
        } => reclaim(deps, &account_id, &coin),
        CallbackMsg::ClaimRewards {
            account_id,
        } => claim_rewards(deps, env, &account_id),
        CallbackMsg::AssertMaxLTV {
            account_id,
            prev_health_state,
        } => assert_max_ltv(deps.as_ref(), &account_id, prev_health_state),
        CallbackMsg::AssertDepositCaps {
            denoms,
        } => assert_deposit_caps(deps.as_ref(), denoms),
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
        } => {
            assert_not_self_liquidation(&liquidator_account_id, &liquidatee_account_id)?;
            match request {
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
            }
        }
        CallbackMsg::SwapExactIn {
            account_id,
            coin_in,
            denom_out,
            slippage,
        } => swap_exact_in(deps, env, &account_id, &coin_in, &denom_out, slippage),
        CallbackMsg::UpdateCoinBalance {
            account_id,
            previous_balance,
            change,
        } => update_coin_balance(deps, env, &account_id, previous_balance, change),
        CallbackMsg::UpdateCoinBalanceAfterVaultLiquidation {
            account_id,
            previous_balance,
            protocol_fee,
        } => update_coin_balance_after_vault_liquidation(
            deps,
            env,
            &account_id,
            &previous_balance,
            protocol_fee,
        ),
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
            minimum_receive,
        } => withdraw_liquidity(deps, env, &account_id, &lp_token, minimum_receive),
        CallbackMsg::RefundAllCoinBalances {
            account_id,
        } => refund_coin_balances(deps, env, &account_id),
        CallbackMsg::AssertAccountReqs {
            account_id,
        } => assert_account_requirements(deps, account_id),
        CallbackMsg::RemoveReentrancyGuard {} => REENTRANCY_GUARD.try_unlock(deps.storage),
    }
}
