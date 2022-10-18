use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, DepsMut, Empty, Env, MessageInfo, Response, StdResult, WasmMsg,
};
use cw721::OwnerOfResponse;
use cw721_base::QueryMsg;

use crate::borrow::borrow;
use crate::deposit::deposit;
use crate::health::assert_below_max_ltv;
use crate::repay::repay;
use crate::state::{
    ACCOUNT_NFT, ALLOWED_COINS, ALLOWED_VAULTS, MAX_CLOSE_FACTOR, MAX_LIQUIDATION_BONUS, ORACLE,
    OWNER, RED_BANK, SWAPPER,
};
use crate::vault::{
    deposit_into_vault, liquidate_vault, request_unlock_from_vault, update_vault_coin_balance,
    withdraw_from_vault, withdraw_unlocked_from_vault,
};

use crate::liquidate_coin::{assert_health_factor_improved, liquidate_coin};
use crate::swap::swap_exact_in;
use crate::update_coin_balances::update_coin_balances;
use crate::withdraw::withdraw;
use account_nft::msg::ExecuteMsg as NftExecuteMsg;
use rover::coins::Coins;
use rover::error::{ContractError, ContractResult};
use rover::msg::execute::{Action, CallbackMsg};
use rover::msg::instantiate::ConfigUpdates;
use rover::traits::Stringify;

pub fn create_credit_account(deps: DepsMut, user: Addr) -> ContractResult<Response> {
    let contract_addr = ACCOUNT_NFT.load(deps.storage)?;

    let nft_mint_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract_addr.to_string(),
        funds: vec![],
        msg: to_binary(&NftExecuteMsg::Mint {
            user: user.to_string(),
        })?,
    });

    Ok(Response::new()
        .add_message(nft_mint_msg)
        .add_attribute("action", "rover/credit_manager/create_credit_account"))
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_config: ConfigUpdates,
) -> ContractResult<Response> {
    let owner = OWNER.load(deps.storage)?;

    if info.sender != owner {
        return Err(ContractError::Unauthorized {
            user: info.sender.into(),
            action: "update config".to_string(),
        });
    }

    let mut response =
        Response::new().add_attribute("action", "rover/credit_manager/update_config");

    if let Some(addr_str) = new_config.account_nft {
        let validated = deps.api.addr_validate(&addr_str)?;
        ACCOUNT_NFT.save(deps.storage, &validated)?;

        // Accept ownership. NFT contract owner must have proposed Rover as a new owner first.
        let accept_ownership_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: addr_str.clone(),
            funds: vec![],
            msg: to_binary(&NftExecuteMsg::AcceptOwnership {})?,
        });

        response = response
            .add_message(accept_ownership_msg)
            .add_attribute("key", "account_nft")
            .add_attribute("value", addr_str);
    }

    if let Some(addr_str) = new_config.owner {
        let validated = deps.api.addr_validate(&addr_str)?;
        OWNER.save(deps.storage, &validated)?;
        response = response
            .add_attribute("key", "owner")
            .add_attribute("value", addr_str);
    }

    if let Some(coins) = new_config.allowed_coins {
        coins
            .iter()
            .try_for_each(|denom| ALLOWED_COINS.save(deps.storage, denom, &Empty {}))?;
        response = response
            .add_attribute("key", "allowed_coins")
            .add_attribute("value", coins.join(", "));
    }

    if let Some(vaults) = new_config.allowed_vaults {
        vaults.iter().try_for_each(|unchecked| {
            let vault = unchecked.check(deps.api)?;
            ALLOWED_VAULTS.save(deps.storage, &vault.address, &Empty {})
        })?;
        response = response
            .add_attribute("key", "allowed_vaults")
            .add_attribute("value", vaults.to_string())
    }

    if let Some(unchecked) = new_config.red_bank {
        RED_BANK.save(deps.storage, &unchecked.check(deps.api)?)?;
        response = response
            .add_attribute("key", "red_bank")
            .add_attribute("value", unchecked.address());
    }

    if let Some(unchecked) = new_config.oracle {
        ORACLE.save(deps.storage, &unchecked.check(deps.api)?)?;
        response = response
            .add_attribute("key", "oracle")
            .add_attribute("value", unchecked.address());
    }

    if let Some(unchecked) = new_config.swapper {
        SWAPPER.save(deps.storage, &unchecked.check(deps.api)?)?;
        response = response
            .add_attribute("key", "swapper")
            .add_attribute("value", unchecked.address());
    }

    if let Some(bonus) = new_config.max_liquidation_bonus {
        MAX_LIQUIDATION_BONUS.save(deps.storage, &bonus)?;
        response = response
            .add_attribute("key", "max_liquidation_bonus")
            .add_attribute("value", bonus.to_string());
    }

    if let Some(cf) = new_config.max_close_factor {
        MAX_CLOSE_FACTOR.save(deps.storage, &cf)?;
        response = response
            .add_attribute("key", "max_close_factor")
            .add_attribute("value", cf.to_string());
    }

    Ok(response)
}

