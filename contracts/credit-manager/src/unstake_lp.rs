use cosmwasm_std::{Coin, DepsMut, OverflowError, Response};
use mars_types::credit_manager::{ActionAmount, ActionCoin};

use crate::{
    error::{ContractError, ContractResult},
    state::INCENTIVES,
    utils::increment_coin_balance,
};

pub fn unstake_lp(
    deps: DepsMut,
    account_id: &str,
    lp_coin: ActionCoin,
) -> ContractResult<Response> {
    let incentives = INCENTIVES.load(deps.storage)?;

    // Query rewards user is receiving, update balance
    let lp_position = incentives.query_astroport_staked_lp_position(
        &deps.querier,
        account_id,
        lp_coin.denom.as_str(),
    )?;

    for reward in lp_position.rewards.iter() {
        increment_coin_balance(deps.storage, account_id, reward)?;
    }

    let new_amount = match lp_coin.amount {
        ActionAmount::Exact(amt) => {
            if lp_position.lp_coin.amount.lt(&amt) {
                return Err(ContractError::Overflow(OverflowError {
                    operation: cosmwasm_std::OverflowOperation::Sub,
                    operand1: lp_position.lp_coin.amount.to_string(),
                    operand2: amt.to_string(),
                }));
            } else {
                lp_position.lp_coin.amount.checked_sub(amt)?
            }
        }
        ActionAmount::AccountBalance => lp_position.lp_coin.amount,
    };

    let updated_coin = Coin {
        denom: lp_coin.denom.clone(),
        amount: new_amount,
    };

    increment_coin_balance(deps.storage, account_id, &updated_coin)?;

    // unstake msg
    let unstake_msg = incentives.unstake_astro_lp_msg(account_id, updated_coin)?;

    Ok(Response::new()
        .add_message(unstake_msg)
        .add_attribute("action", "unstake_lp")
        .add_attribute("account_id", account_id)
        .add_attribute("lp_unstaked", format!("{}{}", new_amount, &lp_coin.denom)))
}
