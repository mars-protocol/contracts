#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, IbcMsg, IbcTimeout,
    IbcTimeoutBlock, MessageInfo, Response, StdResult, Uint128, WasmMsg,
};

use mars_outpost::address_provider::{self, MarsContract};
use mars_outpost::error::MarsError;
use mars_outpost::helpers::option_string_to_addr;
use mars_outpost::protocol_rewards_collector::{
    Config, CreateOrUpdateConfig, InstantiateMsg, QueryMsg,
};
use mars_outpost::red_bank;

use osmo_bindings::{OsmosisMsg, OsmosisQuery};

use crate::error::{ContractError, ContractResult};
use crate::helpers::{stringify_option_amount, unwrap_option_amount};
use crate::msg::ExecuteMsg;
use crate::state::{CONFIG, INSTRUCTIONS};
use crate::swap::SwapInstructions;

// INIT

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<OsmosisQuery>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    let cfg = msg.check(deps.api)?;
    cfg.validate()?;

    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::default())
}

// HANDLERS

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<OsmosisQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response<OsmosisMsg>> {
    match msg {
        ExecuteMsg::UpdateConfig(new_cfg) => update_config(deps, info.sender, new_cfg),
        ExecuteMsg::SetSwapInstructions {
            denom_in,
            denom_out,
            instructions,
        } => set_swap_instructions(deps, info.sender, denom_in, denom_out, instructions),
        ExecuteMsg::WithdrawFromRedBank {
            denom,
            amount,
        } => withdraw_from_red_bank(deps, denom, amount),
        ExecuteMsg::DistributeRewards {
            denom,
            amount,
        } => distribute_rewards(deps, env, denom, amount),
        ExecuteMsg::SwapAsset {
            denom,
            amount,
        } => swap_asset(deps, env, denom, amount),
        ExecuteMsg::ExecuteCosmosMsg(cosmos_msg) => {
            execute_cosmos_msg(deps, info.sender, cosmos_msg)
        }
    }
}

pub fn update_config(
    deps: DepsMut<impl cosmwasm_std::CustomQuery>,
    sender: Addr,
    new_cfg: CreateOrUpdateConfig,
) -> ContractResult<Response<OsmosisMsg>> {
    let mut cfg = CONFIG.load(deps.storage)?;

    if sender != cfg.owner {
        return Err(MarsError::Unauthorized {}.into());
    }

    let CreateOrUpdateConfig {
        owner,
        address_provider,
        safety_tax_rate,
        safety_fund_denom,
        fee_collector_denom,
        channel_id,
        revision,
        block_timeout,
    } = new_cfg;

    cfg.owner = option_string_to_addr(deps.api, owner, cfg.owner)?;
    cfg.address_provider = option_string_to_addr(deps.api, address_provider, cfg.address_provider)?;
    cfg.safety_tax_rate = safety_tax_rate.unwrap_or(cfg.safety_tax_rate);
    cfg.safety_fund_denom = safety_fund_denom.unwrap_or(cfg.safety_fund_denom);
    cfg.fee_collector_denom = fee_collector_denom.unwrap_or(cfg.fee_collector_denom);
    cfg.channel_id = channel_id.unwrap_or(cfg.channel_id);
    cfg.revision = revision.unwrap_or(cfg.revision);
    cfg.block_timeout = block_timeout.unwrap_or(cfg.block_timeout);

    cfg.validate()?;

    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::new().add_attribute("action", "mars/rewards-collector/update_config"))
}

pub fn set_swap_instructions(
    deps: DepsMut<OsmosisQuery>,
    sender: Addr,
    denom_in: String,
    denom_out: String,
    instructions: SwapInstructions,
) -> ContractResult<Response<OsmosisMsg>> {
    let cfg = CONFIG.load(deps.storage)?;

    if sender != cfg.owner {
        return Err(MarsError::Unauthorized {}.into());
    }

    instructions.validate(&deps.querier, &denom_in, &denom_out)?;

    INSTRUCTIONS.save(deps.storage, (denom_in.clone(), denom_out.clone()), &instructions)?;

    Ok(Response::new()
        .add_attribute("action", "mars/rewards-collector/update_swap_instructions")
        .add_attribute("denom_in", denom_in)
        .add_attribute("denom_out", denom_out))
}

