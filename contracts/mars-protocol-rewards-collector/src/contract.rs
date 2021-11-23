use cosmwasm_std::{
    entry_point, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, Uint128, WasmMsg,
};

use mars_core::astroport::asset::AssetInfo;

use mars_core::asset::{build_send_asset_with_tax_deduction_msg, get_asset_balance, Asset};
use mars_core::error::MarsError;
use mars_core::helpers::{option_string_to_addr, zero_address};
use mars_core::swapping::execute_swap;

use mars_core::address_provider::{self, MarsContract};
use mars_core::red_bank;

use crate::error::ContractError;
use crate::msg::{CreateOrUpdateConfig, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{ASSET_CONFIG, CONFIG};
use crate::{AssetConfig, Config};

// INIT

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
        address_provider_address,
        safety_fund_fee_share,
        treasury_fee_share,
        astroport_factory_address,
        astroport_max_spread,
    } = msg.config;

    // All fields should be available
    let available = owner.is_some()
        && address_provider_address.is_some()
        && safety_fund_fee_share.is_some()
        && treasury_fee_share.is_some()
        && astroport_factory_address.is_some()
        && astroport_max_spread.is_some();

    if !available {
        return Err(MarsError::InstantiateParamsUnavailable {}.into());
    };

    let config = Config {
        owner: option_string_to_addr(deps.api, owner, zero_address())?,
        address_provider_address: option_string_to_addr(
            deps.api,
            address_provider_address,
            zero_address(),
        )?,
        safety_fund_fee_share: safety_fund_fee_share.unwrap(),
        treasury_fee_share: treasury_fee_share.unwrap(),
        astroport_factory_address: option_string_to_addr(
            deps.api,
            astroport_factory_address,
            zero_address(),
        )?,
        astroport_max_spread: astroport_max_spread.unwrap(),
    };

    config.validate()?;

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

// HANDLERS

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { config } => execute_update_config(deps, env, info, config),
        ExecuteMsg::UpdateAssetConfig { asset, enabled } => {
            execute_update_asset_config(deps, env, info, asset, enabled)
        }
        ExecuteMsg::WithdrawFromRedBank { asset, amount } => {
            execute_withdraw_from_red_bank(deps, env, info, asset, amount)
        }
        ExecuteMsg::DistributeProtocolRewards { asset, amount } => {
            execute_distribute_protocol_rewards(deps, env, info, asset, amount)
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
        ExecuteMsg::ExecuteCosmosMsg(cosmos_msg) => {
            Ok(execute_execute_cosmos_msg(deps, env, info, cosmos_msg)?)
        }
    }
}

/// Update config
pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_config: CreateOrUpdateConfig,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(MarsError::Unauthorized {}.into());
    }

    // Destructuring a struct’s fields into separate variables in order to force
    // compile error if we add more params
    let CreateOrUpdateConfig {
        owner,
        address_provider_address,
        safety_fund_fee_share,
        treasury_fee_share,
        astroport_factory_address,
        astroport_max_spread,
    } = new_config;

    config.owner = option_string_to_addr(deps.api, owner, config.owner)?;
    config.address_provider_address = option_string_to_addr(
        deps.api,
        address_provider_address,
        config.address_provider_address,
    )?;
    config.safety_fund_fee_share = safety_fund_fee_share.unwrap_or(config.safety_fund_fee_share);
    config.treasury_fee_share = treasury_fee_share.unwrap_or(config.treasury_fee_share);
    config.astroport_factory_address = option_string_to_addr(
        deps.api,
        astroport_factory_address,
        config.astroport_factory_address,
    )?;
    config.astroport_max_spread = astroport_max_spread.unwrap_or(config.astroport_max_spread);

    config.validate()?;

    CONFIG.save(deps.storage, &config)?;

    let res = Response::new().add_attribute("action", "update_config");
    Ok(res)
}

