use std::cmp;
use std::str::FromStr;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, Event, MessageInfo,
    QueryRequest, Reply, Response, StdError, StdResult, SubMsg, Uint128, WasmMsg, WasmQuery,
};

use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use mars_core::address_provider::{self, MarsContract};
use mars_core::error::MarsError;
use mars_core::math::decimal::Decimal;
use mars_core::staking::msg as mars_staking;
use mars_core::vesting::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg};
use mars_core::vesting::{Allocation, Config};

use crate::error::ContractError;
use crate::state::{ALLOCATIONS, CONFIG, TEMP_DATA, VOTING_POWER_SNAPSHOTS};

// INSTANTIATE

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let timestamp = env.block.time.seconds();
    if msg.unlock_start_time > timestamp
        && msg.unlock_cliff > 0u64
        && msg.unlock_duration > msg.unlock_cliff
    {
        CONFIG.save(deps.storage, &msg.check(deps.api)?)?;
        Ok(Response::default())
    } else {
        Err(ContractError::InvalidUnlockTimeSetup {})
    }
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
        ExecuteMsg::Receive(cw20_msg) => receive_cw20(deps, env, info, cw20_msg),
        ExecuteMsg::Withdraw {} => execute_withdraw(deps, env, info),
    }
}

fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let api = deps.api;
    match from_binary(&cw20_msg.msg)? {
        ReceiveMsg::CreateAllocation { user_address } => execute_create_allocation(
            deps,
            env,
            info.sender,
            api.addr_validate(&cw20_msg.sender)?,
            api.addr_validate(&user_address)?,
            cw20_msg.amount,
        ),
    }
}

pub fn execute_create_allocation(
    deps: DepsMut,
    env: Env,
    token: Addr,
    creator: Addr,
    user_address: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let mut addresses_query = address_provider::helpers::query_addresses(
        &deps.querier,
        config.address_provider_address,
        vec![
            MarsContract::ProtocolAdmin,
            MarsContract::Staking,
            MarsContract::MarsToken,
        ],
    )?;
    let mars_token_address = addresses_query.pop().unwrap();
    let staking_address = addresses_query.pop().unwrap();
    let protocol_admin_address = addresses_query.pop().unwrap();

    // only Mars token can be used to create allocations
    if token != mars_token_address {
        return Err(ContractError::InvalidTokenDeposit {});
    }

    // only protocol admin can create allocations
    if creator != protocol_admin_address {
        return Err(MarsError::Unauthorized {}.into());
    }

    // save the staker's address as temporary data so that it can be accessed when handling reply
    TEMP_DATA.save(deps.storage, &user_address)?;

    // create submsg to stake deposited Mars tokens
    // reply will be handled by `after_staking`
    Ok(Response::new()
        .add_submessage(SubMsg::reply_on_success(
            WasmMsg::Execute {
                contract_addr: mars_token_address.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: staking_address.to_string(),
                    amount,
                    msg: to_binary(&mars_staking::ReceiveMsg::Stake {
                        recipient: Some(env.contract.address.to_string()),
                    })?,
                })?,
                funds: vec![],
            },
            0,
        ))
        .add_attribute("action", "create_allocation")
        .add_attribute("user", user_address)
        .add_attribute("mars_received", amount))
}

