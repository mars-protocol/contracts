use std::collections::BTreeSet;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_binary, Addr, Coin, CosmosMsg, Decimal, StdResult, Uint128, WasmMsg};
use mars_account_nft::nft_config::NftConfigUpdates;
use mars_owner::OwnerUpdate;
use mars_rover_health_types::{AccountKind, HealthState};

use crate::{
    adapters::vault::{Vault, VaultPositionType, VaultUnchecked},
    msg::instantiate::ConfigUpdates,
};

#[cw_serde]
pub enum ExecuteMsg {
    //--------------------------------------------------------------------------------------------------
    // Public messages
    //--------------------------------------------------------------------------------------------------
    /// Mints NFT representing a credit account for user. User can have many.
    CreateCreditAccount(AccountKind),
    /// Update user's position on their credit account
    UpdateCreditAccount {
        account_id: String,
        actions: Vec<Action>,
    },
    /// Repay debt on behalf of an account, funded from wallet. Must send exactly one coin in message funds.
    /// Allows repaying debts of assets that have been de-listed from credit manager.
    RepayFromWallet {
        account_id: String,
    },

    //--------------------------------------------------------------------------------------------------
    // Privileged messages
    //--------------------------------------------------------------------------------------------------
    /// Update contract config constants
    UpdateConfig {
        updates: ConfigUpdates,
    },
    /// Manages owner role state
    UpdateOwner(OwnerUpdate),
    /// Update nft contract config
    UpdateNftConfig {
        config: Option<NftConfigUpdates>,
        ownership: Option<cw721_base::Action>,
    },
    /// Internal actions only callable by the contract itself
    Callback(CallbackMsg),
}

#[cw_serde]
pub enum ActionAmount {
    Exact(Uint128),
    AccountBalance,
}

impl ActionAmount {
    pub fn value(&self) -> Option<Uint128> {
        match self {
            ActionAmount::Exact(amt) => Some(*amt),
            ActionAmount::AccountBalance => None,
        }
    }
}

#[cw_serde]
pub struct ActionCoin {
    pub denom: String,
    pub amount: ActionAmount,
}

impl From<&Coin> for ActionCoin {
    fn from(value: &Coin) -> Self {
        Self {
            denom: value.denom.to_string(),
            amount: ActionAmount::Exact(value.amount),
        }
    }
}

#[cw_serde]
pub enum ChangeExpected {
    Increase,
    Decrease,
}

#[cw_serde]
pub enum LiquidateRequest<T> {
    /// Pay back debt of a liquidatable rover account for a bonus. Requires specifying 1) the debt
    /// denom/amount of what the liquidator wants to payoff and 2) the request coin denom which the
    /// liquidatee should have a balance of. The amount returned to liquidator will be the request coin
    /// of the amount that precisely matches the value of the debt + a liquidation bonus.
    /// The debt amount will be adjusted down if:
    /// - Exceeds liquidatee's total debt for denom
    /// - Not enough liquidatee request coin balance to match
    /// - The value of the debt repaid exceeds the maximum close factor %
    ///
    /// Liquidation should prioritize first the not lent coin and if more needs to be serviced to the liquidator
    /// it should reclaim (withdrawn from Red Bank).
    Deposit(String),
    /// Pay back debt of a liquidatable rover account for a via liquidating a Lent position.
    /// Lent shares are transfered from the liquidatable to the liquidator.
    Lend(String),
    /// Pay back debt of a liquidatable rover account for a via liquidating a vault position.
    /// Similar to `Deposit` msg and will make similar adjustments to the request.
    /// The vault position will be withdrawn (and force withdrawn if a locked vault position) and
    /// the underlying assets will transferred to the liquidator.
    /// The `VaultPositionType` will determine which bucket to liquidate from.
    Vault {
        request_vault: T,
        position_type: VaultPositionType,
    },
}

