use std::fmt::Debug;

use cosmwasm_schema::{cw_serde, schemars::JsonSchema};
use cosmwasm_std::{Response, StdResult, Storage};
use cw_storage_plus::Item;

use crate::error::{ContractError, ContractResult};

#[cw_serde]
pub enum GuardState {
    Unlocked,
    Locked,
}

/// Contracts we call from Credit Manager should not be attempting to execute actions.
/// This prevents reentrancy attacks where a contract we call (that turned evil) deposits
/// into their own credit account and trick our state updates like update_coin_balances.rs which
/// rely on pre-post querying of bank balances of Rover.
/// NOTE: https://twitter.com/larry0x/status/1595919149381079041
pub struct ReentrancyGuard<'a>(Item<'a, GuardState>);

impl<'a> ReentrancyGuard<'a> {
    pub const fn new(namespace: &'a str) -> Self {
        Self(Item::new(namespace))
    }

    /// Ensures the guard is unlocked and sets lock
    pub fn try_lock(&self, storage: &mut dyn Storage) -> ContractResult<()> {
        self.assert_unlocked(storage)?;
        self.transition_state(storage, GuardState::Locked)?;
        Ok(())
    }

    /// Sets guard to unlocked and returns response to be used for callback
    pub fn try_unlock<C>(&self, storage: &mut dyn Storage) -> ContractResult<Response<C>>
    where
        C: Clone + Debug + PartialEq + JsonSchema,
    {
        self.transition_state(storage, GuardState::Unlocked)?;
        Ok(Response::new().add_attribute("action", "remove_reentrancy_guard"))
    }

    fn assert_unlocked(&self, storage: &mut dyn Storage) -> ContractResult<()> {
        match self.state(storage)? {
            GuardState::Locked => {
                Err(ContractError::ReentrancyGuard("Reentrancy guard is active".to_string()))
            }
            GuardState::Unlocked => Ok(()),
        }
    }

    fn state(&self, storage: &dyn Storage) -> StdResult<GuardState> {
        Ok(self.0.may_load(storage)?.unwrap_or(GuardState::Unlocked))
    }

    fn transition_state(
        &self,
        storage: &mut dyn Storage,
        new_state: GuardState,
    ) -> ContractResult<()> {
        let current_state = self.state(storage)?;

        let new_state = match (current_state, new_state) {
            (GuardState::Locked, GuardState::Unlocked) => Ok(GuardState::Unlocked),
            (GuardState::Unlocked, GuardState::Locked) => Ok(GuardState::Locked),
            _ => Err(ContractError::ReentrancyGuard(
                "Invalid reentrancy guard state transition".to_string(),
            )),
        }?;

        Ok(self.0.save(storage, &new_state)?)
    }
}
