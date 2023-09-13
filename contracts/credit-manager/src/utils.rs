use std::{collections::HashSet, hash::Hash};

use cosmwasm_std::{
    to_binary, Addr, Coin, CosmosMsg, Decimal, Deps, DepsMut, Empty, QuerierWrapper, StdResult,
    Storage, Uint128, WasmMsg,
};
use cw721::OwnerOfResponse;
use cw721_base::QueryMsg;
use mars_rover::{
    error::{ContractError, ContractResult},
    msg::{
        execute::{CallbackMsg, ChangeExpected},
        ExecuteMsg,
    },
};
use mars_rover_health_types::AccountKind;

use crate::{
    state::{ACCOUNT_KINDS, ACCOUNT_NFT, COIN_BALANCES, PARAMS, RED_BANK, TOTAL_DEBT_SHARES},
    update_coin_balances::query_balance,
};

pub fn assert_is_token_owner(deps: &DepsMut, user: &Addr, account_id: &str) -> ContractResult<()> {
    let owner = query_nft_token_owner(deps.as_ref(), account_id)?;
    if user != &owner {
        return Err(ContractError::NotTokenOwner {
            user: user.to_string(),
            account_id: account_id.to_string(),
        });
    }
    Ok(())
}

pub fn query_nft_token_owner(deps: Deps, account_id: &str) -> ContractResult<String> {
    let contract_addr = ACCOUNT_NFT.load(deps.storage)?;
    let res: OwnerOfResponse = deps.querier.query_wasm_smart(
        contract_addr.address(),
        &QueryMsg::<Empty>::OwnerOf {
            token_id: account_id.to_string(),
            include_expired: None,
        },
    )?;
    Ok(res.owner)
}

pub fn assert_coin_is_whitelisted(deps: &mut DepsMut, denom: &str) -> ContractResult<()> {
    let params = PARAMS.load(deps.storage)?;
    match params.query_asset_params(&deps.querier, denom) {
        Ok(p) if p.credit_manager.whitelisted => Ok(()),
        _ => Err(ContractError::NotWhitelisted(denom.to_string())),
    }
}

pub fn assert_coins_are_whitelisted(deps: &mut DepsMut, denoms: Vec<&str>) -> ContractResult<()> {
    denoms.iter().try_for_each(|denom| assert_coin_is_whitelisted(deps, denom))
}

pub fn get_account_kind(storage: &dyn Storage, account_id: &str) -> ContractResult<AccountKind> {
    Ok(ACCOUNT_KINDS.may_load(storage, account_id)?.unwrap_or(AccountKind::Default))
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
    let new_value = value_opt.unwrap_or_else(Uint128::zero).checked_sub(coin.amount)?;
    if new_value.is_zero() {
        path.remove(storage);
    } else {
        path.save(storage, &new_value)?;
    }
    Ok(new_value)
}

pub fn update_balance_msg(
    querier: &QuerierWrapper,
    credit_manager_addr: &Addr,
    account_id: &str,
    denom: &str,
    change: ChangeExpected,
) -> StdResult<CosmosMsg> {
    let previous_balance = query_balance(querier, credit_manager_addr, denom)?;
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: credit_manager_addr.to_string(),
        funds: vec![],
        msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::UpdateCoinBalance {
            account_id: account_id.to_string(),
            previous_balance,
            change,
        }))?,
    }))
}

pub fn update_balances_msgs(
    querier: &QuerierWrapper,
    credit_manager_addr: &Addr,
    account_id: &str,
    denoms: Vec<&str>,
    change: ChangeExpected,
) -> StdResult<Vec<CosmosMsg>> {
    denoms
        .iter()
        .map(|denom| {
            update_balance_msg(querier, credit_manager_addr, account_id, denom, change.clone())
        })
        .collect()
}

pub fn update_balance_after_vault_liquidation_msg(
    querier: &QuerierWrapper,
    credit_manager_addr: &Addr,
    account_id: &str,
    denom: &str,
    protocol_fee: Decimal,
) -> StdResult<CosmosMsg> {
    let previous_balance = query_balance(querier, credit_manager_addr, denom)?;
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: credit_manager_addr.to_string(),
        funds: vec![],
        msg: to_binary(&ExecuteMsg::Callback(
            CallbackMsg::UpdateCoinBalanceAfterVaultLiquidation {
                account_id: account_id.to_string(),
                previous_balance,
                protocol_fee,
            },
        ))?,
    }))
}

pub fn debt_shares_to_amount(deps: Deps, denom: &str, shares: Uint128) -> ContractResult<Coin> {
    // total shares of debt issued for denom
    let total_debt_shares = TOTAL_DEBT_SHARES.load(deps.storage, denom).unwrap_or(Uint128::zero());

    // total rover debt amount in Redbank for asset
    let red_bank = RED_BANK.load(deps.storage)?;
    let total_debt_amount = red_bank.query_debt(&deps.querier, denom)?;

    // Amount of debt for token's position. Rounded up to favor participants in the debt pool.
    let amount = total_debt_amount.checked_mul_ceil((shares, total_debt_shares))?;

    Ok(Coin {
        denom: denom.to_string(),
        amount,
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
