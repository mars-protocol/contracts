use cosmwasm_std::{Addr, Uint128};
use cw_asset::AssetInfoKey;
use cw_storage_plus::Map;

pub const DEBT_AMOUNT: Map<(Addr, AssetInfoKey), Uint128> = Map::new("debt_amount");
