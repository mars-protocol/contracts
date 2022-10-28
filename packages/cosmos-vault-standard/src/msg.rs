#[cfg(feature = "cw20")]
use {
    cosmwasm_std::{StdError, StdResult},
    cw20::Cw20Coin,
    cw_asset::{Asset, AssetInfo},
    std::convert::TryFrom,
};

#[cfg(feature = "lockup")]
use crate::extensions::lockup::{LockupExecuteMsg, LockupQueryMsg};

#[cfg(feature = "keeper")]
use crate::extensions::keeper::{KeeperExecuteMsg, KeeperQueryMsg};

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cosmwasm_std::{Coin, Empty};
use schemars::JsonSchema;

#[cw_serde]
pub enum ExecuteMsg<T = ExtensionExecuteMsg> {
    /// Called to deposit into the vault. Native assets are passed in the funds
    /// parameter.
    Deposit {
        /// With the cw20 feature, it is allowed to deposit CW20 tokens. These
        /// must be passed in with to the `cw20s` argument and have allowance
        /// pre-approved.
        #[cfg(feature = "cw20")]
        cw20s: Option<Vec<Cw20Coin>>,
        /// The optional recipient of the vault token. If not set, the caller
        /// address will be used instead.
        recipient: Option<String>,
    },

    /// Called to redeem vault tokens and receive assets back from the vault.
    /// The native vault token must be passed in the funds parameter, unless the
    /// lockup extension is called, in which case the vault token has already
    /// been passed to ExecuteMsg::Unlock.
    Redeem {
        /// An optional field containing which address should receive the
        /// withdrawn underlying assets. If not set, the caller address will be
        /// used instead.
        recipient: Option<String>,
        /// The amount of vault tokens sent to the contract. In the case that
        /// the vault token is a Cosmos native denom, we of course have this
        /// information in the info.funds, but if the vault implements the Cw4626
        /// API, then we need this argument. We figured it's better to have one
        /// API for both types of vaults, so we require this argument.
        amount: Uint128,
    },

    /// Support for custom extensions
    VaultExtension(T),
}

