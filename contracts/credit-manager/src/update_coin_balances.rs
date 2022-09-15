use cosmwasm_std::{
    BalanceResponse, BankQuery, Coin, Deps, DepsMut, Env, QueryRequest, Response, StdResult,
};

use rover::error::ContractResult;
use rover::NftTokenId;

use crate::utils::{decrement_coin_balance, increment_coin_balance};

pub fn query_balance(deps: Deps, env: &Env, denom: &str) -> StdResult<Coin> {
    let res: BalanceResponse = deps.querier.query(&QueryRequest::Bank(BankQuery::Balance {
        address: env.contract.address.to_string(),
        denom: denom.to_string(),
    }))?;
    Ok(Coin {
        denom: denom.to_string(),
        amount: res.amount.amount,
    })
}

pub fn update_coin_balances(
    deps: DepsMut,
    env: Env,
    token_id: NftTokenId,
    previous_balances: &[Coin],
) -> ContractResult<Response> {
    let mut response = Response::new();

    for prev in previous_balances {
        let curr = query_balance(deps.as_ref(), &env, &prev.denom)?;
        if prev.amount > curr.amount {
            let new_amount = prev.amount.checked_sub(curr.amount)?;
            decrement_coin_balance(
                deps.storage,
                token_id,
                &Coin {
                    denom: curr.denom.clone(),
                    amount: new_amount,
                },
            )?;
            response = response
                .clone()
                .add_attribute("denom", curr.denom.clone())
                .add_attribute("decremented", new_amount);
        } else {
            let new_amount = curr.amount.checked_sub(prev.amount)?;
            increment_coin_balance(
                deps.storage,
                token_id,
                &Coin {
                    denom: curr.denom.clone(),
                    amount: new_amount,
                },
            )?;
            response = response
                .clone()
                .add_attribute("denom", curr.denom.clone())
                .add_attribute("incremented", new_amount);
        }
    }

    Ok(response.add_attribute("action", "rover/credit_manager/update_coin_balance"))
}
