use cosmwasm_std::StdError;
use thiserror::Error;

use mars_core::error::MarsError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Mars(#[from] MarsError),

    #[error("Invalid Proposal: {error:?}")]
    InvalidProposal { error: String },

    #[error("Proposal is not active")]
    ProposalNotActive {},

    #[error("User has already voted on this proposal")]
    VoteUserAlreadyVoted {},
    #[error("User has no voting power at block: {block:?}")]
    VoteNoVotingPower { block: u64 },
    #[error("Voting period has ended")]
    VoteVotingPeriodEnded {},

    #[error("Voting period has not ended")]
    EndProposalVotingPeriodNotEnded {},

    #[error("Proposal has not passed or has already been executed")]
    ExecuteProposalNotPassed {},
    #[error("Proposal must end it's delay period in order to be executed")]
    ExecuteProposalDelayNotEnded {},
    #[error("Proposal has expired")]
    ExecuteProposalExpired {},
}

impl ContractError {
    pub fn invalid_proposal<S: Into<String>>(error: S) -> ContractError {
        ContractError::InvalidProposal {
            error: error.into(),
        }
    }
}