/// Contains ExecuteMsgs of all enabled extensions. To enable extensions defined
/// outside of this create, you can define your own `ExtensionExecuteMsg` type
/// in your contract crate and pass it in as the generic parameter to ExecuteMsg
#[cw_serde]
pub enum ExtensionExecuteMsg {
    #[cfg(feature = "keeper")]
    Keeper(KeeperExecuteMsg),
    #[cfg(feature = "lockup")]
    Lockup(LockupExecuteMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg<T = ExtensionQueryMsg>
where
    T: JsonSchema,
{
    /// Returns `VaultStandardInfo` with information on the version of the vault
    /// standard used as well as any enabled extensions.
    #[returns(VaultStandardInfo)]
    VaultStandardInfo {},

    /// Returns `VaultInfo` representing vault requirements, lockup, & vault
    /// token denom.
    #[returns(VaultInfo)]
    Info {},

    /// Returns `Uint128` amount of vault tokens that will be returned for the
    /// passed in assets.
    ///
    /// Allows an on-chain or off-chain user to simulate the effects of their
    /// deposit at the current block, given current on-chain conditions.
    ///
    /// MUST return as close to and no more than the exact amount of Vault
    /// shares that would be minted in a deposit call in the same transaction.
    /// I.e. deposit should return the same or more shares as previewDeposit if
    /// called in the same transaction.
    ///
    /// MUST NOT account for deposit limits like those returned from maxDeposit
    /// and should always act as though the deposit would be accepted,
    /// regardless if the user has enough tokens approved, etc.
    ///
    /// MUST be inclusive of deposit fees. Integrators should be aware of the
    /// existence of deposit fees.
    #[returns(Uint128)]
    PreviewDeposit {
        coins: Vec<Coin>,
        #[cfg(feature = "cw20")]
        cw20s: Vec<Cw20Coin>,
    },

    /// Returns `AssetsResponse` representing all the assets that would be
    /// redeemed in exchange for vault tokens. Used by Rover to calculate vault
    /// position values.
    #[returns(AssetsResponse)]
    PreviewRedeem { amount: Uint128 },

    /// Returns `Option<AssetsResponse>` maximum amount of assets that can be
    /// deposited into the Vault for the `recipient`, through a call to Deposit.
    ///
    /// MUST return the maximum amount of assets deposit would allow to be
    /// deposited for `recipient` and not cause a revert, which MUST NOT be higher
    /// than the actual maximum that would be accepted (it should underestimate
    /// if necessary). This assumes that the user has infinite assets, i.e.
    /// MUST NOT rely on the asset balances of `recipient`.
    ///
    /// MUST factor in both global and user-specific limits, like if deposits
    /// are entirely disabled (even temporarily) it MUST return 0.
    #[returns(Option<AssetsResponse>)]
    MaxDeposit { recipient: String },

    /// Returns `Option<Uint128>` maximum amount of Vault shares that can be redeemed
    /// from the owner balance in the Vault, through a call to Withdraw
    ///
    /// TODO: Keep this? Could potentially be combined with MaxWithdraw to return
    /// a MaxWithdrawResponse type that includes both max assets that can be
    /// withdrawn as well as max vault shares that can be withdrawn in exchange
    /// for assets.
    #[returns(Option<Uint128>)]
    MaxRedeem { owner: String },

    /// Returns `AssetsResponse` assets managed by vault.
    /// Useful for display purposes, and does not have to confer the exact
    /// amount of underlying assets.
    #[returns(AssetsResponse)]
    TotalAssets {},

    /// Returns `Uint128` total amount of vault tokens in circulation.
    #[returns(Uint128)]
    TotalVaultTokenSupply {},

    /// The amount of shares that the vault would exchange for the amount of
    /// assets provided, in an ideal scenario where all the conditions are met.
    ///
    /// Useful for display purposes and does not have to confer the exact amount
    /// of shares returned by the vault if the passed in assets were deposited.
    /// This calculation may not reflect the “per-user” price-per-share, and
    /// instead should reflect the “average-user’s” price-per-share, meaning
    /// what the average user should expect to see when exchanging to and from.
    #[returns(Uint128)]
    ConvertToShares {
        coins: Vec<Coin>,
        #[cfg(feature = "cw20")]
        cw20s: Vec<Cw20Coin>,
    },

    /// Returns `AssetsResponse` assets that the Vault would exchange for
    /// the amount of shares provided, in an ideal scenario where all the
    /// conditions are met.
    ///
    /// Useful for display purposes and does not have to confer the exact amount
    /// of assets returned by the vault if the passed in shares were withdrawn.
    /// This calculation may not reflect the “per-user” price-per-share, and
    /// instead should reflect the “average-user’s” price-per-share, meaning
    /// what the average user should expect to see when exchanging to and from.
    #[returns(AssetsResponse)]
    ConvertToAssets { shares: Uint128 },

    /// TODO: How to handle return derive? We must supply a type here, but we
    /// don't know it.
    #[returns(Empty)]
    VaultExtension(T),
}

/// Contains QueryMsgs of all enabled extensions. To enable extensions defined
/// outside of this create, you can define your own `ExtensionQueryMsg` type
/// in your contract crate and pass it in as the generic parameter to QueryMsg
#[cw_serde]
pub enum ExtensionQueryMsg {
    #[cfg(feature = "keeper")]
    Keeper(KeeperQueryMsg),
    #[cfg(feature = "lockup")]
    Lockup(LockupQueryMsg),
}

/// Struct returned from QueryMsg::VaultStandardInfo with information about the
/// used version of the vault standard and any extensions used.
///
/// This struct should be stored as an Item under the `vault_standard_info` key,
/// so that other contracts can do a RawQuery and read it directly from storage
/// instead of needing to do a costly SmartQuery.
#[cw_serde]
pub struct VaultStandardInfo {
    /// The version of the vault standard used. A number, e.g. 1, 2, etc.
    pub version: u16,
    /// A list of vault standard extensions used by the vault.
    /// E.g. ["cw20", "lockup", "keeper"]
    pub extensions: Vec<String>,
}

#[cw_serde]
pub struct AssetsResponse {
    pub coin: Coin,
    #[cfg(feature = "cw20")]
    pub cw20s: Vec<Cw20Coin>,
}

/// Returned by QueryMsg::Info and contains information about this vault
#[cw_serde]
pub struct VaultInfo {
    pub req_denom: String,
    #[cfg(feature = "cw20")]
    pub deposit_cw20s: Vec<Cw20Coin>,
    /// Denom of vault token
    pub vault_token_denom: String,
}