pub fn execute_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut allocation = ALLOCATIONS.load(deps.storage, &info.sender)?;
    let mut snapshots = VOTING_POWER_SNAPSHOTS.load(deps.storage, &info.sender)?;

    let mut addresses_query = address_provider::helpers::query_addresses(
        &deps.querier,
        config.address_provider_address.clone(),
        vec![MarsContract::Staking, MarsContract::XMarsToken],
    )?;
    let xmars_token_address = addresses_query.pop().unwrap();
    let staking_address = addresses_query.pop().unwrap();

    // withdrawable amount is the amount unlocked minus the amount already withdrawn
    let mars_unlocked_amount = compute_unlocked_amount(
        &config,
        env.block.time.seconds(),
        allocation.mars_allocated_amount,
    );
    let mars_withdrawable_amount = mars_unlocked_amount - allocation.mars_withdrawn_amount;

    // the withdrawable Mars are held by the vesting contract in the form of xMars
    // calculate how many xMars is withdrawable
    let xmars_withdrawable_amount = mars_withdrawable_amount.multiply_ratio(
        allocation.xmars_minted_amount,
        allocation.mars_staked_amount,
    );

    // query the xmars-mars ratio
    let xmars_per_mars_option: Option<Decimal> =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: staking_address.to_string(),
            msg: to_binary(&mars_staking::QueryMsg::XMarsPerMars {})?,
        }))?;
    let xmars_per_mars = xmars_per_mars_option.ok_or(ContractError::XMarsRatioUndefined {})?;

    // if 1 xMars > 1 Mars (in normal circumstances, if staking rewards are accrued), the staker
    // only gets the initially staked Mars. the rest of xMars are burned, effectively returning the
    // staking reward back to others
    //
    // if 1 xMars < 1 Mars (in case there is a shortfall event), we unstall all and burn nothing.
    // in this case, the staker takes the loss
    let xmars_unstake_amount = cmp::min(
        mars_withdrawable_amount * xmars_per_mars,
        xmars_withdrawable_amount,
    );
    let xmars_burn_amount = xmars_withdrawable_amount.checked_sub(xmars_unstake_amount)?;

    // update allocation
    allocation.mars_withdrawn_amount += mars_withdrawable_amount;
    allocation.mars_staked_amount -= mars_withdrawable_amount;
    allocation.xmars_minted_amount -= xmars_withdrawable_amount;
    ALLOCATIONS.save(deps.storage, &info.sender, &allocation)?;

    // update snapshot
    snapshots.push((env.block.height, allocation.xmars_minted_amount));
    VOTING_POWER_SNAPSHOTS.save(deps.storage, &info.sender, &snapshots)?;

    let mut response = Response::new();
    if !xmars_unstake_amount.is_zero() {
        response = response.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: xmars_token_address.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: staking_address.to_string(),
                amount: xmars_unstake_amount,
                msg: to_binary(&mars_staking::ReceiveMsg::Unstake {
                    recipient: Some(info.sender.to_string()),
                })?,
            })?,
            funds: vec![],
        }));
    }
    if !xmars_burn_amount.is_zero() {
        response = response.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: xmars_token_address.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Burn {
                amount: xmars_burn_amount,
            })?,
            funds: vec![],
        }));
    }

    Ok(response
        .add_attribute("action", "withdraw")
        .add_attribute("user", &info.sender)
        .add_attribute("xmars_unstaked", xmars_unstake_amount)
        .add_attribute("xmars_burned", xmars_burn_amount)
        .add_attribute("xmars_withdrawable", xmars_withdrawable_amount)
        .add_attribute("xmars_per_mars", xmars_per_mars.to_string()))
}

fn compute_unlocked_amount(config: &Config<Addr>, timestamp: u64, amount: Uint128) -> Uint128 {
    // Before the end of cliff period, no token will be unlocked
    if timestamp < config.unlock_start_time + config.unlock_cliff {
        Uint128::zero()
    // After the end of cliff, tokens unlock linearly between start time and end time
    } else if timestamp < config.unlock_start_time + config.unlock_duration {
        amount.multiply_ratio(timestamp - config.unlock_start_time, config.unlock_duration)
    // After end time, all tokens are fully unlocked
    } else {
        amount
    }
}

// REPLY

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> Result<Response, ContractError> {
    match reply.id {
        0 => after_staking(deps, env, reply.result.unwrap().events),
        _ => Err(StdError::generic_err(format!("Invalid reply ID: {}", reply.id)).into()),
    }
}

