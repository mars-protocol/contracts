use crate::math::decimal::Decimal;
use cosmwasm_std::{Coin, Deps, StdResult, Uint128};
use terra_cosmwasm::TerraQuerier;

pub fn deduct_tax(deps: Deps, coin: Coin) -> StdResult<Coin> {
    let tax_amount = compute_tax(deps, &coin)?;
    Ok(Coin {
        denom: coin.denom,
        amount: coin.amount - tax_amount,
    })
}

pub fn compute_tax(deps: Deps, coin: &Coin) -> StdResult<Uint128> {
    let terra_querier = TerraQuerier::new(&deps.querier);
    let tax_rate: Decimal = (terra_querier.query_tax_rate()?).rate.into();
    let tax_cap = (terra_querier.query_tax_cap(coin.denom.to_string())?).cap;
    let amount = coin.amount;
    Ok(std::cmp::min(
        amount - Decimal::divide_uint128_by_decimal(amount, Decimal::one() + tax_rate)?,
        tax_cap,
    ))
}