/// The list of actions that users can perform on their positions
#[cw_serde]
pub enum Action {
    /// Deposit coin of specified denom and amount. Verifies if the correct amount is sent with transaction.
    Deposit(Coin),
    /// Withdraw coin of specified denom and amount
    Withdraw(ActionCoin),
    /// Borrow coin of specified amount from Red Bank
    Borrow(Coin),
    /// Lend coin to the Red Bank
    Lend(ActionCoin),
    /// Reclaim the coins that were lent to the Red Bank.
    Reclaim(ActionCoin),
    /// For assets lent to the Red Bank, some can accumulate incentive rewards.
    /// This message claims all of them adds them to account balance.
    ClaimRewards {},
    /// Repay coin of specified amount back to Red Bank. If `amount: AccountBalance` is passed,
    /// the repaid amount will be the minimum between account balance for denom and total owed.
    /// The sender will repay on behalf of the recipient account. If 'recipient_account_id: None',
    /// the sender repays to its own account.
    Repay {
        recipient_account_id: Option<String>,
        coin: ActionCoin,
    },
    /// Deposit coins into vault strategy
    /// If `coin.amount: AccountBalance`, Rover attempts to deposit the account's entire balance into the vault
    EnterVault {
        vault: VaultUnchecked,
        coin: ActionCoin,
    },
    /// Withdraw underlying coins from vault
    ExitVault {
        vault: VaultUnchecked,
        amount: Uint128,
    },
    /// Requests unlocking of shares for a vault with a required lock period
    RequestVaultUnlock {
        vault: VaultUnchecked,
        amount: Uint128,
    },
    /// Withdraws the assets for unlocking position id from vault. Required time must have elapsed.
    ExitVaultUnlocked {
        id: u64,
        vault: VaultUnchecked,
    },
    /// Pay back debt of a liquidatable rover account for a via liquidating a specific type of the position.
    Liquidate {
        /// The credit account id of the one with a liquidation threshold health factor 1 or below
        liquidatee_account_id: String,
        /// The coin they wish to acquire from the liquidatee (amount returned will include the bonus)
        debt_coin: Coin,
        /// Position details to be liquidated
        request: LiquidateRequest<VaultUnchecked>,
    },
    /// Perform a swapper with an exact-in amount. Requires slippage allowance %.
    /// If `coin_in.amount: AccountBalance`, the accounts entire balance of `coin_in.denom` will be used.
    SwapExactIn {
        coin_in: ActionCoin,
        denom_out: String,
        slippage: Decimal,
    },
    /// Add Vec<Coin> to liquidity pool in exchange for LP tokens
    ProvideLiquidity {
        coins_in: Vec<ActionCoin>,
        lp_token_out: String,
        minimum_receive: Uint128,
    },
    /// Send LP token and withdraw corresponding reserve assets from pool.
    /// If `lp_token.amount: AccountBalance`, the account balance of `lp_token.denom` will be used.
    WithdrawLiquidity {
        lp_token: ActionCoin,
        minimum_receive: Vec<Coin>,
    },
    /// Refunds all coin balances back to user wallet
    RefundAllCoinBalances {},
}