pub fn after_staking(
    deps: DepsMut,
    env: Env,
    events: Vec<Event>,
) -> Result<Response, ContractError> {
    // parse events to find the amounts of Mars staked and xMars minted
    let event = events
        .iter()
        .find(|event| _event_contains_attribute(event, "action", "stake"))
        .ok_or_else(|| ContractError::ReplyParseFailed {
            key: "stake".to_string(),
        })?;

    let mars_staked_amount_str = event
        .attributes
        .iter()
        .cloned()
        .find(|attr| attr.key == "mars_staked")
        .ok_or_else(|| ContractError::ReplyParseFailed {
            key: "mars_staked".to_string(),
        })?
        .value;
    let mars_staked_amount = Uint128::from_str(&mars_staked_amount_str)?;

    let xmars_minted_amount_str = event
        .attributes
        .iter()
        .cloned()
        .find(|attr| attr.key == "xmars_minted")
        .ok_or_else(|| ContractError::ReplyParseFailed {
            key: "xmars_minted".to_string(),
        })?
        .value;
    let xmars_minted_amount = Uint128::from_str(&xmars_minted_amount_str)?;

    // load temporary data, then delete it
    let staker = TEMP_DATA.load(deps.storage)?;
    TEMP_DATA.remove(deps.storage);

    // save the user's allocation
    match ALLOCATIONS.may_load(deps.storage, &staker)? {
        None => {
            let allocation = Allocation {
                mars_allocated_amount: mars_staked_amount,
                mars_withdrawn_amount: Uint128::zero(),
                mars_staked_amount,
                xmars_minted_amount,
            };
            ALLOCATIONS.save(deps.storage, &staker, &allocation)?
        }
        Some(_) => {
            return Err(ContractError::DataAlreadyExists {
                user_address: staker.to_string(),
            })
        }
    }

    // save the user's voting power snapshots
    match VOTING_POWER_SNAPSHOTS.may_load(deps.storage, &staker)? {
        None => {
            let snapshots = vec![(env.block.height, xmars_minted_amount)];
            VOTING_POWER_SNAPSHOTS.save(deps.storage, &staker, &snapshots)?
        }
        Some(_) => {
            return Err(ContractError::DataAlreadyExists {
                user_address: staker.to_string(),
            })
        }
    }

    Ok(Response::new()
        .add_attribute("action", "after_staking")
        .add_attribute("staker", staker)
        .add_attribute("mars_staked", mars_staked_amount)
        .add_attribute("xmars_minted", xmars_minted_amount))
}

fn _event_contains_attribute(event: &Event, key: &str, value: &str) -> bool {
    event
        .attributes
        .iter()
        .any(|attr| attr.key == key && attr.value == value)
}

