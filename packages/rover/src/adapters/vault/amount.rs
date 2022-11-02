use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;

use crate::adapters::vault::{
    UnlockingChange, UpdateType, VaultPositionUpdate, VaultUnlockingPosition,
};
use crate::error::{ContractError, ContractResult};

#[cw_serde]
pub enum VaultPositionAmount {
    Unlocked(VaultAmount),
    Locking(LockingVaultAmount),
}

impl VaultPositionAmount {
    pub fn is_empty(&self) -> bool {
        self.unlocked().is_zero() && self.locked().is_zero() && self.unlocking().is_empty()
    }

    pub fn unlocked(&self) -> Uint128 {
        match self {
            VaultPositionAmount::Unlocked(amount) => amount.total(),
            _ => Uint128::zero(),
        }
    }

    pub fn locked(&self) -> Uint128 {
        match self {
            VaultPositionAmount::Locking(amount) => amount.locked.total(),
            _ => Uint128::zero(),
        }
    }

    pub fn unlocking(&self) -> UnlockingPositions {
        match self {
            VaultPositionAmount::Locking(amount) => amount.unlocking.clone(),
            _ => UnlockingPositions(vec![]),
        }
    }

    pub fn get_unlocking_position(&self, id: u64) -> Option<VaultUnlockingPosition> {
        match self {
            VaultPositionAmount::Locking(amount) => amount
                .unlocking
                .positions()
                .iter()
                .find(|p| p.id == id)
                .cloned(),
            _ => None,
        }
    }

    pub fn update(&mut self, update: VaultPositionUpdate) -> ContractResult<()> {
        match self {
            VaultPositionAmount::Unlocked(amount) => match update {
                VaultPositionUpdate::Unlocked(u) => match u {
                    UpdateType::Increment(a) => amount.increment(a),
                    UpdateType::Decrement(a) => amount.decrement(a),
                },
                _ => Err(ContractError::MismatchedVaultType {}),
            },
            VaultPositionAmount::Locking(amount) => match update {
                VaultPositionUpdate::Locked(u) => match u {
                    UpdateType::Increment(a) => amount.locked.increment(a),
                    UpdateType::Decrement(a) => amount.locked.decrement(a),
                },
                VaultPositionUpdate::Unlocking(u) => match u {
                    UnlockingChange::Add(p) => amount.unlocking.add(p),
                    UnlockingChange::Decrement { id, amount: a } => {
                        amount.unlocking.decrement(id, a)
                    }
                },
                _ => Err(ContractError::MismatchedVaultType {}),
            },
        }
    }
}

#[cw_serde]
pub struct VaultAmount(Uint128);

impl VaultAmount {
    pub fn new(amount: Uint128) -> VaultAmount {
        VaultAmount(amount)
    }

    pub fn total(&self) -> Uint128 {
        self.0
    }

    pub fn increment(&mut self, amount: Uint128) -> ContractResult<()> {
        self.0 = self.0.checked_add(amount)?;
        Ok(())
    }

    pub fn decrement(&mut self, amount: Uint128) -> ContractResult<()> {
        self.0 = self.0.checked_sub(amount)?;
        Ok(())
    }
}

#[cw_serde]
pub struct LockingVaultAmount {
    pub locked: VaultAmount,
    pub unlocking: UnlockingPositions,
}

#[cw_serde]
pub struct UnlockingPositions(Vec<VaultUnlockingPosition>);

impl UnlockingPositions {
    pub fn new(positions: Vec<VaultUnlockingPosition>) -> UnlockingPositions {
        UnlockingPositions(positions)
    }

    pub fn positions(&self) -> Vec<VaultUnlockingPosition> {
        self.0.clone()
    }

    pub fn total(&self) -> Uint128 {
        self.0.iter().map(|u| u.coin.amount).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn add(&mut self, position: VaultUnlockingPosition) -> ContractResult<()> {
        self.0.push(position);
        Ok(())
    }

    pub fn decrement(&mut self, id: u64, amount: Uint128) -> ContractResult<()> {
        let res = self.0.iter_mut().find(|p| p.id == id);
        match res {
            Some(p) => {
                let new_amount = p.coin.amount.checked_sub(amount)?;
                if new_amount.is_zero() {
                    self.remove(id)?;
                } else {
                    p.coin.amount = new_amount;
                }
            }
            None => return Err(ContractError::NoPositionMatch(id.to_string())),
        }
        Ok(())
    }

    pub fn remove(&mut self, id: u64) -> ContractResult<()> {
        self.0.retain(|p| p.id != id);
        Ok(())
    }
}
