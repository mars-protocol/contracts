use std::convert::TryInto;

use cosmwasm_std::StdError;
use cw721_base::QueryMsg as ParentQueryMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    //--------------------------------------------------------------------------------------------------
    // Extended messages
    //--------------------------------------------------------------------------------------------------
    ProposedNewOwner,

    //--------------------------------------------------------------------------------------------------
    // Base cw721 messages
    //--------------------------------------------------------------------------------------------------
    /// Return the owner of the given token, error if token does not exist
    /// Return type: OwnerOfResponse
    OwnerOf {
        token_id: String,
        /// unset or false will filter out expired approvals, you must set to true to see them
        include_expired: Option<bool>,
    },

    /// Return operator that can access all of the owner's tokens.
    /// Return type: `ApprovalResponse`
    Approval {
        token_id: String,
        spender: String,
        include_expired: Option<bool>,
    },

    /// Return approvals that a token has
    /// Return type: `ApprovalsResponse`
    Approvals {
        token_id: String,
        include_expired: Option<bool>,
    },

    /// List all operators that can access all of the owner's tokens
    /// Return type: `OperatorsResponse`
    AllOperators {
        owner: String,
        /// unset or false will filter out expired items, you must set to true to see them
        include_expired: Option<bool>,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Total number of tokens issued
    NumTokens,

    /// With MetaData Extension.
    /// Returns top-level metadata about the contract: `ContractInfoResponse`
    ContractInfo,
    /// With MetaData Extension.
    /// Returns metadata about one particular token, based on *ERC721 Metadata JSON Schema*
    /// but directly from the contract: `NftInfoResponse`
    NftInfo {
        token_id: String,
    },
    /// With MetaData Extension.
    /// Returns the result of both `NftInfo` and `OwnerOf` as one query as an optimization
    /// for clients: `AllNftInfo`
    AllNftInfo {
        token_id: String,
        /// unset or false will filter out expired approvals, you must set to true to see them
        include_expired: Option<bool>,
    },

    /// With Enumerable extension.
    /// Returns all tokens owned by the given address, [] if unset.
    /// Return type: TokensResponse.
    Tokens {
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// With Enumerable extension.
    /// Requires pagination. Lists all token_ids controlled by the contract.
    /// Return type: TokensResponse.
    AllTokens {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Return the minter
    Minter,
}

impl TryInto<ParentQueryMsg> for QueryMsg {
    type Error = StdError;

    fn try_into(self) -> Result<ParentQueryMsg, Self::Error> {
        match self {
            QueryMsg::OwnerOf {
                token_id,
                include_expired,
            } => Ok(ParentQueryMsg::OwnerOf {
                token_id,
                include_expired,
            }),
            QueryMsg::Approval {
                token_id,
                spender,
                include_expired,
            } => Ok(ParentQueryMsg::Approval {
                token_id,
                spender,
                include_expired,
            }),
            QueryMsg::Approvals {
                token_id,
                include_expired,
            } => Ok(ParentQueryMsg::Approvals {
                token_id,
                include_expired,
            }),
            QueryMsg::AllOperators {
                owner,
                include_expired,
                start_after,
                limit,
            } => Ok(ParentQueryMsg::AllOperators {
                owner,
                include_expired,
                start_after,
                limit,
            }),
            QueryMsg::NumTokens {} => Ok(ParentQueryMsg::NumTokens {}),
            QueryMsg::ContractInfo {} => Ok(ParentQueryMsg::ContractInfo {}),
            QueryMsg::NftInfo { token_id } => Ok(ParentQueryMsg::NftInfo { token_id }),
            QueryMsg::AllNftInfo {
                token_id,
                include_expired,
            } => Ok(ParentQueryMsg::AllNftInfo {
                token_id,
                include_expired,
            }),
            QueryMsg::Tokens {
                owner,
                start_after,
                limit,
            } => Ok(ParentQueryMsg::Tokens {
                owner,
                start_after,
                limit,
            }),
            QueryMsg::AllTokens { start_after, limit } => {
                Ok(ParentQueryMsg::AllTokens { start_after, limit })
            }
            QueryMsg::Minter {} => Ok(ParentQueryMsg::Minter {}),
            _ => Err(StdError::generic_err(
                "Attempting to convert to a non-cw721 compatible message",
            )),
        }
    }
}
