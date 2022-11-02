use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;

use crate::adapters::vault::{
    LockingVaultAmount, UnlockingPositions, VaultAmount, VaultPositionAmount,
    VaultUnlockingPosition,
};

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
                VaultPositionAmount::Unlocked(VaultAmount::new(Uint128::zero()))
            }
            _ => VaultPositionAmount::Locking(LockingVaultAmount {
                locked: VaultAmount::new(Uint128::zero()),
                unlocking: UnlockingPositions::new(vec![]),
            }),
        }
    }
}
