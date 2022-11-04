use crate::error::ContractError;
use crate::query::{estimate_provide_liquidity, estimate_withdraw_liquidity};
use crate::state::{COIN_BALANCES, LP_TOKEN_SUPPLY};
use cosmwasm_std::{
    BankMsg, Coin, CosmosMsg, DepsMut, MessageInfo, Response, StdError, StdResult, Storage, Uint128,
};

pub fn provide_liquidity(
    deps: DepsMut,
    info: MessageInfo,
    lp_token_out_denom: String,
    minimum_receive: Uint128,
) -> Result<Response, ContractError> {
    let sent_coin_a = info.funds.get(0).ok_or(ContractError::CoinNotFound)?;
    let sent_coin_b = info.funds.get(1).ok_or(ContractError::CoinNotFound)?;
    let (mut coin0, mut coin1) = COIN_BALANCES.load(deps.storage, &lp_token_out_denom)?;

    if (sent_coin_a.denom != coin0.denom && sent_coin_a.denom != coin1.denom)
        || (sent_coin_b.denom != coin0.denom && sent_coin_b.denom != coin1.denom)
    {
        return Err(ContractError::RequirementsNotMet {
            lp_token: lp_token_out_denom,
            coin0: coin0.denom,
            coin1: coin1.denom,
        });
    }

    let lp_token_amount =
        estimate_provide_liquidity(&deps.as_ref(), &lp_token_out_denom, info.funds.clone())?;

    if minimum_receive > lp_token_amount {
        return Err(ContractError::ReceivedBelowMinimum);
    }

    // Update internal balances
    if coin0.denom == sent_coin_a.denom {
        coin0.amount += sent_coin_a.amount;
        coin1.amount += sent_coin_b.amount;
    } else {
        coin0.amount += sent_coin_b.amount;
        coin1.amount += sent_coin_a.amount;
    }
    COIN_BALANCES.save(deps.storage, &lp_token_out_denom, &(coin0, coin1))?;

    // Send LP tokens to user (assumes mock zapper has been pre-funded with this token)
    mock_lp_token_mint(deps.storage, lp_token_amount, &lp_token_out_denom)?;
    let transfer_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![Coin {
            denom: lp_token_out_denom,
            amount: lp_token_amount,
        }],
    });

    Ok(Response::new().add_message(transfer_msg))
}

pub fn withdraw_liquidity(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let lp_token_sent = info.funds.get(0).ok_or(ContractError::CoinNotFound)?;
    mock_lp_token_burn(deps.storage, lp_token_sent)?;

    let underlying_coins = estimate_withdraw_liquidity(deps.storage, lp_token_sent)?;

    // Update internal balances
    let (mut coin0, mut coin1) = COIN_BALANCES.load(deps.storage, &lp_token_sent.denom)?;
    let coin_a = underlying_coins.get(0).ok_or(ContractError::CoinNotFound)?;
    let coin_b = underlying_coins.get(1).ok_or(ContractError::CoinNotFound)?;
    if coin0.denom == coin_a.denom {
        coin0.amount -= coin_a.amount;
        coin1.amount -= coin_b.amount;
    } else {
        coin0.amount -= coin_b.amount;
        coin1.amount -= coin_a.amount;
    };
    COIN_BALANCES.save(deps.storage, &lp_token_sent.denom, &(coin0, coin1))?;

    let transfer_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: underlying_coins,
    });
    Ok(Response::new().add_message(transfer_msg))
}

fn mock_lp_token_mint(
    storage: &mut dyn Storage,
    amount: Uint128,
    lp_token_out_denom: &str,
) -> Result<(), StdError> {
    let total_supply = LP_TOKEN_SUPPLY
        .load(storage, lp_token_out_denom)
        .unwrap_or(Uint128::zero());
    LP_TOKEN_SUPPLY.save(storage, lp_token_out_denom, &(total_supply + amount))?;
    Ok(())
}

fn mock_lp_token_burn(storage: &mut dyn Storage, lp_token: &Coin) -> Result<Uint128, StdError> {
    LP_TOKEN_SUPPLY.update(storage, &lp_token.denom, |total_supply| -> StdResult<_> {
        Ok(total_supply.unwrap_or_else(Uint128::zero) - lp_token.amount)
    })
}
