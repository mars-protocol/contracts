use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Uint128;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct TotalSupplyResponse {
    pub total_supply: Uint128,
}

pub mod msg {
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    pub use cw20_base::msg::{ExecuteMsg, InstantiateMsg};

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum QueryMsg {
        /// Returns the current balance of the given address, 0 if unset.
        /// Return type: BalanceResponse.
        Balance {
            address: String,
        },
        /// Returns the balance of the given address at a given block
        /// Return type: BalanceResponse.
        BalanceAt {
            address: String,
            block: u64,
        },
        /// Returns metadata on the contract - name, decimals, supply, etc.
        /// Return type: TokenInfoResponse.
        TokenInfo {},
        /// Total Supply at a given block
        /// Return type: TotalSupplyResponse
        TotalSupplyAt {
            block: u64,
        },
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
    }
}
