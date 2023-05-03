use cosmwasm_std::{
    BankMsg, Coin, CosmosMsg, DepsMut, MessageInfo, Response, StdError, StdResult, Storage, Uint128,
};
use cw_utils::one_coin;

use crate::{
    error::{ContractError, ContractResult},
    query::{estimate_provide_liquidity, estimate_withdraw_liquidity},
    state::{COIN_BALANCES, COIN_CONFIG, LP_TOKEN_SUPPLY},
};

pub fn provide_liquidity(
    deps: DepsMut,
    info: MessageInfo,
    lp_token_out_denom: String,
    minimum_receive: Uint128,
) -> ContractResult<Response> {
    let underlying = COIN_CONFIG.load(deps.storage, &lp_token_out_denom)?;
    // Ensure no incorrect denoms sent for expected LP token underlying
    for coin in &info.funds {
        if !underlying.contains(&coin.denom) {
            return Err(ContractError::RequirementsNotMet(format!(
                "{} is unexpected for lp_token_out_denom",
                coin.denom
            )));
        }
    }

    let lp_token_amount =
        estimate_provide_liquidity(&deps.as_ref(), &lp_token_out_denom, info.funds.clone())?;

    if minimum_receive > lp_token_amount {
        return Err(ContractError::ReceivedBelowMinimum);
    }

    for coin in info.funds {
        COIN_BALANCES.update(
            deps.storage,
            (&lp_token_out_denom, &coin.denom),
            |amount_opt| -> StdResult<_> {
                Ok(amount_opt.unwrap_or(Uint128::zero()).checked_add(coin.amount)?)
            },
        )?;
    }

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

pub fn withdraw_liquidity(deps: DepsMut, info: MessageInfo) -> ContractResult<Response> {
    let lp_token_sent = one_coin(&info)?;
    let underlying_coins = estimate_withdraw_liquidity(deps.storage, &lp_token_sent)?;

    for coin in &underlying_coins {
        COIN_BALANCES.update(
            deps.storage,
            (&lp_token_sent.denom, &coin.denom),
            |amount_opt| -> StdResult<_> {
                Ok(amount_opt.unwrap_or(Uint128::zero()).checked_sub(coin.amount)?)
            },
        )?;
    }

    let transfer_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: underlying_coins,
    });

    mock_lp_token_burn(deps.storage, &lp_token_sent)?;

    Ok(Response::new().add_message(transfer_msg))
}

fn mock_lp_token_mint(
    storage: &mut dyn Storage,
    amount: Uint128,
    lp_token_out_denom: &str,
) -> Result<(), StdError> {
    let total_supply = LP_TOKEN_SUPPLY.load(storage, lp_token_out_denom).unwrap_or(Uint128::zero());
    LP_TOKEN_SUPPLY.save(storage, lp_token_out_denom, &(total_supply + amount))?;
    Ok(())
}

fn mock_lp_token_burn(storage: &mut dyn Storage, lp_token: &Coin) -> Result<Uint128, StdError> {
    LP_TOKEN_SUPPLY.update(storage, &lp_token.denom, |total_supply| -> StdResult<_> {
        Ok(total_supply.unwrap_or_else(Uint128::zero) - lp_token.amount)
    })
}
