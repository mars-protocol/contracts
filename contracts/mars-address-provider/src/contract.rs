#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use crate::error::ContractError;
use crate::msg::{ConfigParams, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::CONFIG;
use crate::{Config, MarsContract};

use mars_core::helpers::option_string_to_addr;

// INIT

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    // Initialize config
    let config = Config {
        owner: deps.api.addr_validate(&msg.owner)?,
        council_address: Addr::unchecked(""),
        incentives_address: Addr::unchecked(""),
        safety_fund_address: Addr::unchecked(""),
        mars_token_address: Addr::unchecked(""),
        oracle_address: Addr::unchecked(""),
        protocol_admin_address: Addr::unchecked(""),
        protocol_rewards_collector_address: Addr::unchecked(""),
        red_bank_address: Addr::unchecked(""),
        staking_address: Addr::unchecked(""),
        treasury_address: Addr::unchecked(""),
        vesting_address: Addr::unchecked(""),
        xmars_token_address: Addr::unchecked(""),
    };

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
        ExecuteMsg::UpdateConfig {
            config: config_params,
        } => execute_update_config(deps, env, info, config_params),
    }
}

/// Update config
pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    config_params: ConfigParams,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    let ConfigParams {
        owner,
        council_address,
        incentives_address,
        safety_fund_address,
        mars_token_address,
        oracle_address,
        protocol_admin_address,
        protocol_rewards_collector_address,
        red_bank_address,
        staking_address,
        treasury_address,
        vesting_address,
        xmars_token_address,
    } = config_params;

    // Update config
    config.owner = option_string_to_addr(deps.api, owner, config.owner)?;
    config.council_address =
        option_string_to_addr(deps.api, council_address, config.council_address)?;
    config.incentives_address =
        option_string_to_addr(deps.api, incentives_address, config.incentives_address)?;
    config.safety_fund_address =
        option_string_to_addr(deps.api, safety_fund_address, config.safety_fund_address)?;
    config.mars_token_address =
        option_string_to_addr(deps.api, mars_token_address, config.mars_token_address)?;
    config.oracle_address = option_string_to_addr(deps.api, oracle_address, config.oracle_address)?;
    config.protocol_admin_address = option_string_to_addr(
        deps.api,
        protocol_admin_address,
        config.protocol_admin_address,
    )?;
    config.protocol_rewards_collector_address = option_string_to_addr(
        deps.api,
        protocol_rewards_collector_address,
        config.protocol_rewards_collector_address,
    )?;
    config.red_bank_address =
        option_string_to_addr(deps.api, red_bank_address, config.red_bank_address)?;
    config.staking_address =
        option_string_to_addr(deps.api, staking_address, config.staking_address)?;
    config.treasury_address =
        option_string_to_addr(deps.api, treasury_address, config.treasury_address)?;
    config.vesting_address =
        option_string_to_addr(deps.api, vesting_address, config.vesting_address)?;
    config.xmars_token_address =
        option_string_to_addr(deps.api, xmars_token_address, config.xmars_token_address)?;

    CONFIG.save(deps.storage, &config)?;

    let res = Response::new().add_attribute("action", "update_config");
    Ok(res)
}

// QUERIES

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Address { contract } => to_binary(&query_address(deps, contract)?),
        QueryMsg::Addresses { contracts } => to_binary(&query_addresses(deps, contracts)?),
    }
}

fn query_config(deps: Deps) -> StdResult<Config> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}

fn query_address(deps: Deps, contract: MarsContract) -> StdResult<Addr> {
    let config = CONFIG.load(deps.storage)?;
    Ok(get_address(&config, contract))
}

fn query_addresses(deps: Deps, contracts: Vec<MarsContract>) -> StdResult<Vec<Addr>> {
    let config = CONFIG.load(deps.storage)?;
    let mut ret: Vec<Addr> = Vec::with_capacity(contracts.len());
    for contract in contracts {
        ret.push(get_address(&config, contract));
    }

    Ok(ret)
}