/// Update config
pub fn execute_update_asset_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    asset: Asset,
    enabled: bool,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(MarsError::Unauthorized {}.into());
    }

    let (asset_label, asset_reference, _) = asset.get_attributes();

    let new_asset_config = AssetConfig {
        enabled_for_distribution: enabled,
    };

    ASSET_CONFIG.save(deps.storage, asset_reference.as_slice(), &new_asset_config)?;

    let res = Response::new()
        .add_attribute("action", "update_asset_config")
        .add_attribute("asset", asset_label);
    Ok(res)
}

pub fn execute_withdraw_from_red_bank(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    asset: Asset,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let red_bank_address = address_provider::helpers::query_address(
        &deps.querier,
        config.address_provider_address,
        MarsContract::RedBank,
    )?;

    let withdraw_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: red_bank_address.to_string(),
        msg: to_binary(&red_bank::msg::ExecuteMsg::Withdraw { asset, amount })?,
        funds: vec![],
    });

    let res = Response::new()
        .add_attribute("action", "withdraw_from_red_bank")
        .add_message(withdraw_msg);

    Ok(res)
}

/// Send accumulated asset rewards to protocol contracts
pub fn execute_distribute_protocol_rewards(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    asset: Asset,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let (asset_label, asset_reference, asset_type) = asset.get_attributes();

    let asset_config = ASSET_CONFIG
        .load(deps.storage, &asset_reference)
        .unwrap_or_default();

    if !asset_config.enabled_for_distribution {
        return Err(ContractError::AssetNotEnabled { asset_label });
    }

    let balance = get_asset_balance(
        deps.as_ref(),
        env.contract.address,
        asset_label.clone(),
        asset_type,
    )?;

    let amount_to_distribute = match amount {
        Some(amount) if amount > balance => {
            return Err(ContractError::AmountToDistributeTooLarge { amount, balance })
        }
        Some(amount) => amount,
        None => balance,
    };

    let mars_contracts = vec![
        MarsContract::SafetyFund,
        MarsContract::Staking,
        MarsContract::Treasury,
    ];
    let mut addresses_query = address_provider::helpers::query_addresses(
        &deps.querier,
        config.address_provider_address,
        mars_contracts,
    )?;

    let treasury_address = addresses_query.pop().unwrap();
    let staking_address = addresses_query.pop().unwrap();
    let safety_fund_address = addresses_query.pop().unwrap();

    let safety_fund_amount = amount_to_distribute * config.safety_fund_fee_share;
    let treasury_amount = amount_to_distribute * config.treasury_fee_share;
    let amount_to_distribute_before_staking_rewards =
        safety_fund_amount.checked_add(treasury_amount)?;
    let staking_amount =
        amount_to_distribute.checked_sub(amount_to_distribute_before_staking_rewards)?;

    // only build and add send message if fee is non-zero
    let mut messages = vec![];
    if !safety_fund_amount.is_zero() {
        let safety_fund_msg = build_send_asset_with_tax_deduction_msg(
            deps.as_ref(),
            safety_fund_address,
            asset_label.clone(),
            asset_type,
            safety_fund_amount,
        )?;
        messages.push(safety_fund_msg);
    }
    if !treasury_amount.is_zero() {
        let treasury_msg = build_send_asset_with_tax_deduction_msg(
            deps.as_ref(),
            treasury_address,
            asset_label.clone(),
            asset_type,
            treasury_amount,
        )?;
        messages.push(treasury_msg);
    }
    if !staking_amount.is_zero() {
        let staking_msg = build_send_asset_with_tax_deduction_msg(
            deps.as_ref(),
            staking_address,
            asset_label.clone(),
            asset_type,
            staking_amount,
        )?;
        messages.push(staking_msg);
    }

    let res = Response::new()
        .add_attribute("action", "distribute_protocol_income")
        .add_attribute("asset", asset_label)
        .add_attribute(
            "total_distributed_amount",
            safety_fund_amount + treasury_amount + staking_amount,
        )
        .add_attribute("safety_fund_amount", safety_fund_amount)
        .add_attribute("treasury_amount", treasury_amount)
        .add_attribute("staking_amount", staking_amount)
        .add_messages(messages);

    Ok(res)
}

