#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;

use mars_outpost::oracle::PriceResponse;
use mars_rover::adapters::vault::VaultBase;
use mars_rover::adapters::Oracle;
use mars_rover::traits::IntoDecimal;

use crate::error::ContractResult;
use crate::msg::{
    ConfigResponse, ConfigUpdates, ExecuteMsg, InstantiateMsg, PricingMethod, QueryMsg,
    VaultPricingInfo,
};
use crate::state::{ADMIN, ORACLE, VAULT_PRICING_INFO};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(
        deps.storage,
        &format!("crates.io:{}", CONTRACT_NAME),
        CONTRACT_VERSION,
    )?;

    let oracle = msg.oracle.check(deps.api)?;
    ORACLE.save(deps.storage, &oracle)?;

    for info in msg.vault_pricing {
        VAULT_PRICING_INFO.save(deps.storage, &info.vault_coin_denom, &info)?;
    }

    let admin = deps.api.addr_validate(&msg.admin)?;
    ADMIN.set(deps, Some(admin))?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::UpdateConfig { new_config } => update_config(deps, info, new_config),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _: Env, msg: QueryMsg) -> ContractResult<Binary> {
    let res = match msg {
        QueryMsg::Price { denom } => to_binary(&query_price(deps, &denom)?),
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::PricingInfo { denom } => to_binary(&query_pricing_info(deps, &denom)?),
        QueryMsg::AllPricingInfo { start_after, limit } => {
            to_binary(&query_all_pricing_info(deps, start_after, limit)?)
        }
    };
    res.map_err(Into::into)
}

fn query_pricing_info(deps: Deps, denom: &str) -> StdResult<VaultPricingInfo> {
    VAULT_PRICING_INFO.load(deps.storage, denom)
}

fn query_all_pricing_info(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<VaultPricingInfo>> {
    let start = start_after
        .as_ref()
        .map(|denom| Bound::exclusive(denom.as_str()));

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    VAULT_PRICING_INFO
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| {
            let (_, info) = res?;
            Ok(info)
        })
        .collect::<StdResult<Vec<_>>>()
}

fn query_price(deps: Deps, denom: &str) -> ContractResult<PriceResponse> {
    let info_opt = VAULT_PRICING_INFO.may_load(deps.storage, denom)?;
    let oracle = ORACLE.load(deps.storage)?;

    match info_opt {
        Some(info) => match info.method {
            PricingMethod::PreviewRedeem => {
                let vault = VaultBase::new(info.addr.clone());
                calculate_preview_redeem(&deps, &oracle, &info, &vault)
            }
        },
        _ => Ok(oracle.query_price(&deps.querier, denom)?),
    }
}

fn calculate_preview_redeem(
    deps: &Deps,
    oracle: &Oracle,
    info: &VaultPricingInfo,
    vault: &VaultBase<Addr>,
) -> ContractResult<PriceResponse> {
    let total_issued = vault.query_total_vault_coins_issued(&deps.querier)?;
    let amount = vault.query_preview_redeem(&deps.querier, total_issued)?;
    let price_res = oracle.query_price(&deps.querier, &info.base_denom)?;
    let value = price_res.price.checked_mul(amount.to_dec()?)?;

    let price = if value.is_zero() || total_issued.is_zero() {
        Decimal::zero()
    } else {
        value.checked_div(total_issued.to_dec()?)?
    };

    Ok(PriceResponse {
        denom: info.vault_coin_denom.clone(),
        price,
    })
}

fn query_config(deps: Deps) -> ContractResult<ConfigResponse> {
    Ok(ConfigResponse {
        admin: ADMIN.get(deps)?,
        oracle: ORACLE.load(deps.storage)?,
    })
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_config: ConfigUpdates,
) -> ContractResult<Response> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    let mut response =
        Response::new().add_attribute("action", "rover/oracle-adapter/update_config");

    if let Some(unchecked) = new_config.oracle {
        ORACLE.save(deps.storage, &unchecked.check(deps.api)?)?;
        response = response
            .add_attribute("key", "oracle")
            .add_attribute("value", unchecked.address());
    }

    if let Some(vault_pricing) = new_config.vault_pricing {
        VAULT_PRICING_INFO.clear(deps.storage);
        for info in &vault_pricing {
            VAULT_PRICING_INFO.save(deps.storage, &info.vault_coin_denom, info)?;
        }
        let value_str = if vault_pricing.is_empty() {
            "None".to_string()
        } else {
            vault_pricing
                .into_iter()
                .map(|info| info.vault_coin_denom)
                .collect::<Vec<_>>()
                .join(", ")
        };
        response = response
            .add_attribute("key", "vault_pricing")
            .add_attribute("value", value_str);
    }

    if let Some(addr_str) = new_config.admin {
        let validated = deps.api.addr_validate(&addr_str)?;
        ADMIN.set(deps, Some(validated))?;
        response = response
            .add_attribute("key", "owner")
            .add_attribute("value", addr_str);
    }

    Ok(response)
}
