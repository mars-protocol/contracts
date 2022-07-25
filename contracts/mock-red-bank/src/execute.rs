use cosmwasm_std::{DepsMut, MessageInfo, Response, StdResult, Uint128};
use cw_asset::AssetUnchecked;

use crate::helpers::load_debt_amount;
use crate::state::DEBT_AMOUNT;

pub fn execute_borrow(
    deps: DepsMut,
    info: MessageInfo,
    asset_unchecked: AssetUnchecked,
) -> StdResult<Response> {
    let asset = asset_unchecked.check(deps.api, None)?;
    let debt_amount = load_debt_amount(deps.storage, &info.sender, &asset.info);

    DEBT_AMOUNT.save(
        deps.storage,
        (info.sender.clone(), asset.info.clone().into()),
        &(debt_amount + asset.amount + Uint128::from(1u128)), // The extra unit is simulated accrued interest
    )?;

    Ok(Response::new().add_message(asset.transfer_msg(&info.sender)?))
}
