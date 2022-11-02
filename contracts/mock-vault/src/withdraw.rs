use cosmwasm_std::{
    Addr, BankMsg, Coin, CosmosMsg, DepsMut, MessageInfo, Response, StdError, StdResult, Storage,
    Uint128,
};

use crate::error::{ContractError, ContractResult};
use crate::query::shares_to_base_denom_amount;
use crate::state::{CHAIN_BANK, COIN_BALANCE, LOCKUP_TIME, TOTAL_VAULT_SHARES, VAULT_TOKEN_DENOM};

pub fn withdraw(deps: DepsMut, info: MessageInfo) -> ContractResult<Response> {
    let lockup_time = LOCKUP_TIME.load(deps.storage)?;
    if lockup_time.is_some() {
        return Err(ContractError::UnlockRequired {});
    }
    let vault_tokens = get_vault_token(deps.storage, info.funds.clone())?;
    _exchange(deps.storage, &info.sender, vault_tokens.amount)
}

pub fn redeem_force(deps: DepsMut, info: MessageInfo) -> ContractResult<Response> {
    let vault_tokens = get_vault_token(deps.storage, info.funds.clone())?;
    _exchange(deps.storage, &info.sender, vault_tokens.amount)
}

/// Swap shares for underlying assets
pub fn _exchange(
    storage: &mut dyn Storage,
    send_to: &Addr,
    shares: Uint128,
) -> ContractResult<Response> {
    let amount = withdraw_state_update(storage, shares)?;
    let base_token = COIN_BALANCE.load(storage)?;
    let transfer_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: send_to.to_string(),
        amount: vec![Coin {
            denom: base_token.denom,
            amount,
        }],
    });
    Ok(Response::new().add_message(transfer_msg))
}

pub fn withdraw_state_update(
    storage: &mut dyn Storage,
    shares: Uint128,
) -> ContractResult<Uint128> {
    let base_amount = shares_to_base_denom_amount(storage, shares)?;
    COIN_BALANCE.update(storage, |total| -> StdResult<_> {
        Ok(Coin {
            denom: total.denom,
            amount: total.amount - base_amount,
        })
    })?;

    let current_amount = TOTAL_VAULT_SHARES.load(storage)?;
    TOTAL_VAULT_SHARES.save(storage, &(current_amount - shares))?;

    mock_lp_token_burn(storage, shares)?;
    Ok(base_amount)
}

pub fn get_vault_token(storage: &mut dyn Storage, funds: Vec<Coin>) -> ContractResult<Coin> {
    let vault_token_denom = VAULT_TOKEN_DENOM.load(storage)?;
    let res = funds.iter().find(|coin| coin.denom == vault_token_denom);
    match res {
        Some(c) if !c.amount.is_zero() => Ok(Coin {
            denom: c.denom.clone(),
            amount: c.amount,
        }),
        _ => Err(ContractError::VaultTokenNotSent {}),
    }
}

fn mock_lp_token_burn(storage: &mut dyn Storage, amount: Uint128) -> Result<Uint128, StdError> {
    CHAIN_BANK.update(storage, |bank_amount| -> StdResult<_> {
        Ok(bank_amount + amount)
    })
}
