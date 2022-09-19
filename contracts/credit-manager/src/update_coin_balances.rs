use cosmwasm_std::{
    Addr, BalanceResponse, BankQuery, Coin, Deps, DepsMut, Env, QueryRequest, Response, StdResult,
};

use rover::error::ContractResult;

use crate::utils::{decrement_coin_balance, increment_coin_balance};

pub fn query_balance(deps: Deps, addr: &Addr, denom: &str) -> StdResult<Coin> {
    let res: BalanceResponse = deps.querier.query(&QueryRequest::Bank(BankQuery::Balance {
        address: addr.to_string(),
        denom: denom.to_string(),
    }))?;
    Ok(Coin {
        denom: denom.to_string(),
        amount: res.amount.amount,
    })
}

pub fn query_balances(deps: Deps, addr: &Addr, denoms: &[&str]) -> StdResult<Vec<Coin>> {
    denoms
        .iter()
        .map(|denom| query_balance(deps, addr, denom))
        .collect()
}

pub fn update_coin_balances(
    deps: DepsMut,
    env: Env,
    account_id: &str,
    previous_balances: &[Coin],
) -> ContractResult<Response> {
    let mut response = Response::new();

    for prev in previous_balances {
        let curr = query_balance(deps.as_ref(), &env.contract.address, &prev.denom)?;
        if prev.amount > curr.amount {
            let amount_to_reduce = prev.amount.checked_sub(curr.amount)?;
            decrement_coin_balance(
                deps.storage,
                account_id,
                &Coin {
                    denom: curr.denom.clone(),
                    amount: amount_to_reduce,
                },
            )?;
            response = response
                .add_attribute("denom", curr.denom.clone())
                .add_attribute("decremented", amount_to_reduce);
        } else {
            let amount_to_increment = curr.amount.checked_sub(prev.amount)?;
            increment_coin_balance(
                deps.storage,
                account_id,
                &Coin {
                    denom: curr.denom.clone(),
                    amount: amount_to_increment,
                },
            )?;
            response = response
                .add_attribute("denom", curr.denom.clone())
                .add_attribute("incremented", amount_to_increment);
        }
    }

    Ok(response.add_attribute("action", "rover/credit_manager/update_coin_balance"))
}
