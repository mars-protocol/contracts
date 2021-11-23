use cosmwasm_std::{Addr, Env, Order, StdResult, Storage, Uint128};
use cw_storage_plus::{Bound, Map, Prefix, U64Key};

// STATE
pub const TOTAL_SUPPLY_SNAPSHOTS: Map<U64Key, Uint128> = Map::new("total_supply_snapshots");
pub const BALANCE_SNAPSHOTS: Map<(&Addr, U64Key), Uint128> = Map::new("balance_snapshots");

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

// BALANCE

pub fn capture_balance_snapshot(
    storage: &mut dyn Storage,
    env: &Env,
    addr: &Addr,
    balance: Uint128,
) -> StdResult<()> {
    BALANCE_SNAPSHOTS.save(storage, (addr, U64Key::new(env.block.height)), &balance)
}

pub fn get_balance_snapshot_value_at(
    storage: &dyn Storage,
    addr: &Addr,
    block: u64,
) -> StdResult<Uint128> {
    get_snapshot_value_at(storage, BALANCE_SNAPSHOTS.prefix(addr), block)
}

// TOTAL SUPPLY

pub fn capture_total_supply_snapshot(
    storage: &mut dyn Storage,
    env: &Env,
    total_supply: Uint128,
) -> StdResult<()> {
    TOTAL_SUPPLY_SNAPSHOTS.save(storage, U64Key::new(env.block.height), &total_supply)
}

pub fn get_total_supply_snapshot_value_at(storage: &dyn Storage, block: u64) -> StdResult<Uint128> {
    get_snapshot_value_at(storage, TOTAL_SUPPLY_SNAPSHOTS.prefix(()), block)
}
