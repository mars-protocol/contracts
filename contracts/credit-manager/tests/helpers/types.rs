use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct MockEnv {
    pub credit_manager: Addr,
    pub red_bank: Addr,
    pub nft: Addr,
}
