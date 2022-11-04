use std::collections::HashSet;
use std::hash::Hash;

use cosmwasm_std::{
    to_binary, Addr, Coin, CosmosMsg, Decimal, Deps, QuerierWrapper, StdResult, Storage, Uint128,
    WasmMsg,
};

use mars_rover::error::{ContractError, ContractResult};
use mars_rover::msg::execute::CallbackMsg;
use mars_rover::msg::query::CoinValue;
use mars_rover::msg::ExecuteMsg;
use mars_rover::traits::IntoDecimal;

use crate::state::{ALLOWED_COINS, COIN_BALANCES, ORACLE, RED_BANK, TOTAL_DEBT_SHARES};
use crate::update_coin_balances::query_balance;

pub fn assert_coin_is_whitelisted(storage: &mut dyn Storage, denom: &str) -> ContractResult<()> {
    let is_whitelisted = ALLOWED_COINS.contains(storage, denom);
    if !is_whitelisted {
        return Err(ContractError::NotWhitelisted(denom.to_string()));
    }
    Ok(())
}

pub fn assert_coins_are_whitelisted(
    storage: &mut dyn Storage,
    denoms: Vec<&str>,
) -> ContractResult<()> {
    denoms
        .iter()
        .try_for_each(|denom| assert_coin_is_whitelisted(storage, denom))
}

pub fn increment_coin_balance(
    storage: &mut dyn Storage,
    account_id: &str,
    coin: &Coin,
) -> ContractResult<Uint128> {
    COIN_BALANCES.update(storage, (account_id, &coin.denom), |value_opt| {
        value_opt
            .unwrap_or_else(Uint128::zero)
            .checked_add(coin.amount)
            .map_err(ContractError::Overflow)
    })
}

pub fn decrement_coin_balance(
    storage: &mut dyn Storage,
    account_id: &str,
    coin: &Coin,
) -> ContractResult<Uint128> {
    let path = COIN_BALANCES.key((account_id, &coin.denom));
    let value_opt = path.may_load(storage)?;
    let new_value = value_opt
        .unwrap_or_else(Uint128::zero)
        .checked_sub(coin.amount)?;
    if new_value.is_zero() {
        path.remove(storage);
    } else {
        path.save(storage, &new_value)?;
    }
    Ok(new_value)
}

pub fn update_balance_msg(
    querier: &QuerierWrapper,
    rover_addr: &Addr,
    account_id: &str,
    denom: &str,
) -> StdResult<CosmosMsg> {
    let previous_balance = query_balance(querier, rover_addr, denom)?;
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: rover_addr.to_string(),
        funds: vec![],
        msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::UpdateCoinBalance {
            account_id: account_id.to_string(),
            previous_balance,
        }))?,
    }))
}

pub fn update_balances_msgs(
    querier: &QuerierWrapper,
    rover_addr: &Addr,
    account_id: &str,
    denoms: Vec<&str>,
) -> StdResult<Vec<CosmosMsg>> {
    denoms
        .iter()
        .map(|denom| update_balance_msg(querier, rover_addr, account_id, denom))
        .collect()
}

pub fn debt_shares_to_amount(
    deps: Deps,
    rover_addr: &Addr,
    denom: &str,
    shares: Uint128,
) -> ContractResult<Coin> {
    // total shares of debt issued for denom
    let total_debt_shares = TOTAL_DEBT_SHARES
        .load(deps.storage, denom)
        .unwrap_or(Uint128::zero());

    // total rover debt amount in Redbank for asset
    let red_bank = RED_BANK.load(deps.storage)?;
    let total_debt_amount = red_bank.query_debt(&deps.querier, rover_addr, denom)?;

    // amount of debt for token's position
    // NOTE: Given the nature of integers, the debt is rounded down. This means that the
    //       remaining share owners will take a small hit of the remainder.
    let amount = total_debt_amount.checked_multiply_ratio(shares, total_debt_shares)?;

    Ok(Coin {
        denom: denom.to_string(),
        amount,
    })
}

pub fn coin_value(deps: &Deps, coin: &Coin) -> ContractResult<CoinValue> {
    let oracle = ORACLE.load(deps.storage)?;
    let res = oracle.query_price(&deps.querier, &coin.denom)?;
    let value = res.price.checked_mul(coin.amount.to_dec()?)?;
    Ok(CoinValue {
        denom: coin.denom.clone(),
        amount: coin.amount,
        price: res.price,
        value,
    })
}

pub trait IntoUint128 {
    fn uint128(&self) -> Uint128;
}

impl IntoUint128 for Decimal {
    fn uint128(&self) -> Uint128 {
        *self * Uint128::new(1)
    }
}

pub fn contents_equal<T>(vec_a: &[T], vec_b: &[T]) -> bool
where
    T: Eq + Hash,
{
    let set_a: HashSet<_> = vec_a.iter().collect();
    let set_b: HashSet<_> = vec_b.iter().collect();
    set_a == set_b
}
