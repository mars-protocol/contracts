use cosmwasm_std::{Coin, Deps, StdError, StdResult, Uint128};

use mars_outpost::asset::AssetType;
use mars_outpost::helpers::cw20_get_symbol;
use mars_outpost::red_bank::Market;

use crate::error::ContractError;
use crate::state::{MARKETS, MARKET_REFERENCES_BY_INDEX};

// native coins
pub fn get_denom_amount_from_coins(coins: &[Coin], denom: &str) -> Result<Uint128, ContractError> {
    if coins.len() == 1 && coins[0].denom == denom {
        Ok(coins[0].amount)
    } else {
        Err(ContractError::InvalidNativeCoinsSent {
            denom: denom.to_string(),
        })
    }
}

pub fn get_asset_identifiers(
    deps: Deps,
    asset_reference: Vec<u8>,
    asset_type: AssetType,
) -> StdResult<(String, String)> {
    let asset_label = String::from_utf8(asset_reference)?;
    let denom = get_asset_denom(deps, &asset_label, asset_type)?;
    Ok((denom, asset_label))
}

pub fn get_asset_denom(deps: Deps, asset_label: &str, asset_type: AssetType) -> StdResult<String> {
    match asset_type {
        AssetType::Native => Ok(asset_label.to_string()),
        AssetType::Cw20 => {
            let cw20_contract_address = deps.api.addr_validate(asset_label)?;
            match cw20_get_symbol(&deps.querier, cw20_contract_address.clone()) {
                Ok(symbol) => Ok(symbol),
                Err(_) => {
                    return Err(StdError::generic_err(format!(
                        "failed to get symbol from cw20 contract address: {}",
                        cw20_contract_address
                    )));
                }
            }
        }
    }
}

pub fn market_get_from_index(deps: &Deps, index: u32) -> StdResult<(Vec<u8>, Market)> {
    let asset_reference_vec = match MARKET_REFERENCES_BY_INDEX.load(deps.storage, index) {
        Ok(asset_reference_vec) => asset_reference_vec,
        Err(_) => {
            return Err(StdError::generic_err(format!(
                "no market reference exists with index: {}",
                index
            )))
        }
    };

    match MARKETS.load(deps.storage, asset_reference_vec.as_slice()) {
        Ok(asset_market) => Ok((asset_reference_vec, asset_market)),
        Err(_) => Err(StdError::generic_err(format!(
            "no asset market exists with asset reference: {}",
            String::from_utf8(asset_reference_vec).expect("Found invalid UTF-8")
        ))),
    }
}

// bitwise operations
/// Gets bit: true: 1, false: 0
pub fn get_bit(bitmap: Uint128, index: u32) -> StdResult<bool> {
    if index >= 128 {
        return Err(StdError::generic_err("index out of range"));
    }
    Ok(((bitmap.u128() >> index) & 1) == 1)
}

/// Sets bit to 1
pub fn set_bit(bitmap: &mut Uint128, index: u32) -> StdResult<()> {
    if index >= 128 {
        return Err(StdError::generic_err("index out of range"));
    }
    *bitmap = Uint128::from(bitmap.u128() | (1 << index));
    Ok(())
}

/// Sets bit to 0
pub fn unset_bit(bitmap: &mut Uint128, index: u32) -> StdResult<()> {
    if index >= 128 {
        return Err(StdError::generic_err("index out of range"));
    }
    *bitmap = Uint128::from(bitmap.u128() & !(1 << index));
    Ok(())
}
