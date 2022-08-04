#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, Fraction, IbcMsg, IbcTimeout,
    IbcTimeoutBlock, MessageInfo, Response, StdError, StdResult, Uint128, WasmMsg,
};

use mars_outpost::asset::{get_asset_balance, Asset};
use mars_outpost::error::MarsError;
use mars_outpost::helpers::{option_string_to_addr, zero_address};

use mars_outpost::address_provider::{self, helpers, MarsContract};
use mars_outpost::red_bank;
use osmo_bindings::{OsmosisMsg, Step};

use crate::error::ContractError;
use crate::msg::{CreateOrUpdateConfig, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::CONFIG;
use crate::swap::construct_swap_msg;
use crate::Config;

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
        safety_tax_rate,
        safety_fund_asset,
        fee_collector_asset,
        channel_id,
        revision,
        block_timeout,
    } = msg.config;

    // All fields should be available
    let available = owner.is_some()
        && address_provider_address.is_some()
        && safety_tax_rate.is_some()
        && safety_fund_asset.is_some()
        && fee_collector_asset.is_some()
        && channel_id.is_some()
        && revision.is_some()
        && block_timeout.is_some();

    if !available {
        return Err(MarsError::InstantiateParamsUnavailable {}.into());
    };

    let config = Config {
        owner: option_string_to_addr(deps.api, owner, zero_address())?,
        safety_tax_rate: safety_tax_rate.unwrap(),
        safety_fund_asset: safety_fund_asset.unwrap(),
        fee_collector_asset: fee_collector_asset.unwrap(),
        address_provider_address: option_string_to_addr(
            deps.api,
            address_provider_address,
            zero_address(),
        )?,
        channel_id: channel_id.unwrap(),
        revision: revision.unwrap(),
        block_timeout: block_timeout.unwrap(),
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
) -> Result<Response<OsmosisMsg>, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {
            config,
        } => execute_update_config(deps, env, info, config),
        ExecuteMsg::WithdrawFromRedBank {
            asset,
            amount,
        } => execute_withdraw_from_red_bank(deps, env, info, asset, amount),
        ExecuteMsg::DistributeProtocolRewards {
            asset,
            amount,
        } => execute_distribute_protocol_rewards(deps, env, info, asset, amount),
        ExecuteMsg::SwapAsset {
            asset_in,
            amount,
            safety_fund_asset_steps,
            fee_collector_asset_steps,
        } => Ok(execute_swap_asset(
            deps,
            env,
            asset_in,
            amount,
            &safety_fund_asset_steps,
            &fee_collector_asset_steps,
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
) -> Result<Response<OsmosisMsg>, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(MarsError::Unauthorized {}.into());
    }

    // Destructuring a struct’s fields into separate variables in order to force
    // compile error if we add more params
    let CreateOrUpdateConfig {
        owner,
        address_provider_address,
        safety_tax_rate,
        safety_fund_asset,
        fee_collector_asset,
        channel_id,
        revision,
        block_timeout,
    } = new_config;

    config.owner = option_string_to_addr(deps.api, owner, config.owner)?;
    config.address_provider_address =
        option_string_to_addr(deps.api, address_provider_address, config.address_provider_address)?;
    config.safety_tax_rate = safety_tax_rate.unwrap_or(config.safety_tax_rate);
    config.safety_fund_asset = safety_fund_asset.unwrap_or(config.safety_fund_asset);
    config.fee_collector_asset = fee_collector_asset.unwrap_or(config.fee_collector_asset);
    config.channel_id = channel_id.unwrap_or(config.channel_id);
    config.revision = revision.unwrap_or(config.revision);
    config.block_timeout = block_timeout.unwrap_or(config.block_timeout);

    config.validate()?;

    CONFIG.save(deps.storage, &config)?;

    let res = Response::new().add_attribute("action", "update_config");
    Ok(res)
}

pub fn execute_withdraw_from_red_bank(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    asset: Asset,
    amount: Option<Uint128>,
) -> Result<Response<OsmosisMsg>, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let red_bank_address = address_provider::helpers::query_address(
        deps.as_ref(),
        &config.address_provider_address,
        MarsContract::RedBank,
    )?;

    let withdraw_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: red_bank_address.to_string(),
        msg: to_binary(&red_bank::msg::ExecuteMsg::Withdraw {
            asset,
            amount,
            recipient: None,
        })?,
        funds: vec![],
    });

    let res =
        Response::new().add_attribute("action", "withdraw_from_red_bank").add_message(withdraw_msg);

    Ok(res)
}

