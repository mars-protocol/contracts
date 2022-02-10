#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order,
    Response, StdError, StdResult, Storage, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw_storage_plus::{Bound, U64Key};

use astroport::asset::AssetInfo;

use mars_core::error::MarsError;
use mars_core::helpers::{
    cw20_get_balance, cw20_get_total_supply, option_string_to_addr, zero_address,
};
use mars_core::math::decimal::Decimal;
use mars_core::swapping::execute_swap;

use mars_core::address_provider::{self, MarsContract};

use crate::error::ContractError;
use crate::msg::{CreateOrUpdateConfig, ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg};
use crate::state::{CLAIMS, CONFIG, GLOBAL_STATE, SLASH_EVENTS};
use crate::{Claim, ClaimResponse, Config, GlobalState, SlashEvent};

// INSTANTIATE

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // Destructuring a struct’s fields into separate variables in order to force
    // compile error if we add more params
    let CreateOrUpdateConfig {
        owner,
        cooldown_duration,
        address_provider_address,
        astroport_factory_address,
        astroport_max_spread,
    } = msg.config;

    // All fields should be available
    let available = owner.is_some()
        && cooldown_duration.is_some()
        && address_provider_address.is_some()
        && astroport_factory_address.is_some()
        && astroport_max_spread.is_some();

    if !available {
        return Err(MarsError::InstantiateParamsUnavailable {}.into());
    };

    // Initialize config
    let config = Config {
        owner: option_string_to_addr(deps.api, owner, zero_address())?,
        cooldown_duration: cooldown_duration.unwrap(),
        address_provider_address: option_string_to_addr(
            deps.api,
            address_provider_address,
            zero_address(),
        )?,
        astroport_factory_address: option_string_to_addr(
            deps.api,
            astroport_factory_address,
            zero_address(),
        )?,
        astroport_max_spread: astroport_max_spread.unwrap(),
    };

    CONFIG.save(deps.storage, &config)?;

    // Initialize global state
    GLOBAL_STATE.save(
        deps.storage,
        &GlobalState {
            total_mars_for_claimers: Uint128::zero(),
        },
    )?;

    Ok(Response::default())
}

// EXECUTE

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(cw20_msg) => Ok(execute_receive_cw20(deps, env, info, cw20_msg)?),

        ExecuteMsg::UpdateConfig { config } => Ok(execute_update_config(deps, info, config)?),

        ExecuteMsg::Claim { recipient } => Ok(execute_claim(deps, env, info, recipient)?),

        ExecuteMsg::TransferMars { recipient, amount } => {
            Ok(execute_transfer_mars(deps, env, info, recipient, amount)?)
        }

        ExecuteMsg::SwapAssetToUusd {
            offer_asset_info,
            amount,
        } => Ok(execute_swap_asset_to_uusd(
            deps,
            env,
            offer_asset_info,
            amount,
        )?),

        ExecuteMsg::SwapUusdToMars { amount } => Ok(execute_swap_uusd_to_mars(deps, env, amount)?),
    }
}

pub fn execute_receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    match from_binary(&cw20_msg.msg)? {
        ReceiveMsg::Stake { recipient } => {
            execute_stake(deps, env, info, cw20_msg.sender, recipient, cw20_msg.amount)
        }

        ReceiveMsg::Unstake { recipient } => {
            execute_unstake(deps, env, info, cw20_msg.sender, recipient, cw20_msg.amount)
        }
    }
}

pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_config: CreateOrUpdateConfig,
) -> Result<Response, MarsError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(MarsError::Unauthorized {});
    }

    // Destructuring a struct’s fields into separate variables in order to force
    // compile error if we add more params
    let CreateOrUpdateConfig {
        owner,
        cooldown_duration,
        address_provider_address,
        astroport_factory_address,
        astroport_max_spread,
    } = new_config;

    // Update config
    config.owner = option_string_to_addr(deps.api, owner, config.owner)?;
    config.address_provider_address = option_string_to_addr(
        deps.api,
        address_provider_address,
        config.address_provider_address,
    )?;
    config.astroport_factory_address = option_string_to_addr(
        deps.api,
        astroport_factory_address,
        config.astroport_factory_address,
    )?;
    config.astroport_max_spread = astroport_max_spread.unwrap_or(config.astroport_max_spread);
    config.cooldown_duration = cooldown_duration.unwrap_or(config.cooldown_duration);

    CONFIG.save(deps.storage, &config)?;

    let res = Response::new().add_attribute("action", "update_config");
    Ok(res)
}

pub fn execute_stake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    staker: String,
    option_recipient: Option<String>,
    stake_amount: Uint128,
) -> Result<Response, ContractError> {
    // check stake is valid
    let config = CONFIG.load(deps.storage)?;
    let global_state = GLOBAL_STATE.load(deps.storage)?;

    if stake_amount.is_zero() {
        return Err(ContractError::StakeAmountZero {});
    }

    let staking_tokens_info =
        get_staking_tokens_info(deps.as_ref(), &env, &config, &global_state, stake_amount)?;

    // Has to send Mars tokens
    if info.sender != staking_tokens_info.mars_token_address {
        return Err(MarsError::Unauthorized {}.into());
    }

    let xmars_per_mars_option = compute_xmars_per_mars(&staking_tokens_info)?;

    let mint_amount = if let Some(xmars_per_mars) = xmars_per_mars_option {
        stake_amount * xmars_per_mars
    } else {
        stake_amount
    };

    let recipient = option_recipient.unwrap_or_else(|| staker.clone());

    let res = Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: staking_tokens_info.xmars_token_address.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: recipient.clone(),
                amount: mint_amount,
            })?,
        }))
        .add_attribute("action", "stake")
        .add_attribute("staker", staker)
        .add_attribute("mars_staked", stake_amount)
        .add_attribute("xmars_minted", mint_amount)
        .add_attribute("recipient", recipient);

    Ok(res)
}

