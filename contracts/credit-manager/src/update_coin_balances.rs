use cosmwasm_std::{
    Addr, BalanceResponse, BankQuery, Coin, DepsMut, Env, QuerierWrapper, QueryRequest, Response,
    StdResult,
};
use mars_rover::error::ContractResult;

use crate::utils::{decrement_coin_balance, increment_coin_balance};

pub fn query_balance(querier: &QuerierWrapper, addr: &Addr, denom: &str) -> StdResult<Coin> {
    let res: BalanceResponse = querier.query(&QueryRequest::Bank(BankQuery::Balance {
        address: addr.to_string(),
        denom: denom.to_string(),
    }))?;
    Ok(Coin {
        denom: denom.to_string(),
        amount: res.amount.amount,
    })
}

pub fn update_coin_balance(
    deps: DepsMut,
    env: Env,
    account_id: &str,
    prev: &Coin,
) -> ContractResult<Response> {
    let curr = query_balance(&deps.querier, &env.contract.address, &prev.denom)?;
    if prev.amount > curr.amount {
        let amount_to_reduce = prev.amount.checked_sub(curr.amount)?;
        let coin_to_reduce = Coin {
            denom: curr.denom,
            amount: amount_to_reduce,
        };
        decrement_coin_balance(deps.storage, account_id, &coin_to_reduce)?;
        Ok(Response::new()
            .add_attribute("action", "rover/credit-manager/update_coin_balance")
            .add_attribute("account_id", account_id)
            .add_attribute("coin_decremented", coin_to_reduce.to_string()))
    } else {
        let amount_to_increment = curr.amount.checked_sub(prev.amount)?;
        let coin_to_increment = Coin {
            denom: curr.denom,
            amount: amount_to_increment,
        };
        increment_coin_balance(deps.storage, account_id, &coin_to_increment)?;
        Ok(Response::new()
            .add_attribute("action", "rover/credit-manager/update_coin_balance")
            .add_attribute("account_id", account_id)
            .add_attribute("coin_incremented", coin_to_increment.to_string()))
    }
}
