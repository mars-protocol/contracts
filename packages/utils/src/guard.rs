use cosmwasm_schema::cw_serde;
use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::Item;

use crate::error::GuardError;

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
    pub fn try_lock(&self, storage: &mut dyn Storage) -> Result<(), GuardError> {
        self.assert_unlocked(storage)?;
        self.transition_state(storage, GuardState::Locked)?;
        Ok(())
    }

    /// Sets guard to unlocked
    pub fn try_unlock(&self, storage: &mut dyn Storage) -> Result<(), GuardError> {
        self.transition_state(storage, GuardState::Unlocked)?;
        Ok(())
    }

    pub fn assert_unlocked(&self, storage: &dyn Storage) -> Result<(), GuardError> {
        match self.state(storage)? {
            GuardState::Locked => Err(GuardError::Active {}),
            GuardState::Unlocked => Ok(()),
        }
    }

    pub fn assert_locked(&self, storage: &dyn Storage) -> Result<(), GuardError> {
        match self.state(storage)? {
            GuardState::Locked => Ok(()),
            GuardState::Unlocked => Err(GuardError::Inactive {}),
        }
    }

    fn state(&self, storage: &dyn Storage) -> StdResult<GuardState> {
        Ok(self.0.may_load(storage)?.unwrap_or(GuardState::Unlocked))
    }

    fn transition_state(
        &self,
        storage: &mut dyn Storage,
        new_state: GuardState,
    ) -> Result<(), GuardError> {
        let current_state = self.state(storage)?;

        let new_state = match (current_state, new_state) {
            (GuardState::Locked, GuardState::Unlocked) => Ok(GuardState::Unlocked),
            (GuardState::Unlocked, GuardState::Locked) => Ok(GuardState::Locked),
            _ => Err(GuardError::InvalidState {}),
        }?;

        Ok(self.0.save(storage, &new_state)?)
    }
}
