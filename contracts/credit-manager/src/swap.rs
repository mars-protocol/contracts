use cosmwasm_std::{Coin, Decimal, DepsMut, Env, Response, Uint128};

use mars_rover::error::{ContractError, ContractResult};

use crate::state::{COIN_BALANCES, SWAPPER};
use crate::utils::{assert_coin_is_whitelisted, decrement_coin_balance, update_balance_msg};

pub fn swap_exact_in(
    deps: DepsMut,
    env: Env,
    account_id: &str,
    coin_in_denom: &str,
    coin_in_amount: Option<Uint128>,
    denom_out: &str,
    slippage: Decimal,
) -> ContractResult<Response> {
    assert_coin_is_whitelisted(deps.storage, denom_out)?;

    // Passing None for coin_in_amount defaults to account's balance for `coin_in_denom`
    let coin_in_to_trade = Coin {
        denom: coin_in_denom.to_string(),
        amount: coin_in_amount.unwrap_or(
            COIN_BALANCES
                .may_load(deps.storage, (account_id, coin_in_denom))?
                .unwrap_or(Uint128::zero()),
        ),
    };

    if coin_in_to_trade.amount.is_zero() {
        return Err(ContractError::NoAmount);
    }

    decrement_coin_balance(deps.storage, account_id, &coin_in_to_trade)?;

    // Updates coin balances for account after the swap has taken place
    let update_coin_balance_msg =
        update_balance_msg(&deps.querier, &env.contract.address, account_id, denom_out)?;

    let swapper = SWAPPER.load(deps.storage)?;

    Ok(Response::new()
        .add_message(swapper.swap_exact_in_msg(&coin_in_to_trade, denom_out, slippage)?)
        .add_message(update_coin_balance_msg)
        .add_attribute("action", "rover/credit-manager/swapper"))
}
