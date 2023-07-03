use cosmwasm_std::{
    Addr, BalanceResponse, BankQuery, Coin, Decimal, DepsMut, Env, QuerierWrapper, QueryRequest,
    Response, StdResult,
};
use mars_rover::{
    error::{ContractError::BalanceChange, ContractResult},
    msg::execute::ChangeExpected,
};

use crate::{
    state::REWARDS_COLLECTOR,
    utils::{decrement_coin_balance, increment_coin_balance},
};

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
    prev: Coin,
    change: ChangeExpected,
) -> ContractResult<Response> {
    let curr = query_balance(&deps.querier, &env.contract.address, &prev.denom)?;
    match change {
        ChangeExpected::Increase if prev.amount < curr.amount => {
            let coin_to_increment = Coin {
                denom: curr.denom,
                amount: curr.amount.checked_sub(prev.amount)?,
            };
            increment_coin_balance(deps.storage, account_id, &coin_to_increment)?;
            change_response(account_id, change, coin_to_increment)
        }
        ChangeExpected::Decrease if prev.amount > curr.amount => {
            let coin_to_reduce = Coin {
                denom: curr.denom,
                amount: prev.amount.checked_sub(curr.amount)?,
            };
            decrement_coin_balance(deps.storage, account_id, &coin_to_reduce)?;
            change_response(account_id, change, coin_to_reduce)
        }
        _ => Err(BalanceChange {
            denom: prev.denom,
            prev_amount: prev.amount,
            curr_amount: curr.amount,
        }),
    }
}

fn change_response(
    account_id: &str,
    change: ChangeExpected,
    coin: Coin,
) -> ContractResult<Response> {
    Ok(Response::new()
        .add_attribute("action", "update_coin_balance")
        .add_attribute("account_id", account_id)
        .add_attribute("coin", coin.to_string())
        .add_attribute(
            "change",
            match change {
                ChangeExpected::Increase => "increase",
                ChangeExpected::Decrease => "decrease",
            },
        ))
}

pub fn update_coin_balance_after_vault_liquidation(
    deps: DepsMut,
    env: Env,
    account_id: &str,
    prev: &Coin,
    protocol_fee: Decimal,
) -> ContractResult<Response> {
    let curr = query_balance(&deps.querier, &env.contract.address, &prev.denom)?;
    let mut amount_to_increment = curr.amount.checked_sub(prev.amount)?;

    if !protocol_fee.is_zero() {
        let protocol_fee_amt = amount_to_increment.checked_mul_ceil(protocol_fee)?;
        amount_to_increment = amount_to_increment.checked_sub(protocol_fee_amt)?;

        let rewards_collector_account = REWARDS_COLLECTOR.load(deps.storage)?.account_id;
        increment_coin_balance(
            deps.storage,
            &rewards_collector_account,
            &Coin {
                denom: curr.denom.clone(),
                amount: protocol_fee_amt,
            },
        )?;
    };

    let coin_to_increment = Coin {
        denom: curr.denom,
        amount: amount_to_increment,
    };
    increment_coin_balance(deps.storage, account_id, &coin_to_increment)?;

    Ok(Response::new()
        .add_attribute("action", "update_coin_balance_after_vault_liquidation")
        .add_attribute("account_id", account_id)
        .add_attribute("coin_incremented", coin_to_increment.to_string()))
}
