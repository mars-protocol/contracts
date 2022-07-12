use cosmwasm_std::{Deps, Env, StdResult, Uint128};
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
        // only amount matters for our testing
        amount: debt_amount,
        // for other attributes we fill in some random value
        denom: "".to_string(),
        asset_label: asset_info.to_string(),
        asset_reference: vec![],
        asset_info: asset_info.into(),
        amount_scaled: Uint128::zero(),
    })
}