/// Send accumulated asset rewards to protocol contracts
pub fn execute_distribute_protocol_rewards(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    asset: Asset,
    amount: Option<Uint128>,
) -> Result<Response<OsmosisMsg>, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let (asset_denom, _, asset_type) = asset.get_attributes();

    let to_address = if asset == config.safety_fund_asset {
        helpers::query_address(
            deps.as_ref(),
            &config.address_provider_address,
            MarsContract::SafetyFund,
        )?
    } else if asset == config.fee_collector_asset {
        helpers::query_address(
            deps.as_ref(),
            &config.address_provider_address,
            MarsContract::FeeCollector,
        )?
    } else {
        return Err(ContractError::AssetNotEnabledForDistribution {
            asset_label: asset_denom,
        });
    };

    let balance =
        get_asset_balance(deps.as_ref(), env.contract.address, asset_denom.clone(), asset_type)?;

    let amount_to_distribute = match amount {
        Some(amount) if amount > balance => {
            return Err(ContractError::AmountToDistributeTooLarge {
                amount,
                balance,
            })
        }
        Some(amount) => amount,
        None => balance,
    };

    let msg = CosmosMsg::Ibc(IbcMsg::Transfer {
        channel_id: config.channel_id,
        to_address: to_address.into_string(),
        amount: Coin {
            denom: asset_denom.clone(),
            amount: amount_to_distribute,
        },
        timeout: IbcTimeout::with_block(IbcTimeoutBlock {
            revision: config.revision,
            height: env.block.height + config.block_timeout,
        }),
    });

    let res = Response::new()
        .add_attribute("action", "distribute_protocol_income")
        .add_attribute("asset", asset_denom)
        .add_attribute("amount_to_distribute", amount_to_distribute)
        .add_message(msg);

    Ok(res)
}

/// Swap any asset on the contract
pub fn execute_swap_asset(
    deps: DepsMut,
    env: Env,
    asset_in: Asset,
    amount: Option<Uint128>,
    safety_fund_asset_steps: &[Step],
    fee_collector_asset_steps: &[Step],
) -> StdResult<Response<OsmosisMsg>> {
    let config = CONFIG.load(deps.storage)?;
    let (denom_in, _, asset_type) = asset_in.get_attributes();

    // if amount is None, swap the total balance of asset_in
    let amount_to_swap = match amount {
        Some(swap_amount) => swap_amount,
        None => get_asset_balance(
            deps.as_ref(),
            env.contract.address.clone(),
            denom_in.clone(),
            asset_type,
        )?,
    };

    // split the amount to swap between the safety fund and the fee collector
    // swap the safety fund share to safety_fund_asset, and the fee collector
    // share to fee_collector asset
    let safety_fund_share = amount_to_swap
        .checked_multiply_ratio(
            config.safety_tax_rate.numerator(),
            config.safety_tax_rate.denominator(),
        )
        .map_err(|err| StdError::generic_err(err.to_string()))?;
    let fee_collector_share = amount_to_swap
        .checked_sub(safety_fund_share)
        .map_err(|err| StdError::generic_err(err.to_string()))?;

    let mut messages = vec![];
    if !safety_fund_share.is_zero() {
        if let Ok(msg) = construct_swap_msg(
            deps.as_ref(),
            env.clone(),
            &denom_in,
            safety_fund_share,
            safety_fund_asset_steps,
        ) {
            messages.push(msg);
        }
    }

    if !fee_collector_share.is_zero() {
        if let Ok(msg) = construct_swap_msg(
            deps.as_ref(),
            env,
            &denom_in,
            fee_collector_share,
            fee_collector_asset_steps,
        ) {
            messages.push(msg);
        }
    }

    let response = Response::new()
        .add_attributes(vec![
            attr("action", "swap"),
            attr("denom_in", denom_in),
            attr("amount_to_swap", amount_to_swap),
            attr("safety_fund_share", safety_fund_share),
            attr("fee_collector_share", fee_collector_share),
        ])
        .add_messages(messages);

    Ok(response)
}

/// Execute Cosmos message
pub fn execute_execute_cosmos_msg(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: CosmosMsg<OsmosisMsg>,
) -> Result<Response<OsmosisMsg>, MarsError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(MarsError::Unauthorized {});
    }

    let response = Response::new().add_attribute("action", "execute_cosmos_msg").add_message(msg);

    Ok(response)
}

