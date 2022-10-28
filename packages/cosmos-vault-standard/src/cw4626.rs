use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, Empty, Uint128};
use cw20::{Cw20Coin, Expiration, Logo};

use crate::msg::ExtensionExecuteMsg;

#[cw_serde]
pub enum Cw4626ExecuteMsg<T = ExtensionExecuteMsg, S = Empty> {
    //--------------------------------------------------------------------------------------------------
    // Standard CW20 ExecuteMsgs
    //--------------------------------------------------------------------------------------------------
    /// Transfer is a base message to move tokens to another account without triggering actions
    Transfer {
        recipient: String,
        amount: Uint128,
    },
    /// Send is a base message to transfer tokens to a contract and trigger an action
    /// on the receiving contract.
    Send {
        contract: String,
        amount: Uint128,
        msg: Binary,
    },
    /// Only with "approval" extension. Allows spender to access an additional amount tokens
    /// from the owner's (env.sender) account. If expires is Some(), overwrites current allowance
    /// expiration with this one.
    IncreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<Expiration>,
    },
    /// Only with "approval" extension. Lowers the spender's access of tokens
    /// from the owner's (env.sender) account by amount. If expires is Some(), overwrites current
    /// allowance expiration with this one.
    DecreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<Expiration>,
    },
    /// Only with "approval" extension. Transfers amount tokens from owner -> recipient
    /// if `env.sender` has sufficient pre-approval.
    TransferFrom {
        owner: String,
        recipient: String,
        amount: Uint128,
    },
    /// Only with "approval" extension. Sends amount tokens from owner -> contract
    /// if `env.sender` has sufficient pre-approval.
    SendFrom {
        owner: String,
        contract: String,
        amount: Uint128,
        msg: Binary,
    },
    /// Only with the "marketing" extension. If authorized, updates marketing metadata.
    /// Setting None/null for any of these will leave it unchanged.
    /// Setting Some("") will clear this field on the contract storage
    UpdateMarketing {
        /// A URL pointing to the project behind this token.
        project: Option<String>,
        /// A longer description of the token and it's utility. Designed for tooltips or such
        description: Option<String>,
        /// The address (if any) who can update this data structure
        marketing: Option<String>,
    },
    /// If set as the "marketing" role on the contract, upload a new URL, SVG, or PNG for the token
    UploadLogo(Logo),
    //--------------------------------------------------------------------------------------------------
    // CW4626 ExecuteMsgs
    //--------------------------------------------------------------------------------------------------
    Deposit {
        cw20s: Option<Vec<Cw20Coin>>,
        /// An optional field containing the recipient of the vault token. If not set, the
        /// caller address will be used instead.
        recipient: Option<String>,
    },

    Redeem {
        /// An optional field containing which address should receive the withdrawn underlying assets.
        /// If not set, the caller address will be used instead.
        recipient: Option<String>,
        amount: Uint128,
    },

    Callback(S),

    VaultExtension(T),
}
