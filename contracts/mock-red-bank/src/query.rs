use cosmwasm_std::{Deps, Env, StdResult};
use cw_asset::AssetInfoUnchecked;

use crate::helpers::load_debt_amount;
use crate::msg::UserAssetDebtResponse;

pub fn query_debt(
    deps: Deps,
    _env: Env,
    user_address: String,
    asset_info_unchecked: AssetInfoUnchecked,
) -> StdResult<UserAssetDebtResponse> {
    let user_addr = deps.api.addr_validate(&user_address)?;
    let asset_info = asset_info_unchecked.check(deps.api, None)?;
    let debt_amount = load_debt_amount(deps.storage, &user_addr, &asset_info);
    Ok(UserAssetDebtResponse {
        amount: debt_amount,
        asset_info: asset_info.into(),
    })
}
