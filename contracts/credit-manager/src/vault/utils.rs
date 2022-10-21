use cosmwasm_std::{
    coin, to_binary, Addr, Coin, Deps, QueryRequest, StdResult, Storage, WasmQuery,
};

use mars_oracle_adapter::msg::QueryMsg::PriceableUnderlying;
use rover::adapters::vault::{
    Total, Vault, VaultPosition, VaultPositionAmount, VaultPositionUpdate,
};
use rover::error::{ContractError, ContractResult};

use crate::state::{ALLOWED_VAULTS, ORACLE, VAULT_POSITIONS};
use crate::update_coin_balances::query_balances;

pub fn assert_vault_is_whitelisted(storage: &mut dyn Storage, vault: &Vault) -> ContractResult<()> {
    let is_whitelisted = ALLOWED_VAULTS.has(storage, &vault.address);
    if !is_whitelisted {
        return Err(ContractError::NotWhitelisted(vault.address.to_string()));
    }
    Ok(())
}

pub fn update_vault_position(
    storage: &mut dyn Storage,
    account_id: &str,
    vault_addr: &Addr,
    update: VaultPositionUpdate,
) -> ContractResult<VaultPositionAmount> {
    let path = VAULT_POSITIONS.key((account_id, vault_addr.clone()));
    let mut amount = path
        .may_load(storage)?
        .unwrap_or_else(|| update.default_amount());

    amount.update(update)?;

    if amount.total().is_zero() {
        path.remove(storage);
    } else {
        path.save(storage, &amount)?;
    }
    Ok(amount)
}

/// Returns the denoms received on a withdraw, inferred by vault entry requirements
pub fn query_withdraw_denom_balances(
    deps: Deps,
    rover_addr: &Addr,
    vault: &Vault,
) -> StdResult<Vec<Coin>> {
    let vault_info = vault.query_info(&deps.querier)?;
    let denoms = vault_info
        .accepts
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    query_balances(deps, rover_addr, &denoms)
}

/// Returns the mars-oracle accepted priceable coins
pub fn get_priceable_coins(deps: &Deps, positions: &[VaultPosition]) -> ContractResult<Vec<Coin>> {
    let oracle = ORACLE.load(deps.storage)?;
    let mut coins: Vec<Coin> = vec![];
    for p in positions {
        let vault_info = p.vault.query_info(&deps.querier)?;
        let total_vault_coins = p.amount.total();
        let priceable_coins: Vec<Coin> =
            deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: oracle.address().to_string(),
                msg: to_binary(&PriceableUnderlying {
                    coin: coin(total_vault_coins.u128(), vault_info.token_denom),
                })?,
            }))?;
        coins.extend(priceable_coins)
    }
    Ok(coins)
}
