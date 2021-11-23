use cosmwasm_std::{Addr, CosmosMsg, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::MarsError;
use crate::helpers::all_conditions_valid;
use crate::math::decimal::Decimal;

/// Council global configuration
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Address provider returns addresses for all protocol contracts
    pub address_provider_address: Addr,
    /// Blocks during which a proposal is active since being submitted
    pub proposal_voting_period: u64,
    /// Blocks that need to pass since a proposal succeeds in order for it to be available to be
    /// executed
    pub proposal_effective_delay: u64,
    /// Blocks after the effective_delay during which a successful proposal can be activated before it expires
    pub proposal_expiration_period: u64,
    /// Number of Mars needed to make a proposal. Will be returned if successful. Will be
    /// distributed between stakers if rejected.
    pub proposal_required_deposit: Uint128,
    /// % of total voting power required to participate in the proposal in order to consider it successfull
    pub proposal_required_quorum: Decimal,
    /// % of for votes required in order to consider the proposal successful
    pub proposal_required_threshold: Decimal,
}

impl Config {
    pub fn validate(&self) -> Result<(), MarsError> {
        let conditions_and_names = vec![
            (
                Self::less_or_equal_one(&self.proposal_required_quorum),
                "proposal_required_quorum",
            ),
            (
                Self::less_or_equal_one(&self.proposal_required_threshold),
                "proposal_required_threshold",
            ),
        ];
        all_conditions_valid(conditions_and_names)
    }

    fn less_or_equal_one(value: &Decimal) -> bool {
        value.le(&Decimal::one())
    }
}

/// Global state
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GlobalState {
    /// Number of proposals
    pub proposal_count: u64,
}

/// Proposal metadata stored in state
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Proposal {
    pub proposal_id: u64,
    /// Address submitting the proposal
    pub submitter_address: Addr,
    /// Wether the proposal is Active, Passed, Rejected or Executed
    pub status: ProposalStatus,
    /// Number of for votes
    pub for_votes: Uint128,
    /// Number of against votes
    pub against_votes: Uint128,
    /// Block at which voting for the porposal starts
    pub start_height: u64,
    /// Block at which voting for the porposal ends
    pub end_height: u64,
    /// Title for the proposal
    pub title: String,
    /// Description for the proposal
    pub description: String,
    /// Link provided for cases where the proposal description is too large or
    /// some other external resource is intended to be associated with the proposal
    pub link: Option<String>,
    /// Set of messages available to get executed if the proposal passes
    pub messages: Option<Vec<ProposalMessage>>,
    /// MARS tokens deposited on the proposal submission. Will be returned to
    /// submitter if proposal passes and sent to xMars stakers otherwise
    pub deposit_amount: Uint128,
}

/// Execute call that will be executed by the DAO if the proposal succeeds
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ProposalMessage {
    /// Determines order of execution lower order will be executed first
    pub execution_order: u64,
    /// CosmosMsg that will be executed by the council
    pub msg: CosmosMsg,
}

/// Proposal Status
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    /// Proposal is being voted on
    Active,
    /// Proposal has been approved but has not been executed yet
    Passed,
    /// Proposal was rejected
    Rejected,
    /// Proposal has been approved and executed
    Executed,
}

/// Single vote made by an address
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ProposalVote {
    /// For or Against the proposal
    pub option: ProposalVoteOption,
    /// Voting power
    pub power: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProposalVoteOption {
    For,
    Against,
}

impl std::fmt::Display for ProposalVoteOption {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let display_str = match self {
            ProposalVoteOption::For => "for",
            ProposalVoteOption::Against => "against",
        };
        write!(f, "{}", display_str)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ProposalsListResponse {
    /// Total proposals submitted
    pub proposal_count: u64,
    /// List of proposals (paginated by query)
    pub proposal_list: Vec<Proposal>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ProposalVotesResponse {
    pub proposal_id: u64,
    pub votes: Vec<ProposalVoteResponse>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ProposalVoteResponse {
    pub voter_address: String,
    pub option: ProposalVoteOption,
    pub power: Uint128,
}

pub mod msg {
    use cosmwasm_std::Uint128;
    use cw20::Cw20ReceiveMsg;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    use crate::math::decimal::Decimal;

    use super::{ProposalMessage, ProposalVoteOption};

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    pub struct InstantiateMsg {
        pub config: CreateOrUpdateConfig,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
    pub struct CreateOrUpdateConfig {
        pub address_provider_address: Option<String>,

        pub proposal_voting_period: Option<u64>,
        pub proposal_effective_delay: Option<u64>,
        pub proposal_expiration_period: Option<u64>,
        pub proposal_required_deposit: Option<Uint128>,
        pub proposal_required_quorum: Option<Decimal>,
        pub proposal_required_threshold: Option<Decimal>,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum ExecuteMsg {
        /// Implementation cw20 receive msg
        Receive(Cw20ReceiveMsg),

        /// Vote for a proposal
        CastVote {
            proposal_id: u64,
            vote: ProposalVoteOption,
        },

        /// End proposal after voting period has passed
        EndProposal { proposal_id: u64 },

        /// Execute a successful proposal
        ExecuteProposal { proposal_id: u64 },

        /// Update config
        UpdateConfig { config: CreateOrUpdateConfig },
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum ReceiveMsg {
        /// Submit a proposal to be voted
        /// Requires a Mars deposit equal or greater than the proposal_required_deposit
        SubmitProposal {
            title: String,
            description: String,
            link: Option<String>,
            messages: Option<Vec<ProposalMessage>>,
        },
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum QueryMsg {
        Config {},
        Proposals {
            start: Option<u64>,
            limit: Option<u32>,
        },
        Proposal {
            proposal_id: u64,
        },
        ProposalVotes {
            proposal_id: u64,
            start_after: Option<String>,
            limit: Option<u32>,
        },
    }
}
