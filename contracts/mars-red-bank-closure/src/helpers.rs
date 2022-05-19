use cosmwasm_std::{
    coin, to_binary, Addr, Api, BankMsg, CosmosMsg, QuerierWrapper, QueryRequest, StdResult,
    Uint128, WasmMsg, WasmQuery,
};
use cw20::{AllAccountsResponse, BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg};

use mars_core::asset::Asset;

/// Get the first 10 token owners and their respective token balances
pub fn cw20_get_owners_balances(
    querier: &QuerierWrapper,
    api: &dyn Api,
    token_addr: &Addr,
) -> StdResult<Vec<(Addr, Uint128)>> {
    let AllAccountsResponse { accounts } =
        querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: token_addr.to_string(),
            msg: to_binary(&Cw20QueryMsg::AllAccounts {
                start_after: None,
                limit: Some(10),
            })?,
        }))?;

    accounts
        .iter()
        .map(|acct| {
            let acct_addr = api.addr_validate(acct)?;
            let BalanceResponse { balance } =
                querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                    contract_addr: token_addr.to_string(),
                    msg: to_binary(&Cw20QueryMsg::Balance {
                        address: acct_addr.to_string(),
                    })?,
                }))?;
            Ok((acct_addr, balance))
        })
        .collect::<StdResult<Vec<_>>>()
}

pub fn get_asset_balance<T: Into<String>>(
    querier: &QuerierWrapper,
    asset: &Asset,
    account: T,
) -> StdResult<Uint128> {
    let balance = match &asset {
        Asset::Cw20 { contract_addr } => {
            let query: BalanceResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: contract_addr.clone(),
                msg: to_binary(&Cw20QueryMsg::Balance {
                    address: account.into(),
                })?,
            }))?;
            query.balance
        }
        Asset::Native { denom } => {
            let balance_query = querier.query_balance(account, denom)?;
            balance_query.amount
        }
    };
    Ok(balance)
}

pub fn build_transfer_asset_msg(
    asset: &Asset,
    amount: Uint128,
    recipient: &Addr,
) -> StdResult<CosmosMsg> {
    let msg = match asset {
        Asset::Cw20 { contract_addr } => CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_addr.clone(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: recipient.to_string(),
                amount,
            })?,
            funds: vec![],
        }),
        Asset::Native { denom } => CosmosMsg::Bank(BankMsg::Send {
            to_address: recipient.to_string(),
            amount: vec![coin(amount.u128(), denom)],
        }),
    };
    Ok(msg)
}
