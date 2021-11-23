use crate::{Config, GlobalState, Proposal, ProposalVote};
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map, U64Key};

pub const CONFIG: Item<Config> = Item::new("config");
pub const GLOBAL_STATE: Item<GlobalState> = Item::new("global_state");
pub const PROPOSALS: Map<U64Key, Proposal> = Map::new("proposals");
pub const PROPOSAL_VOTES: Map<(U64Key, &Addr), ProposalVote> = Map::new("proposal_votes");