pub fn withdraw_from_red_bank(
    deps: DepsMut<impl cosmwasm_std::CustomQuery>,
    denom: String,
    amount: Option<Uint128>,
) -> ContractResult<Response<OsmosisMsg>> {
    let cfg = CONFIG.load(deps.storage)?;

    let red_bank_addr = address_provider::helpers::query_address(
        deps.as_ref(),
        &cfg.address_provider,
        MarsContract::RedBank,
    )?;

    // TODO: update red bank execute msg to take denom instead of asset
    let withdraw_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: red_bank_addr.to_string(),
        msg: to_binary(&red_bank::msg::ExecuteMsg::Withdraw {
            asset: mars_outpost::asset::Asset::Native {
                denom: denom.clone(),
            },
            amount,
            recipient: None,
        })?,
        funds: vec![],
    });

    Ok(Response::new()
        .add_message(withdraw_msg)
        .add_attribute("action", "mars/rewards-collector/withdraw_from_red_bank")
        .add_attribute("denom", denom)
        .add_attribute("amount", stringify_option_amount(amount)))
}

pub fn swap_asset(
    deps: DepsMut<impl cosmwasm_std::CustomQuery>,
    env: Env,
    denom: String,
    amount: Option<Uint128>,
) -> ContractResult<Response<OsmosisMsg>> {
    let cfg = CONFIG.load(deps.storage)?;

    // if amount is None, swap the total balance
    let amount_to_swap =
        unwrap_option_amount(&deps.querier, &env.contract.address, &denom, amount)?;

    // split the amount to swap between the safety fund and the fee collector
    let amount_safety_fund = amount_to_swap * cfg.safety_tax_rate;
    let amount_fee_collector = amount_to_swap.checked_sub(amount_safety_fund)?;
    let mut messages = vec![];

    if !amount_safety_fund.is_zero() {
        messages.push(
            INSTRUCTIONS
                .load(deps.storage, (denom.clone(), cfg.safety_fund_denom.clone()))?
                .build_swap_msg(&denom, amount_safety_fund)?,
        );
    }

    if !amount_fee_collector.is_zero() {
        messages.push(
            INSTRUCTIONS
                .load(deps.storage, (denom.clone(), cfg.fee_collector_denom.clone()))?
                .build_swap_msg(&denom, amount_fee_collector)?,
        );
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "mars/rewards-collector/swap_asset")
        .add_attribute("denom", denom)
        .add_attribute("amount_safety_fund", amount_safety_fund)
        .add_attribute("amount_fee_collector", amount_fee_collector))
}

pub fn distribute_rewards(
    deps: DepsMut<impl cosmwasm_std::CustomQuery>,
    env: Env,
    denom: String,
    amount: Option<Uint128>,
) -> ContractResult<Response<OsmosisMsg>> {
    let cfg = CONFIG.load(deps.storage)?;

    let to_address = if denom == cfg.safety_fund_denom {
        address_provider::helpers::query_address(
            deps.as_ref(),
            &cfg.address_provider,
            MarsContract::SafetyFund,
        )?
    } else if denom == cfg.fee_collector_denom {
        address_provider::helpers::query_address(
            deps.as_ref(),
            &cfg.address_provider,
            MarsContract::FeeCollector,
        )?
    } else {
        return Err(ContractError::AssetNotEnabledForDistribution {
            denom: denom.clone(),
        });
    };

    let amount_to_distribute =
        unwrap_option_amount(&deps.querier, &env.contract.address, &denom, amount)?;

    let transfer_msg = CosmosMsg::Ibc(IbcMsg::Transfer {
        channel_id: cfg.channel_id,
        to_address: to_address.to_string(),
        amount: Coin {
            denom: denom.clone(),
            amount: amount_to_distribute,
        },
        timeout: IbcTimeout::with_block(IbcTimeoutBlock {
            revision: cfg.revision,
            height: env.block.height + cfg.block_timeout,
        }),
    });

    Ok(Response::new()
        .add_message(transfer_msg)
        .add_attribute("action", "mars/rewards-collector/distribute_rewards")
        .add_attribute("denom", denom)
        .add_attribute("amount", amount_to_distribute))
}

pub fn execute_cosmos_msg(
    deps: DepsMut<impl cosmwasm_std::CustomQuery>,
    sender: Addr,
    msg: CosmosMsg<OsmosisMsg>,
) -> ContractResult<Response<OsmosisMsg>> {
    let cfg = CONFIG.load(deps.storage)?;

    if sender != cfg.owner {
        return Err(MarsError::Unauthorized {}.into());
    }

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "mars/rewards-collector/execute_cosmos_msg"))
}

// QUERIES

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(
    deps: Deps<impl cosmwasm_std::CustomQuery>,
    _env: Env,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

fn query_config(deps: Deps<impl cosmwasm_std::CustomQuery>) -> StdResult<Config<String>> {
    let cfg = CONFIG.load(deps.storage)?;
    Ok(cfg.into())
}
