use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    pub red_bank_address: Addr,
    pub incentives_address: Addr,
}

pub mod msg {
    use cosmwasm_std::{Binary, Uint128};
    use cw20::{Cw20Coin, Expiration, Logo, MinterResponse};
    use cw20_base::msg::InstantiateMarketingInfo;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, JsonSchema)]
    pub struct InstantiateMsg {
        // cw20_base params
        pub name: String,
        pub symbol: String,
        pub decimals: u8,
        pub initial_balances: Vec<Cw20Coin>,
        pub mint: Option<MinterResponse>,
        pub marketing: Option<InstantiateMarketingInfo>,

        // custom_params
        pub init_hook: Option<InitHook>,
        pub red_bank_address: String,
        pub incentives_address: String,
    }

    /// Hook to be called after token initialization
    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    pub struct InitHook {
        pub msg: Binary,
        pub contract_addr: String,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum ExecuteMsg {
        /// Transfer is a base message to move tokens to another account. Requires to be finalized
        /// by the money market.
        Transfer { recipient: String, amount: Uint128 },

        /// Forced transfer called by the money market when an account is being liquidated
        TransferOnLiquidation {
            sender: String,
            recipient: String,
            amount: Uint128,
        },

        /// Burns tokens from user. Only money market can call this.
        /// Used when user is being liquidated
        Burn { user: String, amount: Uint128 },

        /// Send is a base message to transfer tokens to a contract and trigger an action
        /// on the receiving contract.
        Send {
            contract: String,
            amount: Uint128,
            msg: Binary,
        },

        /// Only with the "mintable" extension. If authorized, creates amount new tokens
        /// and adds to the recipient balance.
        Mint { recipient: String, amount: Uint128 },

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
        /// if `info.sender` has sufficient pre-approval.
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
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum QueryMsg {
        /// Returns the current balance of the given address, 0 if unset.
        /// Return type: BalanceResponse.
        Balance {
            address: String,
        },
        /// Returns both balance (0 if unset) and total supply
        /// Used by incentives contract when computing unclaimed rewards
        /// Return type: BalanceAndTotalSupplyResponse
        BalanceAndTotalSupply {
            address: String,
        },
        /// Returns metadata on the contract - name, decimals, supply, etc.
        /// Return type: TokenInfoResponse.
        TokenInfo {},
        Minter {},
        /// Only with "allowance" extension.
        /// Returns how much spender can use from owner account, 0 if unset.
        /// Return type: AllowanceResponse.
        Allowance {
            owner: String,
            spender: String,
        },
        /// Only with "enumerable" extension (and "allowances")
        /// Returns all allowances this owner has approved. Supports pagination.
        /// Return type: AllAllowancesResponse.
        AllAllowances {
            owner: String,
            start_after: Option<String>,
            limit: Option<u32>,
        },
        /// Only with "enumerable" extension
        /// Returns all accounts that have balances. Supports pagination.
        /// Return type: AllAccountsResponse.
        AllAccounts {
            start_after: Option<String>,
            limit: Option<u32>,
        },
        /// Only with "marketing" extension
        /// Returns more metadata on the contract to display in the client:
        /// - description, logo, project url, etc.
        /// Return type: MarketingInfoResponse
        MarketingInfo {},
        /// Only with "marketing" extension
        /// Downloads the mbeded logo data (if stored on chain). Errors if no logo data ftored for this
        /// contract.
        /// Return type: DownloadLogoResponse.
        DownloadLogo {},
        /// Returns the underlying asset amount for given address.
        /// Return type: BalanceResponse.
        UnderlyingAssetBalance {
            address: String,
        },
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    pub struct BalanceAndTotalSupplyResponse {
        pub balance: Uint128,
        pub total_supply: Uint128,
    }
}