/// Swap any asset on the contract to uusd
pub fn execute_swap_asset_to_uusd(
    deps: DepsMut,
    env: Env,
    offer_asset_info: AssetInfo,
    amount: Option<Uint128>,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    let ask_asset_info = AssetInfo::NativeToken {
        denom: "uusd".to_string(),
    };

    let astroport_max_spread = Some(config.astroport_max_spread);

    execute_swap(
        deps,
        env,
        offer_asset_info,
        ask_asset_info,
        amount,
        config.astroport_factory_address,
        astroport_max_spread,
    )
}

/// Execute Cosmos message
pub fn execute_execute_cosmos_msg(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: CosmosMsg,
) -> Result<Response, MarsError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(MarsError::Unauthorized {});
    }

    let response = Response::new()
        .add_attribute("action", "execute_cosmos_msg")
        .add_message(msg);

    Ok(response)
}

// QUERIES

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::AssetConfig { asset } => to_binary(&query_asset_config(deps, asset)?),
    }
}

fn query_config(deps: Deps) -> StdResult<Config> {
    let config = CONFIG.load(deps.storage)?;

    Ok(config)
}

fn query_asset_config(deps: Deps, asset: Asset) -> StdResult<AssetConfig> {
    let reference = asset.get_reference();

    let asset_config = ASSET_CONFIG
        .load(deps.storage, &reference)
        .unwrap_or_default();

    Ok(asset_config)
}

