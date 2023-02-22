use std::convert::TryInto;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Empty, StdError};
use cw721_base::QueryMsg as ParentQueryMsg;

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    //--------------------------------------------------------------------------------------------------
    // Extended messages
    //--------------------------------------------------------------------------------------------------
    #[returns(crate::nft_config::UncheckedNftConfig)]
    Config {},

    #[returns(u64)]
    NextId {},

    //--------------------------------------------------------------------------------------------------
    // Base cw721 messages
    //--------------------------------------------------------------------------------------------------
    /// Return the owner of the given token, error if token does not exist
    #[returns(cw721::OwnerOfResponse)]
    OwnerOf {
        token_id: String,
        /// unset or false will filter out expired approvals, you must set to true to see them
        include_expired: Option<bool>,
    },

    /// Return operator that can access all of the owner's tokens.
    #[returns(cw721::ApprovalResponse)]
    Approval {
        token_id: String,
        spender: String,
        include_expired: Option<bool>,
    },

    /// Return approvals that a token has
    #[returns(cw721::ApprovalsResponse)]
    Approvals {
        token_id: String,
        include_expired: Option<bool>,
    },

    /// List all operators that can access all of the owner's tokens
    #[returns(cw721::OperatorsResponse)]
    AllOperators {
        owner: String,
        /// unset or false will filter out expired items, you must set to true to see them
        include_expired: Option<bool>,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Total number of tokens issued
    #[returns(cw721::NumTokensResponse)]
    NumTokens {},

    /// With MetaData Extension.
    /// Returns top-level metadata about the contract
    #[returns(cw721::ContractInfoResponse)]
    ContractInfo {},
    /// With MetaData Extension.
    /// Returns metadata about one particular token, based on *ERC721 Metadata JSON Schema*
    /// but directly from the contract
    #[returns(cw721::NftInfoResponse<cosmwasm_std::Empty>)]
    NftInfo {
        token_id: String,
    },
    /// With MetaData Extension.
    /// Returns the result of both `NftInfo` and `OwnerOf` as one query as an optimization for clients
    #[returns(cw721::AllNftInfoResponse<cosmwasm_std::Empty>)]
    AllNftInfo {
        token_id: String,
        /// unset or false will filter out expired approvals, you must set to true to see them
        include_expired: Option<bool>,
    },

    /// With Enumerable extension.
    /// Returns all tokens owned by the given address, [] if unset.
    #[returns(cw721::TokensResponse)]
    Tokens {
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// With Enumerable extension.
    /// Requires pagination. Lists all token_ids controlled by the contract.
    /// Return type: TokensResponse.
    #[returns(cw721::TokensResponse)]
    AllTokens {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Return the minter
    #[returns(cw721_base::MinterResponse)]
    Minter {},
}

impl TryInto<ParentQueryMsg<Empty>> for QueryMsg {
    type Error = StdError;

    fn try_into(self) -> Result<ParentQueryMsg<Empty>, Self::Error> {
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
            QueryMsg::NftInfo {
                token_id,
            } => Ok(ParentQueryMsg::NftInfo {
                token_id,
            }),
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
            QueryMsg::AllTokens {
                start_after,
                limit,
            } => Ok(ParentQueryMsg::AllTokens {
                start_after,
                limit,
            }),
            QueryMsg::Minter {} => Ok(ParentQueryMsg::Minter {}),
            _ => Err(StdError::generic_err(
                "Attempting to convert to a non-cw721 compatible message",
            )),
        }
    }
}
