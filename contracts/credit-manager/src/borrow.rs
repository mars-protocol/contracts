use cosmwasm_std::{DepsMut, Env, Response, StdResult, Uint128};
use cw_asset::Asset;

use crate::deposit::assert_asset_is_whitelisted;
use rover::error::ContractError;
use rover::error::ContractError::NoAmount;

use crate::state::{ASSETS, DEBT_SHARES, RED_BANK, TOTAL_DEBT_SHARES};

pub static DEFAULT_DEBT_UNITS_PER_ASSET_BORROWED: Uint128 = Uint128::new(1_000_000);

/// calculate by how many the user's debt units should be increased
/// if total debt is zero, then we define 1 unit of asset borrowed = 1,000,000 debt unit
/// else, get debt ownership % and multiply by total existing shares
///
/// increment total debt shares, token debt shares, and asset amount
pub fn borrow(
    deps: DepsMut,
    env: Env,
    token_id: &str,
    asset: Asset,
) -> Result<Response, ContractError> {
    if asset.amount.is_zero() {
        return Err(NoAmount {});
    }

    assert_asset_is_whitelisted(deps.storage, &asset.info)?;

    let red_bank = RED_BANK.load(deps.storage)?;
    let total_debt_amount =
        red_bank.query_user_debt(&deps.querier, &env.contract.address, &asset.info)?;

    let debt_shares_to_add = if total_debt_amount.is_zero() {
        asset
            .amount
            .checked_mul(DEFAULT_DEBT_UNITS_PER_ASSET_BORROWED)?
    } else {
        TOTAL_DEBT_SHARES
            .load(deps.storage, asset.clone().info.into())?
            .checked_multiply_ratio(asset.amount, total_debt_amount)?
    };

    TOTAL_DEBT_SHARES.update(
        deps.storage,
        asset.clone().info.into(),
        |shares| -> StdResult<_> { Ok(shares.unwrap_or(Uint128::zero()) + debt_shares_to_add) },
    )?;

    DEBT_SHARES.update(
        deps.storage,
        (token_id, asset.clone().info.into()),
        |current_debt| -> StdResult<_> {
            Ok(current_debt.unwrap_or(Uint128::zero()) + debt_shares_to_add)
        },
    )?;

    ASSETS.update(
        deps.storage,
        (token_id, asset.clone().info.into()),
        |amount| -> StdResult<_> { Ok(amount.unwrap_or(Uint128::zero()) + asset.amount) },
    )?;

    Ok(Response::new()
        .add_message(red_bank.borrow_msg(&asset)?)
        .add_attribute("action", "rover/credit_manager/borrow")
        .add_attribute("debt_shares_added", debt_shares_to_add)
        .add_attribute("assets_borrowed", asset.amount))
}