// QUERIES

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<Config> {
    let config = CONFIG.load(deps.storage)?;

    Ok(config)
}

// TESTS

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::{
        attr, coin, coins, from_binary,
        testing::{mock_env, MockApi, MockStorage},
        BankMsg, Coin, Decimal, OwnedDeps, SubMsg,
    };

    use mars_outpost::testing::{mock_dependencies, mock_info, MarsMockQuerier};
    use osmo_bindings::Swap;

    use crate::ConfigError;

    #[test]
    fn test_proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        // Config with base params valid (just update the rest)
        let base_config = CreateOrUpdateConfig {
            owner: Some("owner".to_string()),
            address_provider_address: Some("address_provider".to_string()),
            safety_tax_rate: Some(Decimal::from_ratio(5u128, 10u128)),
            safety_fund_asset: Some(Asset::Native {
                denom: "uusdc".to_string(),
            }),
            fee_collector_asset: Some(Asset::Native {
                denom: "umars".to_string(),
            }),
            channel_id: Some("channel-110".to_string()),
            revision: Some(1),
            block_timeout: Some(50),
        };

        let info = mock_info("owner");

        // *
        // init config with empty params
        // *
        let empty_config = CreateOrUpdateConfig {
            owner: None,
            address_provider_address: None,
            safety_tax_rate: None,
            safety_fund_asset: None,
            fee_collector_asset: None,
            channel_id: None,
            revision: None,
            block_timeout: None,
        };
        let msg = InstantiateMsg {
            config: empty_config,
        };
        let err = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
        assert_eq!(err, MarsError::InstantiateParamsUnavailable {}.into());

        // *
        // init config with safety_tax_rate greater than 1
        // *
        let mut safety_tax_rate = Decimal::from_ratio(11u128, 10u128);
        let config = CreateOrUpdateConfig {
            safety_tax_rate: Some(safety_tax_rate),
            ..base_config.clone()
        };
        let msg = InstantiateMsg {
            config,
        };
        let response = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
        assert_eq!(
            response,
            ConfigError::Mars(MarsError::InvalidParam {
                param_name: "safety_tax_rate".to_string(),
                invalid_value: safety_tax_rate.to_string(),
                predicate: "<= 1".to_string(),
            })
            .into()
        );

        // *
        // init config with valid params
        // *
        safety_tax_rate = Decimal::from_ratio(5u128, 10u128);
        let config = CreateOrUpdateConfig {
            safety_tax_rate: Some(safety_tax_rate),
            ..base_config
        };
        let msg = InstantiateMsg {
            config,
        };

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: Config = from_binary(&res).unwrap();
        assert_eq!(value.owner, "owner");
        assert_eq!(value.address_provider_address, "address_provider");
        assert_eq!(value.safety_tax_rate, safety_tax_rate);
        assert_eq!(
            value.safety_fund_asset,
            Asset::Native {
                denom: "uusdc".to_string()
            }
        );
        assert_eq!(
            value.fee_collector_asset,
            Asset::Native {
                denom: "umars".to_string()
            }
        );
    }

    #[test]
    fn test_update_config() {
        let mut deps = th_setup(&[]);

        let mut safety_tax_rate = Decimal::percent(10);
        let base_config = CreateOrUpdateConfig {
            owner: Some("owner".to_string()),
            address_provider_address: Some("address_provider".to_string()),
            safety_tax_rate: Some(safety_tax_rate),
            safety_fund_asset: Some(Asset::Native {
                denom: "uusdc".to_string(),
            }),
            fee_collector_asset: Some(Asset::Native {
                denom: "umars".to_string(),
            }),
            channel_id: Some("channel-182".to_string()),
            revision: Some(1),
            block_timeout: Some(50),
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
        // update config with safety_tax_rate greater than 1
        // *
        let info = mock_info("owner");

        safety_tax_rate = Decimal::from_ratio(11u128, 10u128);
        let config = CreateOrUpdateConfig {
            owner: None,
            safety_tax_rate: Some(safety_tax_rate),
            ..base_config.clone()
        };
        let msg = ExecuteMsg::UpdateConfig {
            config,
        };
        let error_res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ConfigError::Mars(MarsError::InvalidParam {
                param_name: "safety_tax_rate".to_string(),
                invalid_value: safety_tax_rate.to_string(),
                predicate: "<= 1".to_string(),
            })
            .into()
        );

        // *
        // update config with safety_tax_rate greater than 1
        // *
        safety_tax_rate = Decimal::from_ratio(12u128, 10u128);
        let config = CreateOrUpdateConfig {
            owner: None,
            safety_tax_rate: Some(safety_tax_rate),
            ..base_config
        };
        let msg = ExecuteMsg::UpdateConfig {
            config,
        };
        let info = mock_info("owner");
        let error_res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
        assert_eq!(
            error_res,
            ConfigError::Mars(MarsError::InvalidParam {
                param_name: "safety_tax_rate".to_string(),
                invalid_value: safety_tax_rate.to_string(),
                predicate: "<= 1".to_string(),
            })
            .into()
        );

        // *
        // update config with all new params
        // *
        safety_tax_rate = Decimal::from_ratio(5u128, 100u128);
        let config = CreateOrUpdateConfig {
            owner: Some("new_owner".to_string()),
            address_provider_address: Some("new_address_provider".to_string()),
            safety_tax_rate: Some(safety_tax_rate),
            safety_fund_asset: Some(Asset::Native {
                denom: "uatom".to_string(),
            }),
            fee_collector_asset: Some(Asset::Native {
                denom: "uosmo".to_string(),
            }),
            channel_id: Some("channel-182".to_string()),
            revision: Some(1),
            block_timeout: Some(50),
        };
        let msg = ExecuteMsg::UpdateConfig {
            config: config.clone(),
        };
        // we can just call .unwrap() to assert this was a success
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Read config from state
        let new_config = CONFIG.load(&deps.storage).unwrap();

        assert_eq!(new_config.owner, config.owner.unwrap());
        assert_eq!(new_config.address_provider_address, config.address_provider_address.unwrap());
        assert_eq!(new_config.safety_tax_rate, config.safety_tax_rate.unwrap());
        assert_eq!(new_config.safety_fund_asset, config.safety_fund_asset.unwrap());
        assert_eq!(new_config.fee_collector_asset, config.fee_collector_asset.unwrap());
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
                    asset,
                    amount: Some(amount),
                    recipient: None
                })
                .unwrap(),
                funds: vec![]
            }))]
        );
        assert_eq!(res.attributes, vec![attr("action", "withdraw_from_red_bank"),]);
    }

    #[test]
    fn test_distribute_protocol_rewards() {
        let balance = 2_000_000_000u128;

        // initialize contract with balance
        let mut deps = th_setup(&[coin(balance, "uusdc"), coin(1_000_000, "umars")]);

        // call function on an asset that isn't enabled for distribution
        let permissible_amount = Uint128::new(1_500_000_000);
        let msg = ExecuteMsg::DistributeProtocolRewards {
            asset: Asset::Native {
                denom: "uosmo".to_string(),
            },
            amount: Some(permissible_amount),
        };
        let info = mock_info("anybody");
        let error_res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::AssetNotEnabledForDistribution {
                asset_label: "uosmo".to_string()
            }
        );

        // call function providing amount exceeding balance
        let exceeding_amount = Uint128::new(2_000_000_001);
        let msg = ExecuteMsg::DistributeProtocolRewards {
            asset: Asset::Native {
                denom: "uusdc".to_string(),
            },
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

        // call function providing an amount less than the balance, and distribute safety fund rewards ("uusdc")
        let permissible_amount = Uint128::new(1_500_000_000);
        let msg = ExecuteMsg::DistributeProtocolRewards {
            asset: Asset::Native {
                denom: "uusdc".to_string(),
            },
            amount: Some(permissible_amount),
        };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Ibc(IbcMsg::Transfer {
                channel_id: "channel-182".to_string(),
                to_address: "safety_fund".to_string(),
                amount: Coin {
                    denom: "uusdc".to_string(),
                    amount: permissible_amount
                },
                timeout: IbcTimeout::with_block(IbcTimeoutBlock {
                    revision: 1,
                    height: 12395,
                })
            }))]
        );

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "distribute_protocol_income"),
                attr("asset", "uusdc"),
                attr("amount_to_distribute", permissible_amount),
            ]
        );

        // call function without providing an amount, and distribute fee collector rewards ("umars")
        let msg = ExecuteMsg::DistributeProtocolRewards {
            asset: Asset::Native {
                denom: "umars".to_string(),
            },
            amount: None,
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Ibc(IbcMsg::Transfer {
                channel_id: "channel-182".to_string(),
                to_address: "fee_collector".to_string(),
                amount: Coin {
                    denom: "umars".to_string(),
                    amount: Uint128::new(1_000_000)
                },
                timeout: IbcTimeout::with_block(IbcTimeoutBlock {
                    revision: 1,
                    height: 12395,
                })
            }))]
        );

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "distribute_protocol_income"),
                attr("asset", "umars"),
                attr("amount_to_distribute", Uint128::new(1_000_000)),
            ]
        );
    }

    #[test]
    fn test_execute_swap_msg() {
        // initialize contract with balance
        let mut deps = th_setup(&coins(700_000, "uatom"));
        let info = mock_info("owner");

        let msg = ExecuteMsg::SwapAsset {
            asset_in: Asset::Native {
                denom: "uatom".to_string(),
            },
            amount: None,
            fee_collector_asset_steps: vec![
                Step {
                    pool_id: 1,
                    denom_out: "uosmo".to_string(),
                },
                Step {
                    pool_id: 3,
                    denom_out: "umars".to_string(),
                },
            ],
            safety_fund_asset_steps: vec![
                Step {
                    pool_id: 1,
                    denom_out: "uosmo".to_string(),
                },
                Step {
                    pool_id: 2,
                    denom_out: "uusdc".to_string(),
                },
            ],
        };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Custom(OsmosisMsg::Swap {
                    first: Swap {
                        pool_id: 1,
                        denom_in: "uatom".to_string(),
                        denom_out: "uosmo".to_string(),
                    },
                    route: vec![Step {
                        pool_id: 2,
                        denom_out: "uusdc".to_string(),
                    }],
                    amount: osmo_bindings::SwapAmountWithLimit::ExactIn {
                        input: Uint128::new(210_000),
                        min_output: Uint128::zero()
                    }
                })),
                SubMsg::new(CosmosMsg::Custom(OsmosisMsg::Swap {
                    first: Swap {
                        pool_id: 1,
                        denom_in: "uatom".to_string(),
                        denom_out: "uosmo".to_string(),
                    },
                    route: vec![Step {
                        pool_id: 3,
                        denom_out: "umars".to_string(),
                    }],
                    amount: osmo_bindings::SwapAmountWithLimit::ExactIn {
                        input: Uint128::new(490_000),
                        min_output: Uint128::zero()
                    }
                }))
            ]
        );

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "swap"),
                attr("denom_in", "uatom"),
                attr("amount_to_swap", "700000"),
                attr("safety_fund_share", "210000"),
                attr("fee_collector_share", "490000"),
            ]
        );

        // test swap all the amount to safety fund (safety fund tax rate = 1)
        let config = CreateOrUpdateConfig {
            owner: None,
            address_provider_address: None,
            fee_collector_asset: None,
            safety_fund_asset: None,
            channel_id: None,
            revision: None,
            block_timeout: None,
            safety_tax_rate: Some(Decimal::percent(100)),
        };
        let conf_msg = ExecuteMsg::UpdateConfig {
            config,
        };

        // change the safety_tax_rate to 1
        let _ = execute(deps.as_mut(), mock_env(), info.clone(), conf_msg).unwrap();

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Custom(OsmosisMsg::Swap {
                first: Swap {
                    pool_id: 1,
                    denom_in: "uatom".to_string(),
                    denom_out: "uosmo".to_string(),
                },
                route: vec![Step {
                    pool_id: 2,
                    denom_out: "uusdc".to_string(),
                }],
                amount: osmo_bindings::SwapAmountWithLimit::ExactIn {
                    input: Uint128::new(700_000),
                    min_output: Uint128::zero()
                }
            })),]
        );

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "swap"),
                attr("denom_in", "uatom"),
                attr("amount_to_swap", "700000"),
                attr("safety_fund_share", "700000"),
                attr("fee_collector_share", "0"),
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
            safety_tax_rate: Some(Decimal::percent(30)),
            safety_fund_asset: Some(Asset::Native {
                denom: "uusdc".to_string(),
            }),
            fee_collector_asset: Some(Asset::Native {
                denom: "umars".to_string(),
            }),
            channel_id: Some("channel-182".to_string()),
            revision: Some(1),
            block_timeout: Some(50),
        };
        let msg = InstantiateMsg {
            config,
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        deps
    }
}