/// Internal actions made by the contract with pre-validated inputs
#[cw_serde]
pub enum CallbackMsg {
    /// Withdraw specified amount of coin from credit account;
    /// Decrement the token's asset amount;
    Withdraw {
        account_id: String,
        coin: ActionCoin,
        recipient: Addr,
    },
    /// Borrow specified amount of coin from Red Bank;
    /// Increase the token's coin amount and debt shares;
    Borrow {
        account_id: String,
        coin: Coin,
    },
    /// Repay coin of specified amount back to Red Bank;
    /// Decrement the token's coin amount and debt shares;
    /// If `coin.amount: AccountBalance` is passed, the repaid amount will be the minimum
    /// between account balance for denom and total owed;
    Repay {
        account_id: String,
        coin: ActionCoin,
    },
    /// Benefactor account repays debt on behalf of recipient
    RepayForRecipient {
        benefactor_account_id: String,
        recipient_account_id: String,
        coin: ActionCoin,
    },
    /// Lend coin to the Red Bank
    Lend {
        account_id: String,
        coin: ActionCoin,
    },
    /// Reclaim lent coin from the Red Bank;
    /// Decrement the token's lent shares and increment the coin amount;
    Reclaim {
        account_id: String,
        coin: ActionCoin,
    },
    /// Calls incentive contract to claim all rewards and increments account balance
    ClaimRewards {
        account_id: String,
    },
    /// Assert MaxLTV is either:
    /// - Healthy, if prior to actions MaxLTV health factor >= 1 or None
    /// - Not further weakened, if prior to actions MaxLTV health factor < 1
    /// Emits a `position_changed` event.
    #[serde(rename = "assert_max_ltv")]
    AssertMaxLTV {
        account_id: String,
        prev_health_state: HealthState,
    },
    /// Assert that the total deposit amounts of the given denoms across Red
    /// Bank and Rover do not exceed their respective deposit caps.
    AssertDepositCaps {
        denoms: BTreeSet<String>,
    },
    /// Adds coin to a vault strategy
    EnterVault {
        account_id: String,
        vault: Vault,
        coin: ActionCoin,
    },
    /// Exchanges vault LP shares for assets
    ExitVault {
        account_id: String,
        vault: Vault,
        amount: Uint128,
    },
    /// Used to update the account balance of vault coins after a vault action has taken place
    UpdateVaultCoinBalance {
        vault: Vault,
        /// Account that needs vault coin balance adjustment
        account_id: String,
        /// Total vault coin balance in Rover
        previous_total_balance: Uint128,
    },
    /// Requests unlocking of shares for a vault with a lock period
    RequestVaultUnlock {
        account_id: String,
        vault: Vault,
        amount: Uint128,
    },
    /// Withdraws assets from vault for a locked position having a lockup period that has been fulfilled
    ExitVaultUnlocked {
        account_id: String,
        vault: Vault,
        position_id: u64,
    },
    /// Pay back debts of a liquidatable rover account for a bonus
    Liquidate {
        liquidator_account_id: String,
        liquidatee_account_id: String,
        debt_coin: Coin,
        request: LiquidateRequest<Vault>,
    },
    /// Perform a swapper with an exact-in amount. Requires slippage allowance %.
    /// If `coin_in.amount: AccountBalance`, the accounts entire balance of `coin_in.denom` will be used.
    SwapExactIn {
        account_id: String,
        coin_in: ActionCoin,
        denom_out: String,
        slippage: Decimal,
    },
    /// Used to update the coin balance of account after an async action
    UpdateCoinBalance {
        /// Account that needs coin balance adjustment
        account_id: String,
        /// Total balance for coin in Rover prior to withdraw
        previous_balance: Coin,
        /// The kind of change that is anticipated to balance of coin.
        /// If does not match expectation, an error is raised.
        change: ChangeExpected,
    },
    /// Used to update the coin balance of account after an async action
    UpdateCoinBalanceAfterVaultLiquidation {
        /// Account that needs coin balance adjustment
        account_id: String,
        /// Total balance for coin in Rover prior to withdraw
        previous_balance: Coin,
        /// Protocol fee percentage transfered to rewards-collector account
        protocol_fee: Decimal,
    },
    /// Add Vec<Coin> to liquidity pool in exchange for LP tokens
    ProvideLiquidity {
        account_id: String,
        coins_in: Vec<ActionCoin>,
        lp_token_out: String,
        minimum_receive: Uint128,
    },
    /// Send LP token and withdraw corresponding reserve assets from pool.
    /// If `lp_token.amount: AccountBalance`, the account balance of `lp_token.denom` will be used.
    WithdrawLiquidity {
        account_id: String,
        lp_token: ActionCoin,
        minimum_receive: Vec<Coin>,
    },
    /// Refunds all coin balances back to user wallet
    RefundAllCoinBalances {
        account_id: String,
    },
    /// Ensures that HLS accounts abide by specific rules
    AssertAccountReqs {
        account_id: String,
    },
    /// At the end of the execution of dispatched actions, this callback removes the guard
    /// and allows subsequent dispatches.
    RemoveReentrancyGuard {},
}

impl CallbackMsg {
    pub fn into_cosmos_msg(&self, contract_addr: &Addr) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
            msg: to_binary(&ExecuteMsg::Callback(self.clone()))?,
            funds: vec![],
        }))
    }
}
