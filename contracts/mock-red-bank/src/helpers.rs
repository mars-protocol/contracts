use cosmwasm_std::{Addr, Storage, Uint128};
use cw_asset::AssetInfo;

use crate::state::DEBT_AMOUNT;

pub fn load_debt_amount(storage: &dyn Storage, user: &Addr, asset: &AssetInfo) -> Uint128 {
    DEBT_AMOUNT
        .load(storage, (user.clone(), asset.into()))
        .unwrap_or(Uint128::zero())
}
