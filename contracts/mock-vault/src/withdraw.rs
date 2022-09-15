use cosmwasm_std::{
    Addr, BankMsg, Coin, CosmosMsg, DepsMut, MessageInfo, Response, StdResult, Storage, Uint128,
};

use crate::error::ContractError;
use crate::query::query_coins_for_shares;
use crate::state::{ASSETS, CHAIN_BANK, LOCKUP_TIME, LP_TOKEN_DENOM, TOTAL_VAULT_SHARES};

pub fn withdraw(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let lockup_time = LOCKUP_TIME.load(deps.storage)?;
    if lockup_time.is_some() {
        return Err(ContractError::UnlockRequired {});
    }
    let vault_tokens = get_vault_token(deps.storage, info.funds.clone())?;
    _exchange(deps.storage, info.sender, vault_tokens.amount)
}

pub fn withdraw_force(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let vault_tokens = get_vault_token(deps.storage, info.funds.clone())?;
    _exchange(deps.storage, info.sender, vault_tokens.amount)
}

/// Swap shares for underlying assets
pub fn _exchange(
    storage: &mut dyn Storage,
    send_to: Addr,
    shares: Uint128,
) -> Result<Response, ContractError> {
    let coins = query_coins_for_shares(storage, shares)?;

    TOTAL_VAULT_SHARES.update(storage, |current_amount| -> StdResult<_> {
        Ok(current_amount - shares)
    })?;

    coins.iter().try_for_each(|asset| -> StdResult<()> {
        ASSETS.update(
            storage,
            asset.clone().denom,
            |current_amount| -> StdResult<_> { Ok(current_amount.unwrap() - asset.amount) },
        )?;
        Ok(())
    })?;

    mock_lp_token_burn(storage, shares)?;

    let transfer_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: send_to.to_string(),
        amount: coins,
    });

    Ok(Response::new().add_message(transfer_msg))
}

pub fn get_vault_token(storage: &mut dyn Storage, funds: Vec<Coin>) -> Result<Coin, ContractError> {
    let vault_token_denom = LP_TOKEN_DENOM.load(storage)?;
    let res = funds.iter().find(|coin| coin.denom == vault_token_denom);
    match res {
        Some(c) if !c.amount.is_zero() => Ok(Coin {
            denom: c.denom.clone(),
            amount: c.amount,
        }),
        _ => Err(ContractError::VaultTokenNotSent {}),
    }
}

fn mock_lp_token_burn(storage: &mut dyn Storage, amount: Uint128) -> StdResult<()> {
    CHAIN_BANK.update(storage, |bank_amount| -> StdResult<_> {
        Ok(bank_amount + amount)
    })?;
    Ok(())
}