pub fn execute_unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    staker: String,
    option_recipient: Option<String>,
    burn_amount: Uint128,
) -> Result<Response, ContractError> {
    // check if unstake is valid
    let config = CONFIG.load(deps.storage)?;
    let mut global_state = GLOBAL_STATE.load(deps.storage)?;

    let staking_tokens_info =
        get_staking_tokens_info(deps.as_ref(), &env, &config, &global_state, Uint128::zero())?;

    if info.sender != staking_tokens_info.xmars_token_address {
        return Err(MarsError::Unauthorized {}.into());
    }
    if burn_amount.is_zero() {
        return Err(ContractError::UnstakeAmountZero {});
    }

    let mars_per_xmars_option = compute_mars_per_xmars(&staking_tokens_info)?;

    let claimable_amount = if let Some(mars_per_xmars) = mars_per_xmars_option {
        burn_amount * mars_per_xmars
    } else {
        return Err(StdError::generic_err("mars/xmars ratio is undefined").into());
    };

    let claim = Claim {
        created_at_block: env.block.height,
        cooldown_end_timestamp: env.block.time.seconds() + config.cooldown_duration,
        amount: claimable_amount,
    };

    let recipient = option_recipient.unwrap_or_else(|| staker.clone());
    let recipient_addr = deps.api.addr_validate(&recipient)?;

    if CLAIMS.may_load(deps.storage, &recipient_addr)?.is_some() {
        return Err(ContractError::UnstakeActiveClaim {});
    }
    CLAIMS.save(deps.storage, &recipient_addr, &claim)?;

    global_state.total_mars_for_claimers = global_state
        .total_mars_for_claimers
        .checked_add(claimable_amount)?;

    if global_state.total_mars_for_claimers > staking_tokens_info.total_mars_in_staking_contract {
        return Err(ContractError::MarsForClaimersOverflow {});
    }

    GLOBAL_STATE.save(deps.storage, &global_state)?;

    let res = Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: staking_tokens_info.xmars_token_address.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Burn {
                amount: burn_amount,
            })?,
        }))
        .add_attribute("action", "unstake")
        .add_attribute("staker", staker)
        .add_attribute("recipient", recipient)
        .add_attribute("xmars_burned", burn_amount)
        .add_attribute("mars_claimable", claimable_amount);
    Ok(res)
}

pub fn execute_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    option_recipient: Option<String>,
) -> Result<Response, ContractError> {
    let mut claim = CLAIMS.load(deps.storage, &info.sender)?;

    if claim.cooldown_end_timestamp > env.block.time.seconds() {
        return Err(ContractError::ClaimCooldownNotEnded {});
    }

    apply_slash_events_to_claim(deps.storage, &mut claim)?;

    let mut global_state = GLOBAL_STATE.load(deps.storage)?;
    global_state.total_mars_for_claimers = global_state
        .total_mars_for_claimers
        .checked_sub(claim.amount)?;

    CLAIMS.remove(deps.storage, &info.sender);
    GLOBAL_STATE.save(deps.storage, &global_state)?;

    let config = CONFIG.load(deps.storage)?;
    let mars_token_address = address_provider::helpers::query_address(
        &deps.querier,
        config.address_provider_address,
        MarsContract::MarsToken,
    )?;

    let recipient = option_recipient.unwrap_or_else(|| info.sender.to_string());

    let res = Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: mars_token_address.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: recipient.clone(),
                amount: claim.amount,
            })?,
        }))
        .add_attribute("action", "claim")
        .add_attribute("claimer", info.sender)
        .add_attribute("mars_claimed", claim.amount)
        .add_attribute("recipient", recipient);
    Ok(res)
}

pub fn execute_transfer_mars(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient_unchecked: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(MarsError::Unauthorized {}.into());
    }

    let mars_token_address = address_provider::helpers::query_address(
        &deps.querier,
        config.address_provider_address,
        MarsContract::MarsToken,
    )?;

    let total_mars_in_staking_contract = cw20_get_balance(
        &deps.querier,
        mars_token_address.clone(),
        env.contract.address,
    )?;

    if amount > total_mars_in_staking_contract {
        return Err(ContractError::TransferMarsAmountTooLarge {});
    }

    let slash_percentage = Decimal::from_ratio(amount, total_mars_in_staking_contract);

    SLASH_EVENTS.save(
        deps.storage,
        U64Key::new(env.block.height),
        &SlashEvent { slash_percentage },
    )?;

    let mut global_state = GLOBAL_STATE.load(deps.storage)?;
    global_state.total_mars_for_claimers =
        global_state.total_mars_for_claimers * (Decimal::one() - slash_percentage);
    GLOBAL_STATE.save(deps.storage, &global_state)?;

    let res = Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: mars_token_address.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: recipient_unchecked.clone(),
                amount,
            })?,
        }))
        .add_attribute("action", "transfer_mars")
        .add_attribute("recipient", recipient_unchecked)
        .add_attribute("amount", amount)
        .add_attribute("slash_percentage", slash_percentage.to_string())
        .add_attribute(
            "new_total_mars_for_claimers",
            global_state.total_mars_for_claimers,
        );

    Ok(res)
}