// TESTS

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::{
        attr, coin, from_binary,
        testing::{mock_env, MockApi, MockStorage, MOCK_CONTRACT_ADDR},
        Addr, BankMsg, Coin, Decimal as StdDecimal, OwnedDeps, StdError, SubMsg,
    };

    use cw20::Cw20ExecuteMsg;

    use mars_core::math::decimal::Decimal;
    use mars_core::{
        tax::deduct_tax,
        testing::{mock_dependencies, mock_info, MarsMockQuerier},
    };

    use crate::ConfigError;

    #[test]
    fn test_proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        // Config with base params valid (just update the rest)
        let astroport_max_spread = StdDecimal::percent(1);
        let base_config = CreateOrUpdateConfig {
            owner: Some("owner".to_string()),
            address_provider_address: Some("address_provider".to_string()),
            safety_fund_fee_share: None,
            treasury_fee_share: None,
            astroport_factory_address: Some("astroport".to_string()),
            astroport_max_spread: Some(astroport_max_spread),
        };

        // *
        // init config with empty params
        // *
        let empty_config = CreateOrUpdateConfig {
            owner: None,
            address_provider_address: None,
            safety_fund_fee_share: None,
            treasury_fee_share: None,
            astroport_factory_address: None,
            astroport_max_spread: None,
        };
        let msg = InstantiateMsg {
            config: empty_config,
        };
        let info = mock_info("owner");
        let response = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
        assert_eq!(response, MarsError::InstantiateParamsUnavailable {}.into());

        // *
        // init config with safety_fund_fee_share and treasury_fee_share greater than 1
        // *
        let mut safety_fund_fee_share = Decimal::from_ratio(11u128, 10u128);
        let mut treasury_fee_share = Decimal::from_ratio(12u128, 10u128);
        let config = CreateOrUpdateConfig {
            safety_fund_fee_share: Some(safety_fund_fee_share),
            treasury_fee_share: Some(treasury_fee_share),
            ..base_config.clone()
        };
        let msg = InstantiateMsg { config };
        let response = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
        assert_eq!(
            response,
            ConfigError::Mars(MarsError::ParamsNotLessOrEqualOne {
                expected_params: "safety_fund_fee_share, treasury_fee_share".to_string(),
                invalid_params: "safety_fund_fee_share, treasury_fee_share".to_string()
            })
            .into()
        );

        // *
        // init config with invalid fee share amounts
        // *
        safety_fund_fee_share = Decimal::from_ratio(7u128, 10u128);
        treasury_fee_share = Decimal::from_ratio(4u128, 10u128);
        let config = CreateOrUpdateConfig {
            safety_fund_fee_share: Some(safety_fund_fee_share),
            treasury_fee_share: Some(treasury_fee_share),
            ..base_config.clone()
        };
        let exceeding_fees_msg = InstantiateMsg { config };
        let response =
            instantiate(deps.as_mut(), mock_env(), info.clone(), exceeding_fees_msg).unwrap_err();
        assert_eq!(response, ConfigError::InvalidFeeShareAmounts {}.into());

        // *
        // init config with valid params
        // *
        safety_fund_fee_share = Decimal::from_ratio(5u128, 10u128);
        treasury_fee_share = Decimal::from_ratio(3u128, 10u128);
        let config = CreateOrUpdateConfig {
            safety_fund_fee_share: Some(safety_fund_fee_share),
            treasury_fee_share: Some(treasury_fee_share),
            ..base_config
        };
        let msg = InstantiateMsg { config };

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: Config = from_binary(&res).unwrap();
        assert_eq!(value.owner, "owner");
        assert_eq!(value.address_provider_address, "address_provider");
        assert_eq!(value.safety_fund_fee_share, safety_fund_fee_share);
        assert_eq!(value.treasury_fee_share, treasury_fee_share);
        assert_eq!(value.astroport_factory_address, "astroport");
        assert_eq!(value.astroport_max_spread, astroport_max_spread);
    }

    #[test]
    fn test_update_config() {
        let mut deps = th_setup(&[]);

        let mut safety_fund_fee_share = Decimal::percent(10);
        let mut treasury_fee_share = Decimal::percent(20);
        let mut astroport_max_spread = StdDecimal::percent(1);
        let base_config = CreateOrUpdateConfig {
            owner: Some("owner".to_string()),
            address_provider_address: Some("address_provider".to_string()),
            safety_fund_fee_share: Some(safety_fund_fee_share),
            treasury_fee_share: Some(treasury_fee_share),
            astroport_factory_address: Some("astroport".to_string()),
            astroport_max_spread: Some(astroport_max_spread),
        };

        // *
        // non owner is not authorized
        // *
        let msg = ExecuteMsg::UpdateConfig {
            config: base_config.clone(),
        };
        let info = mock_info("somebody");
        let error_res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(error_res, MarsError::Unauthorized {}.into());

        // *
        // update config with safety_fund_fee_share, treasury_fee_share greater than 1
        // *
        safety_fund_fee_share = Decimal::from_ratio(11u128, 10u128);
        treasury_fee_share = Decimal::from_ratio(12u128, 10u128);
        let config = CreateOrUpdateConfig {
            owner: None,
            safety_fund_fee_share: Some(safety_fund_fee_share),
            treasury_fee_share: Some(treasury_fee_share),
            ..base_config.clone()
        };
        let msg = ExecuteMsg::UpdateConfig { config };
        let info = mock_info("owner");
        let error_res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
        assert_eq!(
            error_res,
            ConfigError::Mars(MarsError::ParamsNotLessOrEqualOne {
                expected_params: "safety_fund_fee_share, treasury_fee_share".to_string(),
                invalid_params: "safety_fund_fee_share, treasury_fee_share".to_string()
            })
            .into()
        );

        // *
        // update config with invalid fee share amounts
        // *
        safety_fund_fee_share = Decimal::from_ratio(10u128, 10u128);
        let config = CreateOrUpdateConfig {
            owner: None,
            safety_fund_fee_share: Some(safety_fund_fee_share),
            treasury_fee_share: None,
            ..base_config
        };
        let exceeding_fees_msg = ExecuteMsg::UpdateConfig { config };
        let error_res =
            execute(deps.as_mut(), mock_env(), info.clone(), exceeding_fees_msg).unwrap_err();
        assert_eq!(error_res, ConfigError::InvalidFeeShareAmounts {}.into());

        // *
        // update config with all new params
        // *
        safety_fund_fee_share = Decimal::from_ratio(5u128, 100u128);
        treasury_fee_share = Decimal::from_ratio(3u128, 100u128);
        astroport_max_spread = StdDecimal::percent(2);
        let config = CreateOrUpdateConfig {
            owner: Some("new_owner".to_string()),
            address_provider_address: Some("new_address_provider".to_string()),
            safety_fund_fee_share: Some(safety_fund_fee_share),
            treasury_fee_share: Some(treasury_fee_share),
            astroport_factory_address: Some("new_astroport".to_string()),
            astroport_max_spread: Some(astroport_max_spread),
        };
        let msg = ExecuteMsg::UpdateConfig {
            config: config.clone(),
        };
        // we can just call .unwrap() to assert this was a success
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Read config from state
        let new_config = CONFIG.load(&deps.storage).unwrap();

        assert_eq!(new_config.owner, config.owner.unwrap());
        assert_eq!(
            new_config.address_provider_address,
            config.address_provider_address.unwrap()
        );
        assert_eq!(
            new_config.safety_fund_fee_share,
            config.safety_fund_fee_share.unwrap()
        );
        assert_eq!(
            new_config.treasury_fee_share,
            config.treasury_fee_share.unwrap()
        );
        assert_eq!(
            new_config.astroport_factory_address,
            config.astroport_factory_address.unwrap()
        );
        assert_eq!(
            new_config.astroport_max_spread,
            config.astroport_max_spread.unwrap()
        );
    }

    #[test]
    fn test_update_asset_config() {
        let mut deps = th_setup(&[]);

        // asset config with valid params
        let asset = Asset::Native {
            denom: "somecoin".to_string(),
        };
        let msg = ExecuteMsg::UpdateAssetConfig {
            asset: asset.clone(),
            enabled: true,
        };

        // *
        // non owner is not authorized
        // *
        let info = mock_info("somebody");
        let error_res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
        assert_eq!(error_res, MarsError::Unauthorized {}.into());

        // *
        // owner can create asset config
        // *
        let info = mock_info("owner");
        // we can just call .unwrap() to assert this was a success
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "update_asset_config"),
                attr("asset", "somecoin"),
            ]
        );

        // *
        // query asset config
        // *
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::AssetConfig {
                asset: asset.clone(),
            },
        )
        .unwrap();
        let value: AssetConfig = from_binary(&res).unwrap();
        assert_eq!(value.enabled_for_distribution, true);

        // *
        // owner can update asset config
        // *
        let msg = ExecuteMsg::UpdateAssetConfig {
            asset: asset.clone(),
            enabled: false,
        };
        // we can just call .unwrap() to assert this was a success
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "update_asset_config"),
                attr("asset", "somecoin"),
            ]
        );

        let reference = asset.get_reference();
        let value = ASSET_CONFIG
            .load(deps.as_ref().storage, reference.as_slice())
            .unwrap();
        assert_eq!(value.enabled_for_distribution, false);

        // *
        // unknown assets are not enabled
        // *
        let asset = Asset::Native {
            denom: "uluna".to_string(),
        };
        let reference = asset.get_reference();

        // no asset config stored for unknown assets
        let err = ASSET_CONFIG
            .load(deps.as_ref().storage, reference.as_slice())
            .unwrap_err();
        assert_eq!(
            err,
            StdError::not_found("mars_core::protocol_rewards_collector::AssetConfig")
        );

        // querying unknown assets returns that they are not enabled
        let res = query(deps.as_ref(), mock_env(), QueryMsg::AssetConfig { asset }).unwrap();
        let value: AssetConfig = from_binary(&res).unwrap();
        assert_eq!(value.enabled_for_distribution, false);
    }

    #[test]
    fn test_execute_withdraw_from_red_bank() {
        let mut deps = th_setup(&[]);

        // *
        // anyone can execute a withdrawal
        // *
        let asset = Asset::Native {
            denom: "somecoin".to_string(),
        };
        let amount = Uint128::new(123_456);
        let msg = ExecuteMsg::WithdrawFromRedBank {
            asset: asset.clone(),
            amount: Some(amount),
        };
        let info = mock_info("anybody");
        // we can just call .unwrap() to assert this was a success
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "red_bank".to_string(),
                msg: to_binary(&red_bank::msg::ExecuteMsg::Withdraw {
                    asset: asset.clone(),
                    amount: Some(amount)
                })
                .unwrap(),
                funds: vec![]
            }))]
        );
        assert_eq!(
            res.attributes,
            vec![attr("action", "withdraw_from_red_bank"),]
        );
    }

    #[test]
    fn test_distribute_protocol_rewards_native() {
        let balance = 2_000_000_000u128;
        let asset = Asset::Native {
            denom: "somecoin".to_string(),
        };

        // initialize contract with balance
        let mut deps = th_setup(&[coin(balance, "somecoin")]);

        // Set tax data
        deps.querier.set_native_tax(
            Decimal::from_ratio(1u128, 100u128),
            &[(String::from("somecoin"), Uint128::new(100u128))],
        );

        // call function on an asset that isn't enabled
        let permissible_amount = Uint128::new(1_500_000_000);
        let msg = ExecuteMsg::DistributeProtocolRewards {
            asset: asset.clone(),
            amount: Some(permissible_amount),
        };
        let info = mock_info("anybody");
        let error_res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::AssetNotEnabled {
                asset_label: "somecoin".to_string()
            }
        );

        // enable the asset
        let msg = ExecuteMsg::UpdateAssetConfig {
            asset: asset.clone(),
            enabled: true,
        };
        let info = mock_info("owner");
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // call function providing amount exceeding balance
        let exceeding_amount = Uint128::new(2_000_000_001);
        let msg = ExecuteMsg::DistributeProtocolRewards {
            asset: asset.clone(),
            amount: Some(exceeding_amount),
        };
        let info = mock_info("anybody");
        let error_res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::AmountToDistributeTooLarge {
                amount: exceeding_amount,
                balance: Uint128::new(balance)
            }
        );

        // call function providing an amount less than the balance
        let permissible_amount = Uint128::new(1_500_000_000);
        let msg = ExecuteMsg::DistributeProtocolRewards {
            asset: asset.clone(),
            amount: Some(permissible_amount),
        };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let config = CONFIG.load(&deps.storage).unwrap();
        let expected_safety_fund_amount = permissible_amount * config.safety_fund_fee_share;
        let expected_treasury_amount = permissible_amount * config.treasury_fee_share;
        let expected_staking_amount =
            permissible_amount - (expected_safety_fund_amount + expected_treasury_amount);

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: "safety_fund".to_string(),
                    amount: vec![deduct_tax(
                        deps.as_ref(),
                        Coin {
                            denom: "somecoin".to_string(),
                            amount: expected_safety_fund_amount.into(),
                        }
                    )
                    .unwrap()],
                })),
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: "treasury".to_string(),
                    amount: vec![deduct_tax(
                        deps.as_ref(),
                        Coin {
                            denom: "somecoin".to_string(),
                            amount: expected_treasury_amount.into(),
                        }
                    )
                    .unwrap()],
                })),
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: "staking".to_string(),
                    amount: vec![deduct_tax(
                        deps.as_ref(),
                        Coin {
                            denom: "somecoin".to_string(),
                            amount: expected_staking_amount.into(),
                        }
                    )
                    .unwrap()],
                }))
            ]
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "distribute_protocol_income"),
                attr("asset", "somecoin"),
                attr("total_distributed_amount", permissible_amount),
                attr("safety_fund_amount", expected_safety_fund_amount),
                attr("treasury_amount", expected_treasury_amount),
                attr("staking_amount", expected_staking_amount),
            ]
        );

        // call function without providing an amount
        let msg = ExecuteMsg::DistributeProtocolRewards {
            asset: asset.clone(),
            amount: None,
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // verify messages are correct
        let expected_rewards_to_be_distributed = Uint128::new(balance);
        let expected_safety_fund_amount =
            expected_rewards_to_be_distributed * config.safety_fund_fee_share;
        let expected_treasury_amount =
            expected_rewards_to_be_distributed * config.treasury_fee_share;
        let expected_staking_amount = expected_rewards_to_be_distributed
            - (expected_safety_fund_amount + expected_treasury_amount);

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: "safety_fund".to_string(),
                    amount: vec![deduct_tax(
                        deps.as_ref(),
                        Coin {
                            denom: "somecoin".to_string(),
                            amount: expected_safety_fund_amount.into(),
                        }
                    )
                    .unwrap()],
                })),
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: "treasury".to_string(),
                    amount: vec![deduct_tax(
                        deps.as_ref(),
                        Coin {
                            denom: "somecoin".to_string(),
                            amount: expected_treasury_amount.into(),
                        }
                    )
                    .unwrap()],
                })),
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: "staking".to_string(),
                    amount: vec![deduct_tax(
                        deps.as_ref(),
                        Coin {
                            denom: "somecoin".to_string(),
                            amount: expected_staking_amount.into(),
                        }
                    )
                    .unwrap()],
                }))
            ]
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "distribute_protocol_income"),
                attr("asset", "somecoin"),
                attr(
                    "total_distributed_amount",
                    expected_rewards_to_be_distributed
                ),
                attr("safety_fund_amount", expected_safety_fund_amount),
                attr("treasury_amount", expected_treasury_amount),
                attr("staking_amount", expected_staking_amount),
            ]
        );
    }

    #[test]
    fn test_distribute_protocol_rewards_cw20() {
        let mut deps = th_setup(&[]);

        // initialize contract with balance
        let balance = 2_000_000_000u128;
        deps.querier.set_cw20_balances(
            Addr::unchecked("cw20_address"),
            &[(Addr::unchecked(MOCK_CONTRACT_ADDR), balance.into())],
        );

        let asset = Asset::Cw20 {
            contract_addr: "cw20_address".to_string(),
        };

        // call function on an asset that isn't enabled
        let permissible_amount = Uint128::new(1_500_000_000);
        let msg = ExecuteMsg::DistributeProtocolRewards {
            asset: asset.clone(),
            amount: Some(permissible_amount),
        };
        let info = mock_info("anybody");
        let error_res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::AssetNotEnabled {
                asset_label: "cw20_address".to_string()
            }
        );

        // enable the asset
        let msg = ExecuteMsg::UpdateAssetConfig {
            asset: asset.clone(),
            enabled: true,
        };
        let info = mock_info("owner");
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // call function providing amount exceeding balance
        let exceeding_amount = Uint128::new(2_000_000_001);
        let msg = ExecuteMsg::DistributeProtocolRewards {
            asset: asset.clone(),
            amount: Some(exceeding_amount),
        };
        let info = mock_info("anybody");
        let error_res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();

        assert_eq!(
            error_res,
            ContractError::AmountToDistributeTooLarge {
                amount: exceeding_amount,
                balance: Uint128::new(balance)
            }
        );

        // call function providing an amount less than the balance
        let permissible_amount = Uint128::new(1_500_000_000);
        let msg = ExecuteMsg::DistributeProtocolRewards {
            asset: asset.clone(),
            amount: Some(permissible_amount),
        };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let config = CONFIG.load(&deps.storage).unwrap();
        let expected_safety_fund_amount = permissible_amount * config.safety_fund_fee_share;
        let expected_treasury_amount = permissible_amount * config.treasury_fee_share;
        let expected_staking_amount =
            permissible_amount - (expected_safety_fund_amount + expected_treasury_amount);

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "cw20_address".to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: "safety_fund".to_string(),
                        amount: expected_safety_fund_amount,
                    })
                    .unwrap(),
                    funds: vec![],
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "cw20_address".to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: "treasury".to_string(),
                        amount: expected_treasury_amount,
                    })
                    .unwrap(),
                    funds: vec![],
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "cw20_address".to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: "staking".to_string(),
                        amount: expected_staking_amount,
                    })
                    .unwrap(),
                    funds: vec![],
                })),
            ]
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "distribute_protocol_income"),
                attr("asset", "cw20_address"),
                attr("total_distributed_amount", permissible_amount),
                attr("safety_fund_amount", expected_safety_fund_amount),
                attr("treasury_amount", expected_treasury_amount),
                attr("staking_amount", expected_staking_amount),
            ]
        );

        // call function without providing an amount
        let msg = ExecuteMsg::DistributeProtocolRewards {
            asset: asset.clone(),
            amount: None,
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // verify messages are correct
        let expected_rewards_to_be_distributed = Uint128::new(balance);
        let expected_safety_fund_amount =
            expected_rewards_to_be_distributed * config.safety_fund_fee_share;
        let expected_treasury_amount =
            expected_rewards_to_be_distributed * config.treasury_fee_share;
        let expected_staking_amount = expected_rewards_to_be_distributed
            - (expected_safety_fund_amount + expected_treasury_amount);

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "cw20_address".to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: "safety_fund".to_string(),
                        amount: expected_safety_fund_amount,
                    })
                    .unwrap(),
                    funds: vec![],
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "cw20_address".to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: "treasury".to_string(),
                        amount: expected_treasury_amount,
                    })
                    .unwrap(),
                    funds: vec![],
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "cw20_address".to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: "staking".to_string(),
                        amount: expected_staking_amount,
                    })
                    .unwrap(),
                    funds: vec![],
                })),
            ]
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "distribute_protocol_income"),
                attr("asset", "cw20_address"),
                attr(
                    "total_distributed_amount",
                    expected_rewards_to_be_distributed
                ),
                attr("safety_fund_amount", expected_safety_fund_amount),
                attr("treasury_amount", expected_treasury_amount),
                attr("staking_amount", expected_staking_amount),
            ]
        );
    }

    #[test]
    fn test_execute_cosmos_msg() {
        let mut deps = th_setup(&[]);

        let bank = BankMsg::Send {
            to_address: "destination".to_string(),
            amount: vec![Coin {
                denom: "uluna".to_string(),
                amount: Uint128::new(123456),
            }],
        };
        let cosmos_msg = CosmosMsg::Bank(bank);
        let msg = ExecuteMsg::ExecuteCosmosMsg(cosmos_msg.clone());

        // *
        // non owner is not authorized
        // *
        let info = mock_info("somebody");
        let error_res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
        assert_eq!(error_res, MarsError::Unauthorized {}.into());

        // *
        // can execute Cosmos msg
        // *
        let info = mock_info("owner");
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages, vec![SubMsg::new(cosmos_msg)]);
        assert_eq!(res.attributes, vec![attr("action", "execute_cosmos_msg")]);
    }

    // TEST HELPERS

    fn th_setup(contract_balances: &[Coin]) -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier> {
        let mut deps = mock_dependencies(contract_balances);
        let info = mock_info("owner");
        let config = CreateOrUpdateConfig {
            owner: Some("owner".to_string()),
            address_provider_address: Some("address_provider".to_string()),
            safety_fund_fee_share: Some(Decimal::percent(10)),
            treasury_fee_share: Some(Decimal::percent(20)),
            astroport_factory_address: Some("astroport".to_string()),
            astroport_max_spread: Some(StdDecimal::percent(1)),
        };
        let msg = InstantiateMsg { config };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        deps
    }
}
