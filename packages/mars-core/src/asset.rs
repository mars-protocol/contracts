use cosmwasm_std::{to_binary, Addr, BankMsg, Coin, CosmosMsg, Deps, StdResult, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::astroport::asset::AssetInfo as AstroportAssetInfo;
use crate::helpers::cw20_get_balance;
use crate::tax::deduct_tax;

/// Represents either a native asset or a cw20. Meant to be used as part of a msg
/// in a contract call and not to be used internally
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Asset {
    Cw20 { contract_addr: String },
    Native { denom: String },
}

impl Asset {
    /// Get label (denom/address as string),
    /// reference (denom/address as bytes, used as key for storage)
    /// and asset type
    pub fn get_attributes(&self) -> (String, Vec<u8>, AssetType) {
        match &self {
            Asset::Native { denom } => {
                let asset_reference = denom.as_bytes().to_vec();
                (denom.to_string(), asset_reference, AssetType::Native)
            }
            Asset::Cw20 { contract_addr } => {
                let asset_reference = contract_addr.as_bytes().to_vec();
                (contract_addr.to_string(), asset_reference, AssetType::Cw20)
            }
        }
    }

    /// Return bytes used as key for storage
    pub fn get_reference(&self) -> Vec<u8> {
        match &self {
            Asset::Native { denom } => denom.as_bytes().to_vec(),
            Asset::Cw20 { contract_addr } => contract_addr.as_bytes().to_vec(),
        }
    }
}

// Cast astroport::asset::AssetInfo into mars_core::asset::Asset so that they can be compared
impl From<&AstroportAssetInfo> for Asset {
    fn from(info: &AstroportAssetInfo) -> Self {
        match info {
            AstroportAssetInfo::Token { contract_addr } => Asset::Cw20 {
                contract_addr: contract_addr.to_string(),
            },
            AstroportAssetInfo::NativeToken { denom } => Asset::Native {
                denom: denom.clone(),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssetType {
    Cw20,
    Native,
}

/// Prepares a message to send the asset from the contract executing the messages to the recipient.
/// If the `AssetType` is `Native`, a "tax" is charged (see [`build_send_native_asset_with_tax_deduction_msg`] for details). Also sender will always be the contract calling this method
/// as it's the only Bank Transfer
/// No tax is charged on `Cw20` asset transfers.
pub fn build_send_asset_with_tax_deduction_msg(
    deps: Deps,
    recipient_address: Addr,
    asset_label: String,
    asset_type: AssetType,
    amount: Uint128,
) -> StdResult<CosmosMsg> {
    match asset_type {
        AssetType::Native => Ok(build_send_native_asset_with_tax_deduction_msg(
            deps,
            recipient_address,
            asset_label,
            amount,
        )?),
        AssetType::Cw20 => build_send_cw20_token_msg(recipient_address, asset_label, amount),
    }
}

/// Prepare BankMsg::Send message.
/// When doing native transfers a "tax" is charged.
/// The actual amount taken from the contract is: amount + tax.
/// Instead of sending amount, send: amount - compute_tax(amount).
pub fn build_send_native_asset_with_tax_deduction_msg(
    deps: Deps,
    recipient_address: Addr,
    denom: String,
    amount: Uint128,
) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Bank(BankMsg::Send {
        to_address: recipient_address.into(),
        amount: vec![deduct_tax(deps, Coin { denom, amount })?],
    }))
}

pub fn build_send_cw20_token_msg(
    recipient_address: Addr,
    token_contract_address_unchecked: String,
    amount: Uint128,
) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token_contract_address_unchecked,
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: recipient_address.into(),
            amount,
        })?,
        funds: vec![],
    }))
}

/// Gets asset balance for the given address
pub fn get_asset_balance(
    deps: Deps,
    address: Addr,
    asset_label: String,
    asset_type: AssetType,
) -> StdResult<Uint128> {
    match asset_type {
        AssetType::Native => {
            let balance_query = deps.querier.query_balance(address, asset_label.as_str())?;
            Ok(balance_query.amount)
        }
        AssetType::Cw20 => {
            let token_addr = deps.api.addr_validate(&asset_label)?;
            cw20_get_balance(&deps.querier, token_addr, address)
        }
    }
}