pub fn execute_swap_asset_to_uusd(
    deps: DepsMut,
    env: Env,
    offer_asset_info: AssetInfo,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // throw error if the user tries to swap Mars
    let mars_token_address = address_provider::helpers::query_address(
        &deps.querier,
        config.address_provider_address,
        MarsContract::MarsToken,
    )?;

    if let AssetInfo::Token { contract_addr } = offer_asset_info.clone() {
        if contract_addr.to_string().to_lowercase() == mars_token_address.to_string().to_lowercase()
        {
            return Err(ContractError::MarsCannotSwap {});
        }
    }

    let ask_asset_info = AssetInfo::NativeToken {
        denom: "uusd".to_string(),
    };

    let astroport_max_spread = Some(config.astroport_max_spread);

    Ok(execute_swap(
        deps,
        env,
        offer_asset_info,
        ask_asset_info,
        amount,
        config.astroport_factory_address,
        astroport_max_spread,
    )?)
}

pub fn execute_swap_uusd_to_mars(
    deps: DepsMut,
    env: Env,
    amount: Option<Uint128>,
) -> Result<Response, MarsError> {
    let config = CONFIG.load(deps.storage)?;

    let offer_asset_info = AssetInfo::NativeToken {
        denom: "uusd".to_string(),
    };

    let mars_token_address = address_provider::helpers::query_address(
        &deps.querier,
        config.address_provider_address,
        MarsContract::MarsToken,
    )?;

    let ask_asset_info = AssetInfo::Token {
        contract_addr: mars_token_address,
    };

    let astroport_max_spread = Some(config.astroport_max_spread);

    Ok(execute_swap(
        deps,
        env,
        offer_asset_info,
        ask_asset_info,
        amount,
        config.astroport_factory_address,
        astroport_max_spread,
    )?)
}

// QUERY

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::GlobalState {} => to_binary(&query_global_state(deps)?),
        QueryMsg::XMarsPerMars {} => to_binary(&query_xmars_per_mars(deps, env)?),
        QueryMsg::MarsPerXMars {} => to_binary(&query_mars_per_xmars(deps, env)?),
        QueryMsg::Claim { user_address } => to_binary(&query_claim(deps, env, user_address)?),
    }
}

fn query_config(deps: Deps) -> StdResult<Config> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}

fn query_global_state(deps: Deps) -> StdResult<GlobalState> {
    let global_state = GLOBAL_STATE.load(deps.storage)?;
    Ok(global_state)
}

fn query_xmars_per_mars(deps: Deps, env: Env) -> StdResult<Option<Decimal>> {
    let config = CONFIG.load(deps.storage)?;
    let global_state = GLOBAL_STATE.load(deps.storage)?;

    let staking_tokens_info =
        get_staking_tokens_info(deps, &env, &config, &global_state, Uint128::zero())?;

    compute_xmars_per_mars(&staking_tokens_info)
}

fn query_mars_per_xmars(deps: Deps, env: Env) -> StdResult<Option<Decimal>> {
    let config = CONFIG.load(deps.storage)?;
    let global_state = GLOBAL_STATE.load(deps.storage)?;

    let staking_tokens_info =
        get_staking_tokens_info(deps, &env, &config, &global_state, Uint128::zero())?;

    compute_mars_per_xmars(&staking_tokens_info)
}

fn query_claim(deps: Deps, _env: Env, user_address_unchecked: String) -> StdResult<ClaimResponse> {
    let user_address = deps.api.addr_validate(&user_address_unchecked)?;
    let option_claim = CLAIMS.may_load(deps.storage, &user_address)?;

    if let Some(mut claim) = option_claim {
        apply_slash_events_to_claim(deps.storage, &mut claim)?;
        Ok(ClaimResponse { claim: Some(claim) })
    } else {
        Ok(ClaimResponse {
            claim: option_claim,
        })
    }
}

// HELPERS

/// Gets mars and xmars token addresses from address provider and returns them in a tuple.
fn get_token_addresses(deps: Deps, config: &Config) -> StdResult<(Addr, Addr)> {
    let mut addresses_query = address_provider::helpers::query_addresses(
        &deps.querier,
        config.address_provider_address.clone(),
        vec![MarsContract::MarsToken, MarsContract::XMarsToken],
    )
    .map_err(|_| StdError::generic_err("Failed to query token addresses"))?;

    let xmars_token_address = addresses_query.pop().unwrap();
    let mars_token_address = addresses_query.pop().unwrap();

    Ok((mars_token_address, xmars_token_address))
}

fn apply_slash_events_to_claim(storage: &dyn Storage, claim: &mut Claim) -> StdResult<()> {
    let start = Some(Bound::inclusive(U64Key::new(claim.created_at_block)));

    // Slash events are applied in chronological order
    for kv in SLASH_EVENTS.range(storage, start, None, Order::Ascending) {
        let (_, slash_event) = kv?;

        claim.amount = claim.amount * (Decimal::one() - slash_event.slash_percentage);
    }
    Ok(())
}

struct StakingTokensInfo {
    mars_token_address: Addr,
    xmars_token_address: Addr,
    total_mars_in_staking_contract: Uint128,
    total_mars_for_stakers: Uint128,
    total_xmars_supply: Uint128,
}

/// Gets mars and xmars info to check addresses and compute ratios
/// mars_to_deduct accounts for mars that are already in
/// the contract but should not be taken into account for the net amount
/// in the balance
fn get_staking_tokens_info(
    deps: Deps,
    env: &Env,
    config: &Config,
    global_state: &GlobalState,
    mars_to_deduct: Uint128,
) -> StdResult<StakingTokensInfo> {
    let (mars_token_address, xmars_token_address) = get_token_addresses(deps, config)?;

    let total_mars_in_staking_contract = cw20_get_balance(
        &deps.querier,
        mars_token_address.clone(),
        env.contract.address.clone(),
    )?
    .checked_sub(mars_to_deduct)?;
    let total_mars_for_stakers =
        total_mars_in_staking_contract.checked_sub(global_state.total_mars_for_claimers)?;

    let total_xmars_supply = cw20_get_total_supply(&deps.querier, xmars_token_address.clone())?;

    Ok(StakingTokensInfo {
        mars_token_address,
        xmars_token_address,
        total_mars_in_staking_contract,
        total_mars_for_stakers,
        total_xmars_supply,
    })
}