pub fn dispatch_actions(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account_id: &str,
    actions: &[Action],
) -> ContractResult<Response> {
    assert_is_token_owner(&deps, &info.sender, account_id)?;

    let mut response = Response::new();
    let mut callbacks: Vec<CallbackMsg> = vec![];
    let mut received_coins = Coins::from(info.funds.as_slice());

    for action in actions {
        match action {
            Action::Deposit(coin) => {
                response = deposit(
                    deps.storage,
                    response,
                    account_id,
                    coin,
                    &mut received_coins,
                )?;
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
            Action::Repay(coin) => callbacks.push(CallbackMsg::Repay {
                account_id: account_id.to_string(),
                coin: coin.clone(),
            }),
            Action::VaultDeposit {
                vault,
                coins: assets,
            } => callbacks.push(CallbackMsg::VaultDeposit {
                account_id: account_id.to_string(),
                vault: vault.check(deps.api)?,
                coins: assets.clone(),
            }),
            Action::LiquidateCoin {
                liquidatee_account_id,
                debt_coin,
                request_coin_denom,
            } => callbacks.push(CallbackMsg::LiquidateCoin {
                liquidator_account_id: account_id.to_string(),
                liquidatee_account_id: liquidatee_account_id.to_string(),
                debt_coin: debt_coin.clone(),
                request_coin_denom: request_coin_denom.clone(),
            }),
            Action::LiquidateVault {
                liquidatee_account_id,
                debt_coin,
                request_vault,
            } => callbacks.push(CallbackMsg::LiquidateVault {
                liquidator_account_id: account_id.to_string(),
                liquidatee_account_id: liquidatee_account_id.to_string(),
                debt_coin: debt_coin.clone(),
                request_vault: request_vault.check(deps.api)?,
            }),
            Action::SwapExactIn {
                coin_in,
                denom_out,
                slippage,
            } => callbacks.push(CallbackMsg::SwapExactIn {
                account_id: account_id.to_string(),
                coin_in: coin_in.clone(),
                denom_out: denom_out.to_string(),
                slippage: *slippage,
            }),
            Action::VaultWithdraw { vault, amount } => callbacks.push(CallbackMsg::VaultWithdraw {
                account_id: account_id.to_string(),
                vault: vault.check(deps.api)?,
                amount: *amount,
            }),
            Action::VaultRequestUnlock { vault, amount } => {
                callbacks.push(CallbackMsg::VaultRequestUnlock {
                    account_id: account_id.to_string(),
                    vault: vault.check(deps.api)?,
                    amount: *amount,
                })
            }
            Action::VaultWithdrawUnlocked { id, vault } => {
                callbacks.push(CallbackMsg::VaultWithdrawUnlocked {
                    account_id: account_id.to_string(),
                    vault: vault.check(deps.api)?,
                    position_id: *id,
                })
            }
        }
    }

    // after all deposits have been handled, we assert that the `received_natives` list is empty
    // this way, we ensure that the user does not send any extra fund which will get lost in the contract
    if !received_coins.is_empty() {
        return Err(ContractError::ExtraFundsReceived(received_coins));
    }

    // after user selected actions, we assert LTV is healthy; if not, throw error and revert all actions
    callbacks.extend([CallbackMsg::AssertBelowMaxLTV {
        account_id: account_id.to_string(),
    }]);

    let callback_msgs = callbacks
        .iter()
        .map(|callback| callback.into_cosmos_msg(&env.contract.address))
        .collect::<StdResult<Vec<CosmosMsg>>>()?;

    Ok(response
        .add_messages(callback_msgs)
        .add_attribute("action", "rover/execute/update_credit_account"))
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
        CallbackMsg::Borrow { coin, account_id } => borrow(deps, env, &account_id, coin),
        CallbackMsg::Repay { account_id, coin } => repay(deps, env, &account_id, coin),
        CallbackMsg::AssertBelowMaxLTV { account_id } => {
            assert_below_max_ltv(deps.as_ref(), env, &account_id)
        }
        CallbackMsg::VaultDeposit {
            account_id,
            vault,
            coins,
        } => deposit_into_vault(deps, &env.contract.address, &account_id, vault, &coins),
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
        CallbackMsg::LiquidateCoin {
            liquidator_account_id,
            liquidatee_account_id,
            debt_coin,
            request_coin_denom,
        } => liquidate_coin(
            deps,
            env,
            &liquidator_account_id,
            &liquidatee_account_id,
            debt_coin,
            &request_coin_denom,
        ),
        CallbackMsg::LiquidateVault {
            liquidator_account_id,
            liquidatee_account_id,
            debt_coin,
            request_vault,
        } => liquidate_vault(
            deps,
            env,
            &liquidator_account_id,
            &liquidatee_account_id,
            debt_coin,
            request_vault,
        ),
        CallbackMsg::AssertHealthFactorImproved {
            account_id,
            previous_health_factor,
        } => assert_health_factor_improved(deps.as_ref(), env, &account_id, previous_health_factor),
        CallbackMsg::SwapExactIn {
            account_id,
            coin_in,
            denom_out,
            slippage,
        } => swap_exact_in(deps, env, &account_id, coin_in, &denom_out, slippage),
        CallbackMsg::UpdateCoinBalances {
            account_id,
            previous_balances,
        } => update_coin_balances(deps, env, &account_id, &previous_balances),
        CallbackMsg::VaultWithdraw {
            account_id,
            vault,
            amount,
        } => withdraw_from_vault(deps, env, &account_id, vault, amount, false),
        CallbackMsg::VaultForceWithdraw {
            account_id,
            vault,
            amount,
        } => withdraw_from_vault(deps, env, &account_id, vault, amount, true),
        CallbackMsg::VaultRequestUnlock {
            account_id,
            vault,
            amount,
        } => request_unlock_from_vault(deps, &account_id, vault, amount),
        CallbackMsg::VaultWithdrawUnlocked {
            account_id,
            vault,
            position_id,
        } => withdraw_unlocked_from_vault(deps, env, &account_id, vault, position_id),
    }
}

pub fn assert_is_token_owner(deps: &DepsMut, user: &Addr, account_id: &str) -> ContractResult<()> {
    let contract_addr = ACCOUNT_NFT.load(deps.storage)?;
    let owner_res: OwnerOfResponse = deps.querier.query_wasm_smart(
        contract_addr,
        &QueryMsg::<Empty>::OwnerOf {
            token_id: account_id.to_string(),
            include_expired: None,
        },
    )?;

    if user != &owner_res.owner {
        return Err(ContractError::NotTokenOwner {
            user: user.to_string(),
            account_id: account_id.to_string(),
        });
    }

    Ok(())
}
