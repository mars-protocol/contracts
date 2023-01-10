use std::{collections::HashSet, hash::Hash};

use cosmwasm_std::{
    to_binary, Addr, Coin, CosmosMsg, Decimal, Deps, DepsMut, Empty, Order::Ascending,
    QuerierWrapper, StdResult, Storage, Uint128, WasmMsg,
};
use cw721::OwnerOfResponse;
use cw721_base::QueryMsg;
use mars_math::{FractionMath, Fractional};
use mars_rover::{
    error::{ContractError, ContractResult},
    msg::{execute::CallbackMsg, ExecuteMsg},
};

use crate::{
    state::{
        ACCOUNT_NFT, ALLOWED_COINS, COIN_BALANCES, ORACLE, RED_BANK, SWAPPER, TOTAL_DEBT_SHARES,
        VAULT_CONFIGS, ZAPPER,
    },
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
        contract_addr,
        &QueryMsg::<Empty>::OwnerOf {
            token_id: account_id.to_string(),
            include_expired: None,
        },
    )?;
    Ok(res.owner)
}

pub fn assert_coin_is_whitelisted(storage: &mut dyn Storage, denom: &str) -> ContractResult<()> {
    let is_whitelisted = ALLOWED_COINS.contains(storage, denom);
    if !is_whitelisted {
        return Err(ContractError::NotWhitelisted(denom.to_string()));
    }
    Ok(())
}

pub fn assert_coins_are_whitelisted(
    storage: &mut dyn Storage,
    denoms: Vec<&str>,
) -> ContractResult<()> {
    denoms.iter().try_for_each(|denom| assert_coin_is_whitelisted(storage, denom))
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
    rover_addr: &Addr,
    account_id: &str,
    denom: &str,
) -> StdResult<CosmosMsg> {
    let previous_balance = query_balance(querier, rover_addr, denom)?;
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: rover_addr.to_string(),
        funds: vec![],
        msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::UpdateCoinBalance {
            account_id: account_id.to_string(),
            previous_balance,
        }))?,
    }))
}

pub fn update_balances_msgs(
    querier: &QuerierWrapper,
    rover_addr: &Addr,
    account_id: &str,
    denoms: Vec<&str>,
) -> StdResult<Vec<CosmosMsg>> {
    denoms.iter().map(|denom| update_balance_msg(querier, rover_addr, account_id, denom)).collect()
}

pub fn debt_shares_to_amount(
    deps: Deps,
    rover_addr: &Addr,
    denom: &str,
    shares: Uint128,
) -> ContractResult<Coin> {
    // total shares of debt issued for denom
    let total_debt_shares = TOTAL_DEBT_SHARES.load(deps.storage, denom).unwrap_or(Uint128::zero());

    // total rover debt amount in Redbank for asset
    let red_bank = RED_BANK.load(deps.storage)?;
    let total_debt_amount = red_bank.query_debt(&deps.querier, rover_addr, denom)?;

    // Amount of debt for token's position. Rounded up to favor participants in the debt pool.
    let amount = total_debt_amount.checked_mul_ceil(Fractional(shares, total_debt_shares))?;

    Ok(Coin {
        denom: denom.to_string(),
        amount,
    })
}

/// Contracts we call from Rover should not be attempting to execute actions.
/// This assertion prevents a kind of reentrancy attack where a contract we call (that turned evil)
/// can deposit into their own credit account and trick our state updates like update_coin_balances.rs
/// which rely on pre-post querying of bank balances of Rover.
/// NOTE: https://twitter.com/larry0x/status/1595919149381079041
pub fn assert_not_contract_in_config(deps: &Deps, addr_to_flag: &Addr) -> ContractResult<()> {
    let vault_addrs =
        VAULT_CONFIGS.keys(deps.storage, None, None, Ascending).collect::<StdResult<Vec<_>>>()?;
    let config_contracts = vec![
        ACCOUNT_NFT.load(deps.storage)?,
        RED_BANK.load(deps.storage)?.address().clone(),
        ORACLE.load(deps.storage)?.address().clone(),
        SWAPPER.load(deps.storage)?.address().clone(),
        ZAPPER.load(deps.storage)?.address().clone(),
    ];

    let flagged_addr_in_config =
        config_contracts.into_iter().chain(vault_addrs).any(|addr| addr == *addr_to_flag);

    if flagged_addr_in_config {
        return Err(ContractError::Unauthorized {
            user: addr_to_flag.to_string(),
            action: "execute actions on rover".to_string(),
        });
    }
    Ok(())
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
