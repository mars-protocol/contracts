use cosmwasm_std::{
    coins, to_binary, Addr, BankMsg, Coin, CosmosMsg, Deps, StdResult, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::helpers::cw20_get_balance;

/// Represents either a native asset or a cw20. Meant to be used as part of a msg
/// in a contract call and not to be used internally
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Asset {
    Cw20 {
        contract_addr: String,
    },
    Native {
        denom: String,
    },
}

impl Asset {
    /// Get label (denom/address as string),
    /// reference (denom/address as bytes, used as key for storage)
    /// and asset type
    pub fn get_attributes(&self) -> (String, Vec<u8>, AssetType) {
        match &self {
            Asset::Native {
                denom,
            } => {
                let asset_reference = denom.as_bytes().to_vec();
                (denom.clone(), asset_reference, AssetType::Native)
            }
            Asset::Cw20 {
                contract_addr,
            } => {
                let lower_case_contract_addr = contract_addr.to_lowercase();
                let asset_reference = lower_case_contract_addr.as_bytes().to_vec();
                (lower_case_contract_addr, asset_reference, AssetType::Cw20)
            }
        }
    }

    /// Return bytes used as key for storage
    pub fn get_reference(&self) -> Vec<u8> {
        match &self {
            Asset::Native {
                denom,
            } => denom.as_bytes().to_vec(),
            Asset::Cw20 {
                contract_addr,
            } => contract_addr.to_lowercase().as_bytes().to_vec(),
        }
    }
}

impl From<&Coin> for Asset {
    fn from(coin: &Coin) -> Self {
        Asset::Native {
            denom: coin.denom.clone(),
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
/// Sender will always be the contract calling this method
/// as it's the only Bank Transfer
pub fn build_send_asset_msg(
    recipient_address: Addr,
    asset_label: String,
    asset_type: AssetType,
    amount: Uint128,
) -> StdResult<CosmosMsg> {
    match asset_type {
        AssetType::Native => {
            Ok(build_send_native_asset_msg(recipient_address, asset_label, amount)?)
        }
        AssetType::Cw20 => build_send_cw20_token_msg(recipient_address, asset_label, amount),
    }
}

/// Prepare BankMsg::Send message.
pub fn build_send_native_asset_msg(
    recipient_address: Addr,
    denom: String,
    amount: Uint128,
) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Bank(BankMsg::Send {
        to_address: recipient_address.into(),
        amount: coins(amount.u128(), denom),
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
