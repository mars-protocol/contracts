use std::convert::TryInto;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, Empty, StdError};
use cw721::Expiration;
use cw721_base::{Action, ExecuteMsg as ParentExecuteMsg};

use super::NftConfigUpdates;

#[cw_serde]
pub enum ExecuteMsg {
    //--------------------------------------------------------------------------------------------------
    // Extended and overridden messages
    //--------------------------------------------------------------------------------------------------
    /// Update config in storage. Only minter can execute.
    UpdateConfig {
        updates: NftConfigUpdates,
    },
    /// Mint a new NFT to the specified user; can only be called by the contract minter
    Mint {
        user: String,
    },
    /// Burn an NFT the sender has access to. Will attempt to query the Credit Manager first
    /// to ensure the balance is below the config set threshold.
    Burn {
        token_id: String,
    },

    //--------------------------------------------------------------------------------------------------
    // Base cw721 messages
    //--------------------------------------------------------------------------------------------------
    /// Transfer is a base message to move a token to another account without triggering actions
    TransferNft {
        recipient: String,
        token_id: String,
    },
    /// Send is a base message to transfer a token to a contract and trigger an action
    /// on the receiving contract.
    SendNft {
        contract: String,
        token_id: String,
        msg: Binary,
    },
    /// Allows operator to transfer / send the token from the owner's account.
    /// If expiration is set, then this allowance has a time/height limit
    Approve {
        spender: String,
        token_id: String,
        expires: Option<Expiration>,
    },
    /// Remove previously granted Approval
    Revoke {
        spender: String,
        token_id: String,
    },
    /// Allows operator to transfer / send any token from the owner's account.
    /// If expiration is set, then this allowance has a time/height limit
    ApproveAll {
        operator: String,
        expires: Option<Expiration>,
    },
    /// Remove previously granted ApproveAll permission
    RevokeAll {
        operator: String,
    },
    /// Propose new owner (minter) and accept new role
    UpdateOwnership(Action),
}

/// Migrate from V1 to V2
#[cw_serde]
pub enum MigrateV1ToV2 {
    /// Burns empty accounts in batches
    BurnEmptyAccounts {
        limit: Option<u32>,
    },
}

impl TryInto<ParentExecuteMsg<Empty, Empty>> for ExecuteMsg {
    type Error = StdError;

    fn try_into(self) -> Result<ParentExecuteMsg<Empty, Empty>, Self::Error> {
        match self {
            ExecuteMsg::TransferNft {
                recipient,
                token_id,
            } => Ok(ParentExecuteMsg::TransferNft {
                recipient,
                token_id,
            }),
            ExecuteMsg::SendNft {
                contract,
                token_id,
                msg,
            } => Ok(ParentExecuteMsg::SendNft {
                contract,
                token_id,
                msg,
            }),
            ExecuteMsg::Approve {
                spender,
                token_id,
                expires,
            } => Ok(ParentExecuteMsg::Approve {
                spender,
                token_id,
                expires,
            }),
            ExecuteMsg::Revoke {
                spender,
                token_id,
            } => Ok(ParentExecuteMsg::Revoke {
                spender,
                token_id,
            }),
            ExecuteMsg::ApproveAll {
                operator,
                expires,
            } => Ok(ParentExecuteMsg::ApproveAll {
                operator,
                expires,
            }),
            ExecuteMsg::RevokeAll {
                operator,
            } => Ok(ParentExecuteMsg::RevokeAll {
                operator,
            }),
            ExecuteMsg::UpdateOwnership(action) => Ok(ParentExecuteMsg::UpdateOwnership(action)),
            _ => Err(StdError::generic_err(
                "Attempting to convert to a non-cw721 compatible message",
            )),
        }
    }
}
