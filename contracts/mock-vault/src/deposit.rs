use cosmwasm_std::{BankMsg, Coin, CosmosMsg, DepsMut, MessageInfo, Response, StdResult, Uint128};

use crate::contract::STARTING_VAULT_SHARES;
use crate::error::ContractError;
use crate::query::get_all_vault_coins;
use crate::state::{ASSETS, CHAIN_BANK, LP_TOKEN_DENOM, ORACLE, TOTAL_VAULT_SHARES};

pub fn deposit(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let total_shares_opt = TOTAL_VAULT_SHARES.may_load(deps.storage)?;
    let oracle = ORACLE.load(deps.storage)?;
    let all_vault_assets = get_all_vault_coins(deps.storage)?;

    let shares_to_add = match total_shares_opt {
        None => {
            TOTAL_VAULT_SHARES.save(deps.storage, &STARTING_VAULT_SHARES)?;
            STARTING_VAULT_SHARES
        }
        Some(total_shares) => {
            let total_vault_value = oracle.query_total_value(&deps.querier, &all_vault_assets)?;
            let assets_value = oracle.query_total_value(&deps.querier, &info.funds)?;
            let shares_to_add = total_shares
                .checked_multiply_ratio(assets_value.atomics(), total_vault_value.atomics())?;
            TOTAL_VAULT_SHARES.save(deps.storage, &(total_shares + shares_to_add))?;
            shares_to_add
        }
    };

    info.funds.iter().try_for_each(|asset| -> StdResult<()> {
        ASSETS.update(
            deps.storage,
            asset.clone().denom,
            |current_amount| -> StdResult<_> {
                Ok(current_amount.unwrap_or(Uint128::zero()) + asset.amount)
            },
        )?;
        Ok(())
    })?;

    // Send vault tokens to
    let minted = mock_lp_token_mint(deps, shares_to_add)?;
    let transfer_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![minted],
    });

    Ok(Response::new().add_message(transfer_msg))
}

fn mock_lp_token_mint(deps: DepsMut, amount: Uint128) -> StdResult<Coin> {
    let denom = LP_TOKEN_DENOM.load(deps.storage)?;

    CHAIN_BANK.update(deps.storage, |bank_amount| -> StdResult<_> {
        Ok(bank_amount - amount)
    })?;

    Ok(Coin { denom, amount })
}
