use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;
use std::ops::Add;

use crate::adapters::vault::VaultUnlockingPosition;
use crate::error::{ContractError, ContractResult};

#[cw_serde]
pub enum UpdateType {
    Increment(Uint128),
    Decrement(Uint128),
}

#[cw_serde]
pub enum UnlockingChange {
    Add(VaultUnlockingPosition),
    Decrement { id: u64, amount: Uint128 },
}

#[cw_serde]
pub enum VaultPositionUpdate {
    Unlocked(UpdateType),
    Locked(UpdateType),
    Unlocking(UnlockingChange),
}

impl VaultPositionUpdate {
    pub fn default_amount(&self) -> VaultPositionAmount {
        match self {
            VaultPositionUpdate::Unlocked { .. } => {
                VaultPositionAmount::Unlocked(VaultAmount(Uint128::zero()))
            }
            _ => VaultPositionAmount::Locking(LockingVaultAmount {
                locked: VaultAmount(Uint128::zero()),
                unlocking: UnlockingPositions(vec![]),
            }),
        }
    }
}

pub type VaultPositionAmount = VaultPositionAmountBase<VaultAmount, LockingVaultAmount>;

impl Total for VaultPositionAmount {
    fn total(&self) -> Uint128 {
        match self {
            VaultPositionAmount::Unlocked(a) => a.total(),
            VaultPositionAmount::Locking(a) => a.total(),
        }
    }
}

impl VaultPositionAmount {
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

    pub fn unlocking(&self) -> Vec<VaultUnlockingPosition> {
        match self {
            VaultPositionAmount::Locking(amount) => amount.unlocking.positions(),
            _ => vec![],
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

pub trait Total {
    fn total(&self) -> Uint128;
}

#[cw_serde]
pub enum VaultPositionAmountBase<U, L>
where
    U: Total,
    L: Total,
{
    Unlocked(U),
    Locking(L),
}

#[cw_serde]
pub struct VaultAmount(Uint128);

impl Total for VaultAmount {
    fn total(&self) -> Uint128 {
        self.0
    }
}

impl VaultAmount {
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

impl Total for LockingVaultAmount {
    fn total(&self) -> Uint128 {
        self.locked.total().add(self.unlocking.total())
    }
}

#[cw_serde]
pub struct UnlockingPositions(Vec<VaultUnlockingPosition>);

impl UnlockingPositions {
    pub fn positions(&self) -> Vec<VaultUnlockingPosition> {
        self.0.clone()
    }

    pub fn total(&self) -> Uint128 {
        self.0.iter().map(|u| u.amount).sum()
    }

    pub fn add(&mut self, position: VaultUnlockingPosition) -> ContractResult<()> {
        self.0.push(position);
        Ok(())
    }

    pub fn decrement(&mut self, id: u64, amount: Uint128) -> ContractResult<()> {
        let res = self.0.iter_mut().find(|p| p.id == id);
        match res {
            Some(p) => {
                let new_amount = p.amount.checked_sub(amount)?;
                if new_amount.is_zero() {
                    self.remove(id)?;
                } else {
                    p.amount = new_amount;
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
