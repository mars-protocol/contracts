use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;
use cw_vault_standard::{VaultStandardExecuteMsg, VaultStandardQueryMsg};

use crate::performance_fee::PerformanceFeeConfig;

pub type ExecuteMsg = VaultStandardExecuteMsg<ExtensionExecuteMsg>;

pub type QueryMsg = VaultStandardQueryMsg<ExtensionQueryMsg>;

#[cw_serde]
pub struct InstantiateMsg {
    /// The base token denom that will be used for the native vault token, e.g. uusdc.
    pub base_token: String,
    /// The subdenom that will be used for the native vault token, e.g.
    /// the denom of the vault token will be:
    /// "factory/{vault_contract}/{vault_token_subdenom}".
    pub vault_token_subdenom: String,

    /// Optional metadata
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub description: Option<String>,

    /// Credit Manager contract address
    pub credit_manager: String,

    /// Stakers need to wait a cooldown period before being able to withdraw USDC from the vault.
    /// Value defined in seconds.
    pub cooldown_period: u64,

    /// Performance fee configuration
    pub performance_fee_config: PerformanceFeeConfig,
}

#[cw_serde]
pub enum ExtensionExecuteMsg {
    /// Bind Credit Manager account id to the vault
    BindCreditManagerAccount {
        account_id: String,
    },

    /// Unlock liquidity from the vault. This will inform Fund Manager about requested funds.
    /// The unlocked tokens will have to wait a cooldown period before they can be withdrawn.
    Unlock {
        /// The amount of vault tokens to unlock
        amount: Uint128,
    },

    /// Withdraw performance fee from the vault.
    /// This can only be done by the Fund Manager once a certain period (definied by `performance_fee_interval` parameter).
    WithdrawPerformanceFee {
        /// New performance fee config to set. It will be used for future calculations.
        new_performance_fee_config: Option<PerformanceFeeConfig>,
    },
}

#[cw_serde]
pub enum ExtensionQueryMsg {
    VaultInfo {},

    UserUnlocks {
        /// The address of the user to query
        user_address: String,
    },

    AllUnlocks {
        // (user address, created_at)
        start_after: Option<(String, u64)>,
        limit: Option<u32>,
    },

    PerformanceFeeState {},
}

#[cw_serde]
pub struct VaultInfoResponseExt {
    /// The token that is accepted for deposits, withdrawals and used for
    /// accounting in the vault. The denom is a native token
    pub base_token: String,
    /// Vault token denom
    pub vault_token: String,

    /// Optional metadata
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub description: Option<String>,

    /// Credit Manager contract address
    pub credit_manager: String,

    /// Vault account id
    pub vault_account_id: Option<String>,

    /// Stakers need to wait a cooldown period before being able to withdraw USDC from the vault.
    /// Value defined in seconds.
    pub cooldown_period: u64,

    /// Performance fee configuration
    pub performance_fee_config: PerformanceFeeConfig,
}

/// Unlock state for a single user
#[cw_serde]
#[derive(Default)]
pub struct UnlockState {
    pub created_at: u64,
    pub cooldown_end: u64,
    pub vault_tokens: Uint128,
}

#[cw_serde]
#[derive(Default)]
pub struct VaultUnlock {
    pub user_address: String,
    pub created_at: u64,
    pub cooldown_end: u64,
    pub vault_tokens: Uint128,
    pub base_tokens: Uint128,
}