// QUERIES

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Allocation { user_address } => to_binary(&query_allocation(deps, user_address)?),
        QueryMsg::VotingPowerAt {
            user_address,
            block,
        } => to_binary(&query_voting_power_at(deps, user_address, block)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<Config<String>> {
    Ok(CONFIG.load(deps.storage)?.into())
}

pub fn query_allocation(deps: Deps, user_address: String) -> StdResult<Allocation> {
    let address = deps.api.addr_validate(&user_address)?;
    ALLOCATIONS.load(deps.storage, &address)
}

pub fn query_voting_power_at(deps: Deps, user_address: String, block: u64) -> StdResult<Uint128> {
    let address = deps.api.addr_validate(&user_address)?;
    match VOTING_POWER_SNAPSHOTS.may_load(deps.storage, &address) {
        // An allocation exists for the address and is loaded successfully
        Ok(Some(snapshots)) => Ok(_binary_search(&snapshots, block)),
        // No allocation exists for this address, return zero
        Ok(None) => Ok(Uint128::zero()),
        // An allocation exists for this address, but failed to parse. Throw error in this case
        Err(err) => Err(err),
    }
}

fn _binary_search(snapshots: &[(u64, Uint128)], block: u64) -> Uint128 {
    let mut lower = 0usize;
    let mut upper = snapshots.len() - 1;

    if block < snapshots[lower].0 {
        return Uint128::zero();
    }

    if snapshots[upper].0 < block {
        return snapshots[upper].1;
    }

    while lower < upper {
        let center = upper - (upper - lower) / 2;
        let snapshot = snapshots[center];

        #[allow(clippy::comparison_chain)]
        if snapshot.0 == block {
            return snapshot.1;
        } else if snapshot.0 < block {
            lower = center;
        } else {
            upper = center - 1;
        }
    }

    snapshots[lower].1
}

// TESTS

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::DeserializeOwned;

    use cosmwasm_std::testing::{MockApi, MockStorage, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{
        ContractResult, CosmosMsg, OwnedDeps, ReplyOn, SubMsg, SubMsgExecutionResponse, Timestamp,
        WasmMsg,
    };

    use mars_core::testing::{
        mock_dependencies, mock_env, mock_env_at_block_time, mock_info, MarsMockQuerier,
        MockEnvParams,
    };

    #[test]
    fn test_binary_search() {
        let snapshots = vec![
            (10000, Uint128::zero()),
            (10010, Uint128::new(12345)),
            (10020, Uint128::new(69420)),
            (10030, Uint128::new(88888)),
        ];
        assert_eq!(_binary_search(&snapshots, 10035), Uint128::new(88888));
        assert_eq!(_binary_search(&snapshots, 10030), Uint128::new(88888));
        assert_eq!(_binary_search(&snapshots, 10025), Uint128::new(69420));
        assert_eq!(_binary_search(&snapshots, 10020), Uint128::new(69420));
        assert_eq!(_binary_search(&snapshots, 10015), Uint128::new(12345));
        assert_eq!(_binary_search(&snapshots, 10010), Uint128::new(12345));
        assert_eq!(_binary_search(&snapshots, 10005), Uint128::zero());
        assert_eq!(_binary_search(&snapshots, 10000), Uint128::zero());
        assert_eq!(_binary_search(&snapshots, 9995), Uint128::zero());
    }

    #[test]
    fn proper_instantiation() {
        let deps = th_setup();
        let env = mock_env(MockEnvParams::default());

        let res: Config<String> = query_helper(deps.as_ref(), env, QueryMsg::Config {});
        let expected = Config {
            address_provider_address: "address_provider".to_string(),
            unlock_start_time: 1640995200, // 2022-01-01
            unlock_cliff: 15552000,        // 180 days
            unlock_duration: 94608000,     // 3 years
        };
        assert_eq!(res, expected)
    }

    #[test]
    fn invalid_instantiation() {
        let mut deps = mock_dependencies(&[]);
        let block_time_sec = 1000u64;
        let env = mock_env_at_block_time(block_time_sec);

        let valid_msg = InstantiateMsg {
            address_provider_address: "address_provider".to_string(),
            unlock_start_time: 1640995200, // 2022-01-01
            unlock_cliff: 15552000,        // 180 days
            unlock_duration: 94608000,     // 3 years
        };

        // unlock_start_time < current time
        let invalid_msg = InstantiateMsg {
            unlock_start_time: block_time_sec - 1u64,
            ..valid_msg.clone()
        };
        let error_res = instantiate(
            deps.as_mut(),
            env.clone(),
            mock_info("deployer"),
            invalid_msg,
        )
        .unwrap_err();
        assert_eq!(error_res, ContractError::InvalidUnlockTimeSetup {});

        // unlock_duration = 0
        let invalid_msg = InstantiateMsg {
            unlock_duration: 0u64,
            ..valid_msg.clone()
        };
        let error_res = instantiate(
            deps.as_mut(),
            env.clone(),
            mock_info("deployer"),
            invalid_msg,
        )
        .unwrap_err();
        assert_eq!(error_res, ContractError::InvalidUnlockTimeSetup {})
    }

    #[test]
    fn creating_allocation() {
        let mut deps = th_setup();
        let env = mock_env(MockEnvParams::default());

        // create an allocation for alice
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            amount: Uint128::new(100000000), // 100 Mars
            sender: "protocol_admin".to_string(),
            msg: to_binary(&ReceiveMsg::CreateAllocation {
                user_address: "alice".to_string(),
            })
            .unwrap(),
        });
        let res = execute(deps.as_mut(), env.clone(), mock_info("mars_token"), msg).unwrap();
        assert_eq!(res.messages.len(), 1);

        let expected = SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "mars_token".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: "staking".to_string(),
                    amount: Uint128::new(100000000),
                    msg: to_binary(&mars_staking::ReceiveMsg::Stake {
                        recipient: Some(MOCK_CONTRACT_ADDR.to_string()),
                    })
                    .unwrap(),
                })
                .unwrap(),
                funds: vec![],
            }),
            gas_limit: None,
            reply_on: ReplyOn::Success,
        };
        assert_eq!(res.messages[0], expected);

        // should have saved temporary data
        let staker = TEMP_DATA.load(&deps.storage).unwrap();
        assert_eq!(staker, Addr::unchecked("alice"));

        // handle reply
        // stake 100_000_000 uMars, at exchange rate 1 xMars = 1.1 Mars
        // should receive 90909090 uXMars
        let event = Event::new("from_contract")
            .add_attribute("action", "stake")
            .add_attribute("mars_staked", "100000000")
            .add_attribute("xmars_minted", "90909090");
        let _reply = Reply {
            id: 0,
            result: ContractResult::Ok(SubMsgExecutionResponse {
                events: vec![event],
                data: None,
            }),
        };
        reply(deps.as_mut(), env.clone(), _reply.clone()).unwrap();

        // temporary data should have been removed
        let temp_data_load_result = TEMP_DATA.may_load(deps.as_ref().storage);
        assert_eq!(temp_data_load_result, Ok(None));

        // allocation data for alice should have been created
        let msg = QueryMsg::Allocation {
            user_address: "alice".to_string(),
        };
        let res: Allocation = query_helper(deps.as_ref(), env.clone(), msg);
        let expected = Allocation {
            mars_allocated_amount: Uint128::new(100000000),
            mars_withdrawn_amount: Uint128::zero(),
            mars_staked_amount: Uint128::new(100000000),
            xmars_minted_amount: Uint128::new(90909090),
        };
        assert_eq!(res, expected);

        // try create an allocation for alice again; should fail
        TEMP_DATA
            .save(deps.as_mut().storage, &Addr::unchecked("alice"))
            .unwrap();

        let res = reply(deps.as_mut(), env.clone(), _reply);
        let expected = Err(ContractError::DataAlreadyExists {
            user_address: "alice".to_string(),
        });
        assert_eq!(res, expected);

        // non-admin try to create an allocation; should fail
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            amount: Uint128::new(100000000), // 100 Mars
            sender: "not_protocol_admin".to_string(),
            msg: to_binary(&ReceiveMsg::CreateAllocation {
                user_address: "bob".to_string(),
            })
            .unwrap(),
        });
        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info("mars_token"),
            msg.clone(),
        );
        assert_eq!(res, Err(ContractError::Mars(MarsError::Unauthorized {})));

        // try creating an allocation using a token rather than Mars; should fail
        let res = execute(deps.as_mut(), env.clone(), mock_info("not_mars_token"), msg);
        assert_eq!(res, Err(ContractError::InvalidTokenDeposit {}));
    }

    #[test]
    fn withdrawing() {
        // deploy contract
        let mut deps = th_setup();
        let env = mock_env(MockEnvParams::default());

        // create an allocatin for alice
        TEMP_DATA
            .save(deps.as_mut().storage, &Addr::unchecked("alice"))
            .unwrap();

        let event = Event::new("from_contract")
            .add_attribute("action", "stake")
            .add_attribute("mars_staked", "100000000")
            .add_attribute("xmars_minted", "90909090");
        let _reply = Reply {
            id: 0,
            result: ContractResult::Ok(SubMsgExecutionResponse {
                events: vec![event],
                data: None,
            }),
        };
        reply(deps.as_mut(), env.clone(), _reply).unwrap();

        //------------------------------
        // 2023-01-01
        // timestamp: 1672531200
        // 31536000 seconds since unlock started
        //
        // Mars per xMars = 1.2
        //
        // Mars unlocked amount = 100000000 * 31536000 / 94608000 = 33333333
        // Mars withdrawn amount = 0
        // Mars withdrawable amount = 33333333 - 0 = 33333333
        //
        // xMars withdrawable amount = 90909090 * 33333333 / 100000000 = 30303029
        // xMars unstake amount = 33333333 / 1.2 = 27777777
        // xMars burn amount = 30303029 - 27777777 = 2525252
        deps.querier
            .set_staking_mars_per_xmars(Decimal::from_ratio(12u128, 10u128));

        let env = mock_env_at_block_time(1672531200);
        let msg = ExecuteMsg::Withdraw {};
        let res = execute(deps.as_mut(), env.clone(), mock_info("alice"), msg).unwrap();
        assert_eq!(res.messages.len(), 2);

        let expected = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "xmars_token".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: "staking".to_string(),
                amount: Uint128::new(27777777),
                msg: to_binary(&mars_staking::ReceiveMsg::Unstake {
                    recipient: Some("alice".to_string()),
                })
                .unwrap(),
            })
            .unwrap(),
            funds: vec![],
        });
        assert_eq!(res.messages[0].msg, expected);

        let expected = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "xmars_token".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Burn {
                amount: Uint128::new(2525252),
            })
            .unwrap(),
            funds: vec![],
        });
        assert_eq!(res.messages[1].msg, expected);

        // Mars withdrawn amount = 0 + 33333333 = 33333333
        // Mars staked amount = 100000000 - 33333333 = 66666667
        // xMars minted amount = 90909090 - 30303029 = 60606061
        let msg = QueryMsg::Allocation {
            user_address: "alice".to_string(),
        };
        let res: Allocation = query_helper(deps.as_ref(), env, msg);
        let expected = Allocation {
            mars_allocated_amount: Uint128::new(100000000),
            mars_withdrawn_amount: Uint128::new(33333333),
            mars_staked_amount: Uint128::new(66666667),
            xmars_minted_amount: Uint128::new(60606061),
        };
        assert_eq!(res, expected);

        //------------------------------
        // 2077-06-04
        // timestamp: 3389990400
        // 31536000 seconds since unlock started
        //
        // Mars per xMars = 1.3
        //
        // Mars unlocked amount = 100000000 (completely unlocked)
        // Mars withdrawn amount = 33333333
        // Mars withdrawable amount = 100000000 - 33333333 = 66666667
        //
        // xMars withdrawable amount = 60606061 * 66666667 / 66666667 = 60606061
        // xMars unstake amount = 66666667 / 1.3 = 51282051
        // xMars burn amount = 60606061 - 51282051 = 9324010
        deps.querier
            .set_staking_mars_per_xmars(Decimal::from_ratio(13u128, 10u128));

        let env = mock_env_at_block_time(3389990400);
        let msg = ExecuteMsg::Withdraw {};
        let res = execute(deps.as_mut(), env.clone(), mock_info("alice"), msg).unwrap();
        assert_eq!(res.messages.len(), 2);

        let expected = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "xmars_token".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: "staking".to_string(),
                amount: Uint128::new(51282051),
                msg: to_binary(&mars_staking::ReceiveMsg::Unstake {
                    recipient: Some("alice".to_string()),
                })
                .unwrap(),
            })
            .unwrap(),
            funds: vec![],
        });
        assert_eq!(res.messages[0].msg, expected);

        let expected = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "xmars_token".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Burn {
                amount: Uint128::new(9324010),
            })
            .unwrap(),
            funds: vec![],
        });
        assert_eq!(res.messages[1].msg, expected);

        // Mars withdrawn amount = 100000000 (completely withdrawn)
        // Mars staked amount = 0
        // xMars minted amount = 0
        let msg = QueryMsg::Allocation {
            user_address: "alice".to_string(),
        };
        let res: Allocation = query_helper(deps.as_ref(), env, msg);
        let expected = Allocation {
            mars_allocated_amount: Uint128::new(100000000),
            mars_withdrawn_amount: Uint128::new(100000000),
            mars_staked_amount: Uint128::new(0),
            xmars_minted_amount: Uint128::new(0),
        };
        assert_eq!(res, expected);
    }

    #[test]
    fn withdrawing_at_loss() {
        // deploy contract
        let mut deps = th_setup();

        // create an allocatin for alice
        //------------------------------
        // 2023-01-01
        // timestamp: 1640995200
        // block number: 10000
        // 1 xMars = 1.1 Mars
        TEMP_DATA
            .save(deps.as_mut().storage, &Addr::unchecked("alice"))
            .unwrap();

        let event = Event::new("from_contract")
            .add_attribute("action", "stake")
            .add_attribute("mars_staked", "100000000")
            .add_attribute("xmars_minted", "90909090");
        let _reply = Reply {
            id: 0,
            result: ContractResult::Ok(SubMsgExecutionResponse {
                events: vec![event],
                data: None,
            }),
        };
        let env = mock_env(MockEnvParams {
            block_height: 10000,
            block_time: Timestamp::from_seconds(1640995200),
        });
        reply(deps.as_mut(), env, _reply).unwrap();

        //------------------------------
        // 2023-01-01
        // timestamp: 1672531200
        // 31536000 seconds since unlock started
        //
        // assume a shortfall event occurred, and xMars is now worth less than 1 Mars
        // Mars per xMars = 0.8
        //
        // Mars unlocked amount = 100000000 * 31536000 / 94608000 = 33333333
        // Mars withdrawn amount = 0
        // Mars withdrawable amount = 33333333 - 0 = 33333333
        //
        // xMars withdrawable amount = 90909090 * 33333333 / 100000000 = 30303029
        // xMars unstake amount = min(33333333 / 0.8, 30303029) = 30303029
        // xMars burn amount = 30303029 - 30303029 = 0
        deps.querier
            .set_staking_mars_per_xmars(Decimal::from_ratio(8u128, 10u128));

        let env = mock_env_at_block_time(1672531200);
        let msg = ExecuteMsg::Withdraw {};
        let res = execute(deps.as_mut(), env.clone(), mock_info("alice"), msg).unwrap();
        assert_eq!(res.messages.len(), 1);

        let expected = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "xmars_token".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: "staking".to_string(),
                amount: Uint128::new(30303029),
                msg: to_binary(&mars_staking::ReceiveMsg::Unstake {
                    recipient: Some("alice".to_string()),
                })
                .unwrap(),
            })
            .unwrap(),
            funds: vec![],
        });
        assert_eq!(res.messages[0].msg, expected);

        // Mars withdrawn amount = 0 + 33333333 = 33333333
        // Mars staked amount = 100000000 - 33333333 = 66666667
        // xMars minted amount = 90909090 - 30303029 = 60606061
        let msg = QueryMsg::Allocation {
            user_address: "alice".to_string(),
        };
        let res: Allocation = query_helper(deps.as_ref(), env, msg);
        let expected = Allocation {
            mars_allocated_amount: Uint128::new(100000000),
            mars_withdrawn_amount: Uint128::new(33333333),
            mars_staked_amount: Uint128::new(66666667),
            xmars_minted_amount: Uint128::new(60606061),
        };
        assert_eq!(res, expected);
    }

    #[test]
    fn querying_voting_power() {
        // deploy contract
        let mut deps = th_setup();

        // create an allocatin for alice
        //------------------------------
        // 2023-01-01
        // timestamp: 1640995200
        // block number: 10000
        // 1 xMars = 1.1 Mars
        TEMP_DATA
            .save(deps.as_mut().storage, &Addr::unchecked("alice"))
            .unwrap();

        let event = Event::new("from_contract")
            .add_attribute("action", "stake")
            .add_attribute("mars_staked", "100000000")
            .add_attribute("xmars_minted", "90909090");
        let _reply = Reply {
            id: 0,
            result: ContractResult::Ok(SubMsgExecutionResponse {
                events: vec![event],
                data: None,
            }),
        };
        let env = mock_env(MockEnvParams {
            block_height: 10000,
            block_time: Timestamp::from_seconds(1640995200),
        });
        reply(deps.as_mut(), env, _reply).unwrap();

        //------------------------------
        // 2023-01-01
        // timestamp: 1672531200
        // block number: 10500
        // 1 xMars = 1.2 Mars
        deps.querier
            .set_staking_mars_per_xmars(Decimal::from_ratio(12u128, 10u128));

        let env = mock_env(MockEnvParams {
            block_height: 10500,
            block_time: Timestamp::from_seconds(1672531200),
        });
        let msg = ExecuteMsg::Withdraw {};
        execute(deps.as_mut(), env, mock_info("alice"), msg).unwrap();

        //------------------------------
        // 2077-06-04
        // timestamp: 3389990400
        // block number: 11000
        // 1 xMars = 1.3 Mars
        deps.querier
            .set_staking_mars_per_xmars(Decimal::from_ratio(13u128, 10u128));

        let env = mock_env(MockEnvParams {
            block_height: 11000,
            block_time: Timestamp::from_seconds(3389990400),
        });
        let msg = ExecuteMsg::Withdraw {};
        execute(deps.as_mut(), env, mock_info("alice"), msg).unwrap();

        assert_eq!(voting_power(deps.as_ref(), 9750), Uint128::zero());
        assert_eq!(voting_power(deps.as_ref(), 10000), Uint128::new(90909090));
        assert_eq!(voting_power(deps.as_ref(), 10250), Uint128::new(90909090));
        assert_eq!(voting_power(deps.as_ref(), 10500), Uint128::new(60606061));
        assert_eq!(voting_power(deps.as_ref(), 10750), Uint128::new(60606061));
        assert_eq!(voting_power(deps.as_ref(), 11000), Uint128::zero());
        assert_eq!(voting_power(deps.as_ref(), 11250), Uint128::zero());
    }

    // TEST HELPERS
    fn th_setup() -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier> {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env(MockEnvParams::default());

        // instantiate contract
        let msg = InstantiateMsg {
            address_provider_address: "address_provider".to_string(),
            unlock_start_time: 1640995200, // 2022-01-01
            unlock_cliff: 15552000,        // 180 days
            unlock_duration: 94608000,     // 3 years
        };
        instantiate(deps.as_mut(), env.clone(), mock_info("deployer"), msg).unwrap();

        deps
    }

    fn query_helper<T: DeserializeOwned>(deps: Deps, env: Env, msg: QueryMsg) -> T {
        from_binary(&query(deps, env, msg).unwrap()).unwrap()
    }

    fn voting_power(deps: Deps, height: u64) -> Uint128 {
        query_helper(
            deps,
            mock_env(MockEnvParams::default()),
            QueryMsg::VotingPowerAt {
                user_address: "alice".to_string(),
                block: height,
            },
        )
    }
}
