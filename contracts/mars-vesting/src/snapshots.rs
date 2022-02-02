use cosmwasm_std::{Addr, Order, StdResult, Storage, Uint128};
use cw_storage_plus::{Bound, Map, Prefix, U64Key};

// STATE

pub const VOTING_POWER_SNAPSHOTS: Map<(&Addr, U64Key), Uint128> = Map::new("voting_powers");
pub const TOTAL_VOTING_POWER_SNAPSHOTS: Map<U64Key, Uint128> = Map::new("total_voting_powers");

// CORE

fn get_snapshot_value_at(
    storage: &dyn Storage,
    prefix: Prefix<Uint128>,
    block: u64,
) -> StdResult<Uint128> {
    // Look for the last value recorded before the current block (if none then value is zero)
    let end = Bound::inclusive(U64Key::new(block));
    let last_value_up_to_block = prefix
        .range(storage, None, Some(end), Order::Descending)
        .next();

    if let Some(value) = last_value_up_to_block {
        let (_, v) = value?;
        return Ok(v);
    }

    Ok(Uint128::zero())
}

// VOTING POWER

pub fn capture_voting_power_snapshot(
    storage: &mut dyn Storage,
    user_address: &Addr,
    block: u64,
    voting_power: Uint128,
) -> StdResult<()> {
    VOTING_POWER_SNAPSHOTS.save(storage, (user_address, block.into()), &voting_power)
}

pub fn get_voting_power_value_at(
    storage: &dyn Storage,
    user_address: &Addr,
    block: u64,
) -> StdResult<Uint128> {
    get_snapshot_value_at(storage, VOTING_POWER_SNAPSHOTS.prefix(user_address), block)
}

// TOTAL VOTING POWER

pub fn capture_total_voting_power_snapshot(
    storage: &mut dyn Storage,
    block: u64,
    total_voting_power: Uint128,
) -> StdResult<()> {
    TOTAL_VOTING_POWER_SNAPSHOTS.save(storage, block.into(), &total_voting_power)
}

pub fn get_total_voting_power_value_at(storage: &dyn Storage, block: u64) -> StdResult<Uint128> {
    get_snapshot_value_at(storage, TOTAL_VOTING_POWER_SNAPSHOTS.prefix(()), block)
}