fn get_address(config: &Config, address: MarsContract) -> Addr {
    match address {
        MarsContract::Council => config.council_address.clone(),
        MarsContract::Incentives => config.incentives_address.clone(),
        MarsContract::SafetyFund => config.safety_fund_address.clone(),
        MarsContract::MarsToken => config.mars_token_address.clone(),
        MarsContract::Oracle => config.oracle_address.clone(),
        MarsContract::ProtocolAdmin => config.protocol_admin_address.clone(),
        MarsContract::ProtocolRewardsCollector => config.protocol_rewards_collector_address.clone(),
        MarsContract::RedBank => config.red_bank_address.clone(),
        MarsContract::Staking => config.staking_address.clone(),
        MarsContract::Treasury => config.treasury_address.clone(),
        MarsContract::Vesting => config.vesting_address.clone(),
        MarsContract::XMarsToken => config.xmars_token_address.clone(),
    }
}

// TESTS

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage};
    use cosmwasm_std::{from_binary, Coin, OwnedDeps};

    #[test]
    fn test_proper_initialization() {
        let mut deps = mock_dependencies(&[]);
        let owner_address = Addr::unchecked("owner");

        // *
        // init config with empty params
        // *
        let msg = InstantiateMsg {
            owner: "owner".to_string(),
        };
        let info = MessageInfo {
            sender: Addr::unchecked("whoever"),
            funds: vec![],
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let config = CONFIG.load(&deps.storage).unwrap();
        assert_eq!(owner_address, config.owner);
    }

    #[test]
    fn test_update_config() {
        let mut deps = th_setup(&[]);
        // *
        // non owner is not authorized
        // *
        {
            let msg = ExecuteMsg::UpdateConfig {
                config: ConfigParams::default(),
            };
            let info = MessageInfo {
                sender: Addr::unchecked("somebody"),
                funds: vec![],
            };
            let error_res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
            assert_eq!(error_res, ContractError::Unauthorized {});
        }

        // *
        // update config
        // *
        {
            let msg = ExecuteMsg::UpdateConfig {
                config: ConfigParams {
                    incentives_address: Some("incentives".to_string()),
                    mars_token_address: Some("mars-token".to_string()),
                    treasury_address: Some("treasury".to_string()),
                    ..Default::default()
                },
            };
            let info = MessageInfo {
                sender: Addr::unchecked("owner"),
                funds: vec![],
            };

            let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
            assert_eq!(0, res.messages.len());

            // Read config from state
            let new_config = CONFIG.load(&deps.storage).unwrap();

            assert_eq!(new_config.owner, Addr::unchecked("owner"));
            assert_eq!(new_config.xmars_token_address, Addr::unchecked(""),);
            assert_eq!(new_config.incentives_address, Addr::unchecked("incentives"));
            assert_eq!(new_config.mars_token_address, Addr::unchecked("mars-token"));
            assert_eq!(new_config.treasury_address, Addr::unchecked("treasury"));
        }
    }

    #[test]
    fn test_address_queries() {
        let mut deps = th_setup(&[]);
        let env = mock_env();

        let council_address = Addr::unchecked("council");
        let incentives_address = Addr::unchecked("incentives");
        let xmars_token_address = Addr::unchecked("xmars_token");

        CONFIG
            .update(&mut deps.storage, |mut c| -> StdResult<_> {
                c.council_address = council_address.clone();
                c.incentives_address = incentives_address.clone();
                c.xmars_token_address = xmars_token_address.clone();
                Ok(c)
            })
            .unwrap();

        {
            let address_query = query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::Address {
                    contract: MarsContract::Incentives,
                },
            )
            .unwrap();
            let result: Addr = from_binary(&address_query).unwrap();
            assert_eq!(result, incentives_address);
        }

        {
            let addresses_query = query(
                deps.as_ref(),
                env,
                QueryMsg::Addresses {
                    contracts: vec![MarsContract::XMarsToken, MarsContract::Council],
                },
            )
            .unwrap();
            let result: Vec<Addr> = from_binary(&addresses_query).unwrap();
            assert_eq!(result[0], xmars_token_address);
            assert_eq!(result[1], council_address);
        }
    }

    // TEST HELPERS
    fn th_setup(contract_balances: &[Coin]) -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies(contract_balances);
        let msg = InstantiateMsg {
            owner: "owner".to_string(),
        };
        let info = MessageInfo {
            sender: Addr::unchecked("someone"),
            funds: vec![],
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        deps
    }
}
