use cosmwasm_std::{BankMsg, Coin, CosmosMsg, DepsMut, MessageInfo, Response, StdResult, Uint128};

use crate::{
    contract::STARTING_VAULT_SHARES,
    error::{
        ContractError,
        ContractError::{NoCoinsSent, WrongDenomSent},
    },
    state::{CHAIN_BANK, COIN_BALANCE, ORACLE, TOTAL_VAULT_SHARES, VAULT_TOKEN_DENOM},
};

pub fn deposit(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let total_shares = TOTAL_VAULT_SHARES.load(deps.storage)?;
    let oracle = ORACLE.load(deps.storage)?;
    let balance = COIN_BALANCE.load(deps.storage)?;

    let shares_to_add = if total_shares.is_zero() {
        TOTAL_VAULT_SHARES.save(deps.storage, &STARTING_VAULT_SHARES)?;
        STARTING_VAULT_SHARES
    } else {
        let total_vault_value = oracle.query_total_value(&deps.querier, &[balance])?;
        let assets_value = oracle.query_total_value(&deps.querier, &info.funds)?;
        let shares_to_add = total_shares.checked_multiply_ratio(assets_value, total_vault_value)?;
        TOTAL_VAULT_SHARES.save(deps.storage, &(total_shares + shares_to_add))?;
        shares_to_add
    };

    let balance = COIN_BALANCE.load(deps.storage)?;
    let amount_deposited = match info.funds.first() {
        Some(c) if c.denom == balance.denom => c.amount,
        Some(c) if c.denom != balance.denom => return Err(WrongDenomSent),
        _ => return Err(NoCoinsSent),
    };
    COIN_BALANCE.save(
        deps.storage,
        &Coin {
            denom: balance.denom,
            amount: balance.amount + amount_deposited,
        },
    )?;

    // Send vault tokens to user
    let minted = mock_lp_token_mint(deps, shares_to_add)?;
    let transfer_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![minted],
    });

    Ok(Response::new().add_message(transfer_msg))
}

fn mock_lp_token_mint(deps: DepsMut, amount: Uint128) -> StdResult<Coin> {
    let denom = VAULT_TOKEN_DENOM.load(deps.storage)?;

    CHAIN_BANK.update(deps.storage, |bank_amount| -> StdResult<_> { Ok(bank_amount - amount) })?;

    Ok(Coin {
        denom,
        amount,
    })
}
