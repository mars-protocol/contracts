use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_binary, Addr, Coin, CosmosMsg, Decimal, StdResult, Uint128, WasmMsg};

use crate::adapters::vault::{Vault, VaultPositionType, VaultUnchecked};
use crate::msg::instantiate::ConfigUpdates;

#[cw_serde]
pub enum ExecuteMsg {
    //--------------------------------------------------------------------------------------------------
    // Public messages
    //--------------------------------------------------------------------------------------------------
    /// Mints NFT representing a credit account for user. User can have many.
    CreateCreditAccount {},
    /// Update user's position on their credit account
    UpdateCreditAccount {
        account_id: String,
        actions: Vec<Action>,
    },

    //--------------------------------------------------------------------------------------------------
    // Privileged messages
    //--------------------------------------------------------------------------------------------------
    /// Update contract config constants
    UpdateConfig { new_config: ConfigUpdates },
    /// Internal actions only callable by the contract itself
    Callback(CallbackMsg),
}

/// The list of actions that users can perform on their positions
#[cw_serde]
pub enum Action {
    /// Deposit coin of specified denom and amount. Verifies if the correct amount is sent with transaction.
    Deposit(Coin),
    /// Withdraw coin of specified denom and amount
    Withdraw(Coin),
    /// Borrow coin of specified amount from Red Bank
    Borrow(Coin),
    /// Repay coin of specified amount back to Red Bank
    Repay(Coin),
    /// Deposit coins into vault strategy
    /// If amount sent is None, Rover attempts to deposit the account's entire balance into the vault
    EnterVault {
        vault: VaultUnchecked,
        denom: String,
        amount: Option<Uint128>,
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
    ExitVaultUnlocked { id: u64, vault: VaultUnchecked },
    /// Pay back debt of a liquidatable rover account for a bonus. Requires specifying 1) the debt
    /// denom/amount of what the liquidator wants to payoff and 2) the request coin denom which the
    /// liquidatee should have a balance of. The amount returned to liquidator will be the request coin
    /// of the amount that precisely matches the value of the debt + a liquidation bonus.
    /// The debt amount will be adjusted down if:
    /// - Exceeds liquidatee's total debt for denom
    /// - Not enough liquidatee request coin balance to match
    /// - The value of the debt repaid exceeds the maximum close factor %
    LiquidateCoin {
        /// The credit account id of the one with a liquidation threshold health factor 1 or below
        liquidatee_account_id: String,
        /// The coin debt that the liquidator wishes to pay back on behalf of the liquidatee.
        /// The liquidator must already have these assets in their credit account.
        debt_coin: Coin,
        /// The coin they wish to acquire from the liquidatee (amount returned will include the bonus)
        request_coin_denom: String,
    },
    /// Pay back debt of a liquidatable rover account for a via liquidating a vault position.
    /// Similar to LiquidateCoin {} msg and will make similar adjustments to the request.
    /// The vault position will be withdrawn (and force withdrawn if a locked vault position) and
    /// the underlying assets will transferred to the liquidator.
    /// The `VaultPositionType` will determine which bucket to liquidate from.
    LiquidateVault {
        liquidatee_account_id: String,
        debt_coin: Coin,
        request_vault: VaultUnchecked,
        position_type: VaultPositionType,
    },
    /// Perform a swapper with an exact-in amount. Requires slippage allowance %.
    SwapExactIn {
        coin_in: Coin,
        denom_out: String,
        slippage: Decimal,
    },
    /// Add Vec<Coin> to liquidity pool in exchange for LP tokens
    ProvideLiquidity {
        coins_in: Vec<Coin>,
        lp_token_out: String,
        minimum_receive: Uint128,
    },
    /// Send LP token and withdraw corresponding reserve assets from pool
    WithdrawLiquidity { lp_token: Coin },
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
        coin: Coin,
        recipient: Addr,
    },
    /// Borrow specified amount of coin from Red Bank;
    /// Increase the token's coin amount and debt shares;
    Borrow { account_id: String, coin: Coin },
    /// Repay coin of specified amount back to Red Bank;
    /// Decrement the token's coin amount and debt shares;
    Repay { account_id: String, coin: Coin },
    /// Calculate the account's max loan-to-value health factor. If above 1,
    /// emits a `position_changed` event. If 1 or below, raises an error.
    AssertBelowMaxLTV { account_id: String },
    /// Adds coin to a vault strategy
    EnterVault {
        account_id: String,
        vault: Vault,
        denom: String,
        amount: Option<Uint128>,
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
    LiquidateCoin {
        liquidator_account_id: String,
        liquidatee_account_id: String,
        debt_coin: Coin,
        request_coin_denom: String,
    },
    LiquidateVault {
        liquidator_account_id: String,
        liquidatee_account_id: String,
        debt_coin: Coin,
        request_vault: Vault,
        position_type: VaultPositionType,
    },
    /// Perform a swapper with an exact-in amount. Requires slippage allowance %.
    SwapExactIn {
        account_id: String,
        coin_in: Coin,
        denom_out: String,
        slippage: Decimal,
    },
    /// Used to update the coin balance of account after an async action
    UpdateCoinBalance {
        /// Account that needs coin balance adjustment
        account_id: String,
        /// Total balance for coin in Rover prior to withdraw
        previous_balance: Coin,
    },
    /// Add Vec<Coin> to liquidity pool in exchange for LP tokens
    ProvideLiquidity {
        account_id: String,
        coins_in: Vec<Coin>,
        lp_token_out: String,
        minimum_receive: Uint128,
    },
    /// Send LP token and withdraw corresponding reserve assets from pool
    WithdrawLiquidity { account_id: String, lp_token: Coin },
    /// Checks to ensure only one vault position is taken per credit account
    AssertOneVaultPositionOnly { account_id: String },
    /// Refunds all coin balances back to user wallet
    RefundAllCoinBalances { account_id: String },
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
