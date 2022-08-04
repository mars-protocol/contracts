use cosmwasm_std::{to_binary, Addr, Coin, CosmosMsg, StdResult, WasmMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::instantiate::ConfigUpdates;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    //--------------------------------------------------------------------------------------------------
    // Public messages
    //--------------------------------------------------------------------------------------------------
    /// Mints NFT representing a credit account for user. User can have many.
    CreateCreditAccount,
    /// Update user's position on their credit account
    UpdateCreditAccount {
        token_id: String,
        actions: Vec<Action>,
    },

    //--------------------------------------------------------------------------------------------------
    // Privileged messages
    //--------------------------------------------------------------------------------------------------
    /// Update contract config constants
    UpdateConfig { new_config: ConfigUpdates },
    /// Internal actions only callable by the contract itself
    Callback(CallbackMsg),
}

/// The list of actions that users can perform on their positions
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Deposit native coin of specified type and amount. Verifies if the correct amount is sent with transaction.
    Deposit(Coin),

    /// Borrow coin of specified amount from Red Bank
    Borrow(Coin),
}

/// Internal actions made by the contract with pre-validated inputs
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CallbackMsg {
    /// Borrow specified amount of coin from Red Bank;
    /// Increase the token's asset amount and debt shares;
    Borrow { token_id: String, coin: Coin },
    /// Calculate a token's current LTV. If below the maximum LTV, emits a `position_updated`
    /// event; if above the maximum LTV, throw an error
    AssertHealth { token_id: String },
}

impl CallbackMsg {
    pub fn into_cosmos_msg(&self, contract_addr: &Addr) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
            msg: to_binary(&ExecuteMsg::Callback(self.clone()))?,
            funds: vec![],
        }))
    }
}
