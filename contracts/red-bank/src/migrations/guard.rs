use cosmwasm_schema::cw_serde;
use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::Item;

use crate::error::ContractError;

#[cw_serde]
pub enum GuardState {
    Unlocked,
    Locked,
}

pub struct Guard<'a>(Item<'a, GuardState>);

impl<'a> Guard<'a> {
    pub const fn new(namespace: &'a str) -> Self {
        Self(Item::new(namespace))
    }

    /// Ensures the guard is unlocked and sets lock
    pub fn try_lock(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        self.assert_unlocked(storage)?;
        self.transition_state(storage, GuardState::Locked)?;
        Ok(())
    }

    /// Sets guard to unlocked
    pub fn try_unlock(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        self.transition_state(storage, GuardState::Unlocked)?;
        Ok(())
    }

    pub fn assert_unlocked(&self, storage: &dyn Storage) -> Result<(), ContractError> {
        match self.state(storage)? {
            GuardState::Locked => Err(ContractError::Guard("Guard is active".to_string())),
            GuardState::Unlocked => Ok(()),
        }
    }

    pub fn assert_locked(&self, storage: &dyn Storage) -> Result<(), ContractError> {
        match self.state(storage)? {
            GuardState::Locked => Ok(()),
            GuardState::Unlocked => Err(ContractError::Guard("Guard is inactive".to_string())),
        }
    }

    fn state(&self, storage: &dyn Storage) -> StdResult<GuardState> {
        Ok(self.0.may_load(storage)?.unwrap_or(GuardState::Unlocked))
    }

    fn transition_state(
        &self,
        storage: &mut dyn Storage,
        new_state: GuardState,
    ) -> Result<(), ContractError> {
        let current_state = self.state(storage)?;

        let new_state = match (current_state, new_state) {
            (GuardState::Locked, GuardState::Unlocked) => Ok(GuardState::Unlocked),
            (GuardState::Unlocked, GuardState::Locked) => Ok(GuardState::Locked),
            _ => Err(ContractError::Guard("Invalid guard state transition".to_string())),
        }?;

        Ok(self.0.save(storage, &new_state)?)
    }
}
