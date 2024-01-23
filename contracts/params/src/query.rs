use cosmwasm_std::{Addr, Deps, Env, Order, StdResult, Uint128};
use cw_storage_plus::Bound;
use mars_interest_rate::get_underlying_liquidity_amount;
use mars_types::{
    address_provider::{self, MarsAddressType},
    params::{AssetParams, ConfigResponse, TotalDepositResponse, VaultConfig},
    red_bank::{self, Market},
};

use crate::state::{ADDRESS_PROVIDER, ASSET_PARAMS, VAULT_CONFIGS};

pub const DEFAULT_LIMIT: u32 = 10;
pub const MAX_LIMIT: u32 = 30;

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    Ok(ConfigResponse {
        address_provider: ADDRESS_PROVIDER.load(deps.storage)?.to_string(),
    })
}

pub fn query_all_asset_params(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<AssetParams>> {
    let start = start_after.as_ref().map(|denom| Bound::exclusive(denom.as_str()));
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    ASSET_PARAMS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| Ok(res?.1))
        .collect()
}

pub fn query_vault_config(deps: Deps, unchecked: &str) -> StdResult<VaultConfig> {
    let addr = deps.api.addr_validate(unchecked)?;
    VAULT_CONFIGS.load(deps.storage, &addr)
}

pub fn query_all_vault_configs(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<VaultConfig>> {
    let vault_addr: Addr;
    let start = match &start_after {
        Some(unchecked) => {
            vault_addr = deps.api.addr_validate(unchecked)?;
            Some(Bound::exclusive(&vault_addr))
        }
        None => None,
    };

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    VAULT_CONFIGS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| Ok(res?.1))
        .collect()
}

/// Query and compute the total deposited amount of the given asset across Red
/// Bank (RB) and Credit Manager (CM).
///
/// Specifically, the amount is defined as:
///   rb_deposit + cm_deposit - cm_debt_owed_to_rb
///
/// Note:
///
/// 1. We subtract the amount of debt that CM owes to RB to avoid double-
///    counting.
///
/// 2. We only consider spot asset holdings, meaning we don't unwrap DEX LP
///    tokens or vault tokens to the underlying assets. After some discussions
///    we have concluded the latter is not feasible.
///
///    For example, when computing the deposited amount of ATOM, we only include
///    ATOM deposited in RB and CM; we don't include the ATOM-OSMO LP token, or
///    the ATOM-OSMO farming vault.
pub fn query_total_deposit(
    deps: Deps,
    env: &Env,
    denom: String,
) -> StdResult<TotalDepositResponse> {
    let current_timestamp = env.block.time.seconds();

    // query contract addresses
    let address_provider_addr = ADDRESS_PROVIDER.load(deps.storage)?;
    let addresses = address_provider::helpers::query_contract_addrs(
        deps,
        &address_provider_addr,
        vec![MarsAddressType::RedBank, MarsAddressType::CreditManager],
    )?;
    let credit_manager_addr = &addresses[&MarsAddressType::CreditManager];
    let red_bank_addr = &addresses[&MarsAddressType::RedBank];

    // amount of this asset deposited into Red Bank
    // if the market doesn't exist on RB, we default to zero
    let rb_deposit = deps
        .querier
        .query_wasm_smart::<Option<Market>>(
            red_bank_addr,
            &red_bank::QueryMsg::Market {
                denom: denom.clone(),
            },
        )?
        .map(|market| {
            get_underlying_liquidity_amount(
                market.collateral_total_scaled,
                &market,
                current_timestamp,
            )
        })
        .transpose()?
        .unwrap_or_else(Uint128::zero);

    // amount of this asset deposited into Credit Manager
    // this is simply the coin balance of the CM contract
    // note that this way, we don't include LP tokens or vault positions
    let cm_deposit = deps.querier.query_balance(credit_manager_addr, &denom)?.amount;

    // total deposited amount
    let amount = rb_deposit.checked_add(cm_deposit)?;

    // additionally, we include the deposit cap in the response
    let asset_params = ASSET_PARAMS.load(deps.storage, &denom)?;

    Ok(TotalDepositResponse {
        denom,
        amount,
        cap: asset_params.deposit_cap,
    })
}
