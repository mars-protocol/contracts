use std::ops::Add;

use cosmwasm_std::{from_binary, Api, DepsMut, MessageInfo, Response, StdResult, Storage, Uint128};
use cw20::Cw20ReceiveMsg;
use cw_asset::{Asset, AssetInfo, AssetInfoUnchecked, AssetList, AssetUnchecked};

use rover::msg::execute::ReceiveMsg;

use crate::error::ContractError;
use crate::error::ContractError::{FundsMismatch, NotWhitelisted, WrongDepositMethodForCW20};
use crate::execute::assert_is_token_owner;
use crate::state::{ALLOWED_ASSETS, ASSETS};

pub fn native_deposit(
    storage: &mut dyn Storage,
    api: &dyn Api,
    response: Response,
    nft_token_id: &str,
    asset_unchecked: &AssetUnchecked,
    received_coins: &mut AssetList,
) -> Result<Response, ContractError> {
    let asset = asset_unchecked.check(api, None)?;
    assert_asset_is_whitelisted(storage, &asset.info)?;

    if asset.amount.is_zero() {
        return Ok(response);
    }

    match &asset.info {
        AssetInfo::Native(_) => {
            assert_sent_fund(&asset, received_coins)?;
            received_coins.deduct(&asset)?;
        }
        AssetInfo::Cw20(_) => {
            return Err(WrongDepositMethodForCW20 {});
        }
    }

    // increase the user asset amount
    increment_position(storage, nft_token_id, &asset.info, &asset.amount)?;

    Ok(response
        .add_attribute("deposit_received", asset.to_string())
        .add_attribute("action", "rover/credit_manager/callback/deposit"))
}

/// Assert that fund of exactly the same type and amount was sent along with a message
fn assert_sent_fund(expected: &Asset, received_coins: &AssetList) -> Result<(), ContractError> {
    let received_amount = if let Some(coin) = received_coins.find(&expected.info) {
        coin.amount
    } else {
        Uint128::zero()
    };

    if received_amount != expected.amount {
        return Err(FundsMismatch {
            expected: expected.amount,
            received: received_amount,
        });
    }

    Ok(())
}

pub fn receive_cw20(
    deps: DepsMut,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    match from_binary(&cw20_msg.msg)? {
        ReceiveMsg::Deposit { token_id } => {
            let sender = deps.api.addr_validate(&cw20_msg.sender)?;
            assert_is_token_owner(&deps, &sender, &token_id)?;
            let asset = AssetInfoUnchecked::cw20(&info.sender).check(deps.api, None)?;
            assert_asset_is_whitelisted(deps.storage, &asset)?;
            increment_position(deps.storage, &token_id, &asset, &cw20_msg.amount)?;
            Ok(Response::new()
                .add_attribute("deposit_received", asset.to_string())
                .add_attribute("action", "rover/execute/receive_cw20"))
        }
    }
}

fn assert_asset_is_whitelisted(
    storage: &mut dyn Storage,
    asset: &AssetInfo,
) -> Result<(), ContractError> {
    let is_whitelisted = ALLOWED_ASSETS.has(storage, asset.into());
    if !is_whitelisted {
        return Err(NotWhitelisted(asset.to_string()));
    }
    Ok(())
}

fn increment_position(
    storage: &mut dyn Storage,
    token_id: &str,
    asset: &AssetInfo,
    amount: &Uint128,
) -> StdResult<()> {
    ASSETS.update(
        storage,
        (token_id, asset.into()),
        |value_opt| -> StdResult<_> { Ok(value_opt.unwrap_or(Uint128::zero()).add(amount)) },
    )?;
    Ok(())
}