/// Compute the ratio between xMars and Mars token in terms of how many xMars token will be minted
/// by staking 1 Mars token.
fn compute_xmars_per_mars(staking_tokens_info: &StakingTokensInfo) -> StdResult<Option<Decimal>> {
    let total_mars_for_stakers = staking_tokens_info.total_mars_for_stakers;
    let total_xmars_supply = staking_tokens_info.total_xmars_supply;
    // Mars/xMars ratio is undefined if either `total_mars_for_stakers` or `total_xmars_supply` is zero
    // in this case, we return None
    if total_mars_for_stakers.is_zero() || total_xmars_supply.is_zero() {
        Ok(None)
    } else {
        Ok(Some(Decimal::from_ratio(
            staking_tokens_info.total_xmars_supply,
            staking_tokens_info.total_mars_for_stakers,
        )))
    }
}

/// Compute the ratio between Mars and xMars in terms of how many Mars tokens can be claimed by
/// burning 1 xMars token.
///
/// This is calculated by simply taking the inversion of `xmars_per_mars`.
fn compute_mars_per_xmars(staking_tokens_info: &StakingTokensInfo) -> StdResult<Option<Decimal>> {
    let total_mars_for_stakers = staking_tokens_info.total_mars_for_stakers;
    let total_xmars_supply = staking_tokens_info.total_xmars_supply;

    // Mars/xMars ratio is undefined if either `total_mars_for_stakers` or `total_xmars_supply` is zero
    // in this case, we return None
    if total_mars_for_stakers.is_zero() || total_xmars_supply.is_zero() {
        Ok(None)
    } else {
        Ok(Some(Decimal::from_ratio(
            total_mars_for_stakers,
            total_xmars_supply,
        )))
    }
}

// TESTS

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{
        attr, Addr, Coin, CosmosMsg, Decimal as StdDecimal, OwnedDeps, StdError, SubMsg, Timestamp,
    };
    use mars_core::testing::{
        mock_dependencies, mock_env, mock_env_at_block_height, mock_env_at_block_time,
        MarsMockQuerier, MockEnvParams,
    };

    const TEST_COOLDOWN_DURATION: u64 = 1000;

    #[test]
    fn test_proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        // *
        // init config with empty params
        // *
        let empty_config = CreateOrUpdateConfig {
            owner: None,
            address_provider_address: None,
            astroport_factory_address: None,
            astroport_max_spread: None,
            cooldown_duration: None,
        };
        let msg = InstantiateMsg {
            config: empty_config,
        };
        let info = mock_info("owner", &[]);
        let response = instantiate(
            deps.as_mut(),
            mock_env(MockEnvParams::default()),
            info.clone(),
            msg,
        )
        .unwrap_err();
        assert_eq!(
            response,
            ContractError::Mars(MarsError::InstantiateParamsUnavailable {})
        );

        let config = CreateOrUpdateConfig {
            owner: Some(String::from("owner")),
            address_provider_address: Some(String::from("address_provider")),
            astroport_factory_address: Some(String::from("astroport_factory")),
            astroport_max_spread: Some(StdDecimal::from_ratio(1u128, 100u128)),
            cooldown_duration: Some(20),
        };
        let msg = InstantiateMsg { config };

        let res =
            instantiate(deps.as_mut(), mock_env(MockEnvParams::default()), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let config = CONFIG.load(deps.as_ref().storage).unwrap();
        assert_eq!(config.owner, Addr::unchecked("owner"));
        assert_eq!(
            config.address_provider_address,
            Addr::unchecked("address_provider")
        );
    }

    #[test]
    fn test_update_config() {
        let mut deps = mock_dependencies(&[]);

        // *
        // init config with valid params
        // *
        let init_config = CreateOrUpdateConfig {
            owner: Some(String::from("owner")),
            address_provider_address: Some(String::from("address_provider")),
            astroport_factory_address: Some(String::from("astroport_factory")),
            astroport_max_spread: Some(StdDecimal::from_ratio(1u128, 100u128)),
            cooldown_duration: Some(20),
        };
        let msg = InstantiateMsg {
            config: init_config.clone(),
        };
        let info = mock_info("owner", &[]);
        let _res =
            instantiate(deps.as_mut(), mock_env(MockEnvParams::default()), info, msg).unwrap();

        // *
        // non owner is not authorized
        // *
        let msg = ExecuteMsg::UpdateConfig {
            config: init_config,
        };
        let info = mock_info("somebody", &[]);
        let error_res =
            execute(deps.as_mut(), mock_env(MockEnvParams::default()), info, msg).unwrap_err();
        assert_eq!(error_res, ContractError::Mars(MarsError::Unauthorized {}));

        // *
        // update config with all new params
        // *
        let config = CreateOrUpdateConfig {
            owner: Some(String::from("new_owner")),
            address_provider_address: Some(String::from("new_address_provider")),
            astroport_factory_address: Some(String::from("new_factory")),
            astroport_max_spread: Some(StdDecimal::from_ratio(2u128, 100u128)),
            cooldown_duration: Some(200),
        };
        let msg = ExecuteMsg::UpdateConfig {
            config: config.clone(),
        };
        let info = mock_info("owner", &[]);
        // we can just call .unwrap() to assert this was a success
        let res = execute(deps.as_mut(), mock_env(MockEnvParams::default()), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Read config from state
        let new_config = CONFIG.load(deps.as_ref().storage).unwrap();

        assert_eq!(new_config.owner, "new_owner");
        assert_eq!(new_config.address_provider_address, "new_address_provider");
        assert_eq!(new_config.astroport_factory_address, "new_factory");
        assert_eq!(
            new_config.cooldown_duration,
            config.cooldown_duration.unwrap()
        );
    }

    #[test]
    fn test_stake() {
        let mut deps = th_setup(&[]);

        // no Mars in pool
        // stake X Mars -> should receive X xMars
        {
            let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
                sender: String::from("staker"),
                amount: Uint128::new(2_000_000),
                msg: to_binary(&ReceiveMsg::Stake { recipient: None }).unwrap(),
            });

            deps.querier.set_cw20_balances(
                Addr::unchecked("mars_token"),
                &[(Addr::unchecked(MOCK_CONTRACT_ADDR), Uint128::new(2_000_000))],
            );

            deps.querier
                .set_cw20_total_supply(Addr::unchecked("xmars_token"), Uint128::zero());

            let info = mock_info("mars_token", &[]);
            let res = execute(
                deps.as_mut(),
                mock_env(MockEnvParams::default()),
                info.clone(),
                msg,
            )
            .unwrap();

            assert_eq!(
                vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("xmars_token"),
                    funds: vec![],
                    msg: to_binary(&Cw20ExecuteMsg::Mint {
                        recipient: String::from("staker"),
                        amount: Uint128::new(2_000_000),
                    })
                    .unwrap(),
                }))],
                res.messages
            );
            assert_eq!(
                vec![
                    attr("action", "stake"),
                    attr("staker", String::from("staker")),
                    attr("mars_staked", 2_000_000.to_string()),
                    attr("xmars_minted", 2_000_000.to_string()),
                    attr("recipient", String::from("staker")),
                ],
                res.attributes
            );
        }

        // Some Mars in pool and some xMars supply
        // * stake Mars -> should receive less xMars
        // * set recipient -> should send xMars to recipient
        // * some open claims -> do not count on staked mars
        {
            let stake_amount = Uint128::new(2_000_000);
            let mars_in_contract = Uint128::new(4_000_000);
            let xmars_supply = Uint128::new(1_000_000);
            let total_mars_for_claimers = Uint128::new(500_000);

            GLOBAL_STATE
                .save(
                    &mut deps.storage,
                    &GlobalState {
                        total_mars_for_claimers: total_mars_for_claimers,
                    },
                )
                .unwrap();

            deps.querier.set_cw20_balances(
                Addr::unchecked("mars_token"),
                &[(Addr::unchecked(MOCK_CONTRACT_ADDR), mars_in_contract)],
            );

            deps.querier
                .set_cw20_total_supply(Addr::unchecked("xmars_token"), xmars_supply);

            let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
                msg: to_binary(&ReceiveMsg::Stake {
                    recipient: Some(String::from("recipient")),
                })
                .unwrap(),

                sender: String::from("staker"),
                amount: stake_amount,
            });
            let info = mock_info("mars_token", &[]);

            let res =
                execute(deps.as_mut(), mock_env(MockEnvParams::default()), info, msg).unwrap();

            let expected_minted_xmars = stake_amount.multiply_ratio(
                xmars_supply,
                mars_in_contract - stake_amount - total_mars_for_claimers,
            );

            assert_eq!(
                vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("xmars_token"),
                    funds: vec![],
                    msg: to_binary(&Cw20ExecuteMsg::Mint {
                        recipient: String::from("recipient"),
                        amount: expected_minted_xmars,
                    })
                    .unwrap(),
                }))],
                res.messages
            );
            assert_eq!(
                vec![
                    attr("action", "stake"),
                    attr("staker", String::from("staker")),
                    attr("mars_staked", stake_amount),
                    attr("xmars_minted", expected_minted_xmars),
                    attr("recipient", String::from("recipient")),
                ],
                res.attributes
            );
        }

        // stake other token -> Unauthorized
        {
            let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
                sender: String::from("staker"),
                amount: Uint128::new(2_000_000),
                msg: to_binary(&ReceiveMsg::Stake { recipient: None }).unwrap(),
            });

            let info = mock_info("other_token", &[]);
            let res_error =
                execute(deps.as_mut(), mock_env(MockEnvParams::default()), info, msg).unwrap_err();
            assert_eq!(res_error, ContractError::Mars(MarsError::Unauthorized {}));
        }
    }

    #[test]
    fn test_unstake() {
        let mut deps = th_setup(&[]);

        // setup variables for unstake
        let unstake_amount = Uint128::new(1_000_000);
        let unstake_mars_in_contract = Uint128::new(4_000_000);
        let unstake_xmars_supply = Uint128::new(3_000_000);
        let unstake_height = 123456;
        let unstake_time = 1_000_000_000;
        let env = mock_env(MockEnvParams {
            block_height: unstake_height,
            block_time: Timestamp::from_seconds(unstake_time),
        });
        let initial_mars_for_claimers = Uint128::new(700_000);
        let mut mars_for_claimers = initial_mars_for_claimers;

        deps.querier.set_cw20_balances(
            Addr::unchecked("mars_token"),
            &[(
                Addr::unchecked(MOCK_CONTRACT_ADDR),
                unstake_mars_in_contract,
            )],
        );
        deps.querier
            .set_cw20_total_supply(Addr::unchecked("xmars_token"), unstake_xmars_supply);
        GLOBAL_STATE
            .save(
                &mut deps.storage,
                &GlobalState {
                    total_mars_for_claimers: initial_mars_for_claimers,
                },
            )
            .unwrap();

        // unstake other token -> Unauthorized
        {
            let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
                msg: to_binary(&ReceiveMsg::Unstake {
                    recipient: Some(String::from("recipient")),
                })
                .unwrap(),
                sender: String::from("staker"),
                amount: unstake_amount,
            });
            let info = mock_info("other_token", &[]);
            let res_error = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap_err();
            assert_eq!(res_error, ContractError::Mars(MarsError::Unauthorized {}));
        }

        // valid unstake
        {
            let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
                msg: to_binary(&ReceiveMsg::Unstake {
                    recipient: Some(String::from("recipient")),
                })
                .unwrap(),
                sender: String::from("staker"),
                amount: unstake_amount,
            });
            let info = mock_info("xmars_token", &[]);

            let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();

            let expected_claimable_mars = unstake_amount.multiply_ratio(
                unstake_mars_in_contract - initial_mars_for_claimers,
                unstake_xmars_supply,
            );

            assert_eq!(
                vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("xmars_token"),
                    funds: vec![],
                    msg: to_binary(&Cw20ExecuteMsg::Burn {
                        amount: unstake_amount,
                    })
                    .unwrap(),
                })),],
                res.messages
            );
            assert_eq!(
                vec![
                    attr("action", "unstake"),
                    attr("staker", String::from("staker")),
                    attr("recipient", String::from("recipient")),
                    attr("xmars_burned", unstake_amount),
                    attr("mars_claimable", expected_claimable_mars),
                ],
                res.attributes
            );

            let claim = CLAIMS
                .load(&deps.storage, &Addr::unchecked("recipient"))
                .unwrap();

            assert_eq!(
                claim,
                Claim {
                    created_at_block: unstake_height,
                    cooldown_end_timestamp: unstake_time + TEST_COOLDOWN_DURATION,
                    amount: expected_claimable_mars,
                }
            );

            mars_for_claimers += expected_claimable_mars;

            let global_state = GLOBAL_STATE.load(&deps.storage).unwrap();

            assert_eq!(global_state.total_mars_for_claimers, mars_for_claimers);
        }

        // cannot unstake again (recipient has an open claim)
        {
            let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
                msg: to_binary(&ReceiveMsg::Unstake {
                    recipient: Some(String::from("recipient")),
                })
                .unwrap(),
                sender: String::from("staker"),
                amount: unstake_amount,
            });
            let info = mock_info("xmars_token", &[]);

            let err = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap_err();

            assert_eq!(err, ContractError::UnstakeActiveClaim {});
        }

        // unstake again, but use `None` as recipient
        // recipient should default to staker, which does not have an open claim
        {
            let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
                msg: to_binary(&ReceiveMsg::Unstake { recipient: None }).unwrap(),
                sender: String::from("staker"),
                amount: unstake_amount,
            });
            let info = mock_info("xmars_token", &[]);

            let res = execute(deps.as_mut(), env, info, msg.clone()).unwrap();

            assert_eq!(attr("recipient", String::from("staker")), res.attributes[2]);

            let expected_claimable_mars = unstake_amount.multiply_ratio(
                unstake_mars_in_contract - mars_for_claimers,
                unstake_xmars_supply,
            );

            let claim = CLAIMS
                .load(&deps.storage, &Addr::unchecked("staker"))
                .unwrap();

            assert_eq!(
                claim,
                Claim {
                    created_at_block: unstake_height,
                    cooldown_end_timestamp: unstake_time + TEST_COOLDOWN_DURATION,
                    amount: expected_claimable_mars,
                }
            );

            mars_for_claimers += expected_claimable_mars;

            let global_state = GLOBAL_STATE.load(&deps.storage).unwrap();

            assert_eq!(global_state.total_mars_for_claimers, mars_for_claimers);
        }
    }

    #[test]
    fn test_claim() {
        let mut deps = th_setup(&[]);
        let initial_mars_for_claimers = Uint128::new(4_000_000_000000);
        let claimer_address = Addr::unchecked("claimer");
        let claim = Claim {
            amount: Uint128::new(5_000_000000),
            created_at_block: 123456_u64,
            cooldown_end_timestamp: 1_000_000_u64,
        };

        CLAIMS
            .save(&mut deps.storage, &claimer_address, &claim)
            .unwrap();
        GLOBAL_STATE
            .save(
                &mut deps.storage,
                &GlobalState {
                    total_mars_for_claimers: initial_mars_for_claimers,
                },
            )
            .unwrap();

        // Claim previous to cooldown end fails
        {
            let info = mock_info("claimer", &[]);
            let env = mock_env_at_block_time(999_999);
            let msg = ExecuteMsg::Claim { recipient: None };
            let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
            assert_eq!(err, ContractError::ClaimCooldownNotEnded {});
        }

        // Query claim gives correct claim
        {
            let queried_claim = query_claim(
                deps.as_ref(),
                mock_env_at_block_time(1_233_000),
                "claimer".to_string(),
            )
            .unwrap();
            assert_eq!(claim.amount, queried_claim.claim.unwrap().amount);
        }

        // Successful claim
        {
            let info = mock_info("claimer", &[]);
            let env = mock_env_at_block_time(1_233_000);
            let msg = ExecuteMsg::Claim { recipient: None };
            let res = execute(deps.as_mut(), env, info, msg).unwrap();

            assert_eq!(
                res.messages,
                vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("mars_token"),
                    funds: vec![],
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: claimer_address.clone().to_string(),
                        amount: claim.amount,
                    })
                    .unwrap(),
                })),]
            );

            assert_eq!(
                res.attributes,
                vec![
                    attr("action", "claim"),
                    attr("claimer", "claimer"),
                    attr("mars_claimed", claim.amount),
                    attr("recipient", "claimer"),
                ]
            );

            let queried_claim = query_claim(
                deps.as_ref(),
                mock_env_at_block_time(1_233_000),
                "claimer".to_string(),
            )
            .unwrap();
            assert_eq!(None, queried_claim.claim);

            let global_state = GLOBAL_STATE.load(&deps.storage).unwrap();
            assert_eq!(
                global_state.total_mars_for_claimers,
                initial_mars_for_claimers - claim.amount
            );
            assert_eq!(
                CLAIMS.may_load(&deps.storage, &claimer_address).unwrap(),
                None
            );
        }

        // Claim now fails (it was deleted)
        {
            let info = mock_info("claimer", &[]);
            let env = mock_env_at_block_time(1_233_000);
            let msg = ExecuteMsg::Claim { recipient: None };
            let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
            assert!(
                matches!(err, ContractError::Std(StdError::NotFound { .. })),
                "Expected StdError::NotFound, received {}",
                err
            );
        }
    }

    #[test]
    fn test_claim_with_slash() {
        let mut deps = th_setup(&[]);
        let claimer_address = Addr::unchecked("claimer");

        let initial_mars_for_claimers = Uint128::new(100_000_000_00000);
        let initial_claim_amount = Uint128::new(5_000_000000);
        let claim = Claim {
            amount: initial_claim_amount,
            created_at_block: 100_000_u64,
            cooldown_end_timestamp: 1_000_000_u64,
        };

        let claim_height = 150_000_u64;
        let claim_time = 1_000_000_u64;
        let env = mock_env(MockEnvParams {
            block_height: claim_height,
            block_time: Timestamp::from_seconds(claim_time),
        });

        let slash_percentage_one = Decimal::from_ratio(1_u128, 2_u128);
        let slash_percentage_two = Decimal::from_ratio(1_u128, 3_u128);

        CLAIMS
            .save(&mut deps.storage, &claimer_address, &claim)
            .unwrap();
        GLOBAL_STATE
            .save(
                &mut deps.storage,
                &GlobalState {
                    total_mars_for_claimers: initial_mars_for_claimers,
                },
            )
            .unwrap();

        SLASH_EVENTS
            .save(
                &mut deps.storage,
                U64Key::new(claim.created_at_block - 1),
                &SlashEvent {
                    slash_percentage: Decimal::from_ratio(80_u128, 100_u128),
                },
            )
            .unwrap();
        SLASH_EVENTS
            .save(
                &mut deps.storage,
                U64Key::new(claim.created_at_block),
                &SlashEvent {
                    slash_percentage: slash_percentage_one,
                },
            )
            .unwrap();

        // one slash (slashes previous to claim don't count)
        // set other as recipient
        {
            let expected_claim_amount =
                initial_claim_amount * (Decimal::one() - slash_percentage_one);
            let queried_claim = query_claim(
                deps.as_ref(),
                mock_env_at_block_time(1_233_000),
                "claimer".to_string(),
            )
            .unwrap();
            assert_eq!(expected_claim_amount, queried_claim.claim.unwrap().amount);

            let info = mock_info("claimer", &[]);
            let msg = ExecuteMsg::Claim {
                recipient: Some("recipient".to_string()),
            };
            let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

            assert_eq!(
                res.messages,
                vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("mars_token"),
                    funds: vec![],
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: "recipient".to_string(),
                        amount: expected_claim_amount,
                    })
                    .unwrap(),
                })),]
            );

            assert_eq!(
                res.attributes,
                vec![
                    attr("action", "claim"),
                    attr("claimer", "claimer"),
                    attr("mars_claimed", expected_claim_amount),
                    attr("recipient", "recipient"),
                ]
            );

            let global_state = GLOBAL_STATE.load(&deps.storage).unwrap();
            assert_eq!(
                global_state.total_mars_for_claimers,
                initial_mars_for_claimers - expected_claim_amount
            );
            assert_eq!(
                CLAIMS.may_load(&deps.storage, &claimer_address).unwrap(),
                None
            );
        }

        // create claim again as previous was deleted
        CLAIMS
            .save(&mut deps.storage, &claimer_address, &claim)
            .unwrap();
        GLOBAL_STATE
            .save(
                &mut deps.storage,
                &GlobalState {
                    total_mars_for_claimers: initial_mars_for_claimers,
                },
            )
            .unwrap();
        SLASH_EVENTS
            .save(
                &mut deps.storage,
                U64Key::new(claim.created_at_block + 200),
                &SlashEvent {
                    slash_percentage: slash_percentage_two,
                },
            )
            .unwrap();

        // two slashes
        {
            let expected_claim_amount = (initial_claim_amount
                * (Decimal::one() - slash_percentage_one))
                * (Decimal::one() - slash_percentage_two);
            let queried_claim = query_claim(
                deps.as_ref(),
                mock_env_at_block_time(1_233_000),
                "claimer".to_string(),
            )
            .unwrap();
            assert_eq!(expected_claim_amount, queried_claim.claim.unwrap().amount);

            let info = mock_info("claimer", &[]);
            let msg = ExecuteMsg::Claim { recipient: None };
            let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

            assert_eq!(
                res.messages,
                vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("mars_token"),
                    funds: vec![],
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: "claimer".to_string(),
                        amount: expected_claim_amount,
                    })
                    .unwrap(),
                })),]
            );

            assert_eq!(
                res.attributes,
                vec![
                    attr("action", "claim"),
                    attr("claimer", "claimer"),
                    attr("mars_claimed", expected_claim_amount),
                    attr("recipient", "claimer"),
                ]
            );

            let global_state = GLOBAL_STATE.load(&deps.storage).unwrap();
            assert_eq!(
                global_state.total_mars_for_claimers,
                initial_mars_for_claimers - expected_claim_amount
            );
            assert_eq!(
                CLAIMS.may_load(&deps.storage, &claimer_address).unwrap(),
                None
            );
        }
    }

    #[test]
    fn test_transfer_mars() {
        let mut deps = th_setup(&[]);
        let initial_mars_for_claimers = Uint128::new(4_000_000_000000);
        let initial_mars_in_contract = Uint128::new(10_000_000_000000);
        let transfer_amount = Uint128::new(4_000_000_000000);
        let transfer_block = 123456_u64;

        deps.querier.set_cw20_balances(
            Addr::unchecked("mars_token"),
            &[(
                Addr::unchecked(MOCK_CONTRACT_ADDR),
                initial_mars_in_contract,
            )],
        );

        GLOBAL_STATE
            .save(
                &mut deps.storage,
                &GlobalState {
                    total_mars_for_claimers: initial_mars_for_claimers,
                },
            )
            .unwrap();

        // Transfer by non owner fails
        {
            let env = mock_env(MockEnvParams::default());
            let info = mock_info("anyone", &[]);
            let msg = ExecuteMsg::TransferMars {
                recipient: "recipient".to_string(),
                amount: transfer_amount,
            };
            let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
            assert_eq!(err, ContractError::Mars(MarsError::Unauthorized {}));
        }

        // Transfer big amount fails
        {
            let env = mock_env(MockEnvParams::default());
            let info = mock_info("owner", &[]);
            let msg = ExecuteMsg::TransferMars {
                recipient: "recipient".to_string(),
                amount: initial_mars_in_contract + Uint128::new(10),
            };
            let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
            assert_eq!(err, ContractError::TransferMarsAmountTooLarge {});
        }

        // Successful transfer
        {
            let env = mock_env_at_block_height(transfer_block);
            let info = mock_info("owner", &[]);
            let msg = ExecuteMsg::TransferMars {
                recipient: "recipient".to_string(),
                amount: transfer_amount,
            };
            let res = execute(deps.as_mut(), env, info, msg).unwrap();
            assert_eq!(
                res.messages,
                vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("mars_token"),
                    funds: vec![],
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: "recipient".to_string(),
                        amount: transfer_amount,
                    })
                    .unwrap(),
                })),]
            );

            let expected_slash_percentage =
                Decimal::from_ratio(transfer_amount, initial_mars_in_contract);

            // should be reduced proportionally
            let expected_total_mars_for_claimers = initial_mars_for_claimers.multiply_ratio(
                initial_mars_in_contract - transfer_amount,
                initial_mars_in_contract,
            );

            assert_eq!(
                res.attributes,
                vec![
                    attr("action", "transfer_mars"),
                    attr("recipient", "recipient"),
                    attr("amount", transfer_amount),
                    attr("slash_percentage", expected_slash_percentage.to_string()),
                    attr(
                        "new_total_mars_for_claimers",
                        expected_total_mars_for_claimers
                    ),
                ]
            );

            let slash_event = SLASH_EVENTS
                .load(&deps.storage, U64Key::new(transfer_block))
                .unwrap();
            assert_eq!(
                slash_event,
                SlashEvent {
                    slash_percentage: expected_slash_percentage
                }
            );

            let global_state = GLOBAL_STATE.load(&deps.storage).unwrap();
            assert_eq!(
                global_state.total_mars_for_claimers,
                expected_total_mars_for_claimers
            );
        }
    }

    #[test]
    fn test_cannot_swap_mars() {
        let mut deps = th_setup(&[]);
        // *
        // can't swap Mars with SwapAssetToUusd
        // *
        let msg = ExecuteMsg::SwapAssetToUusd {
            offer_asset_info: AssetInfo::Token {
                contract_addr: Addr::unchecked("mars_token"),
            },
            amount: None,
        };
        let info = mock_info("owner", &[]);
        let response =
            execute(deps.as_mut(), mock_env(MockEnvParams::default()), info, msg).unwrap_err();
        assert_eq!(response, ContractError::MarsCannotSwap {});

        // *
        // Check for case sensitivity
        // *
        let msg = ExecuteMsg::SwapAssetToUusd {
            offer_asset_info: AssetInfo::Token {
                contract_addr: Addr::unchecked("MARS_token"),
            },
            amount: None,
        };
        let info = mock_info("owner", &[]);
        let response =
            execute(deps.as_mut(), mock_env(MockEnvParams::default()), info, msg).unwrap_err();
        assert_eq!(response, ContractError::MarsCannotSwap {});
    }

    // TEST HELPERS
    fn th_setup(contract_balances: &[Coin]) -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier> {
        let mut deps = mock_dependencies(contract_balances);

        let config = CreateOrUpdateConfig {
            owner: Some(String::from("owner")),
            address_provider_address: Some(String::from("address_provider")),
            astroport_factory_address: Some(String::from("astroport_factory")),
            astroport_max_spread: Some(StdDecimal::from_ratio(1u128, 100u128)),
            cooldown_duration: Some(TEST_COOLDOWN_DURATION),
        };
        let msg = InstantiateMsg { config };
        let info = mock_info("owner", &[]);
        instantiate(deps.as_mut(), mock_env(MockEnvParams::default()), info, msg).unwrap();

        deps
    }
}
