use std::cmp;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    QuerierWrapper, QueryRequest, Response, StdResult, Uint128, WasmMsg, WasmQuery,
};

use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use mars_core::address_provider::{self, MarsContract};
use mars_core::error::MarsError;
use mars_core::math::decimal::Decimal;
use mars_core::staking;
use mars_core::vesting::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg};
use mars_core::vesting::{Allocation, Config, Schedule};

use crate::error::ContractError;
use crate::snapshots::{
    capture_total_voting_power_snapshot, capture_voting_power_snapshot,
    get_total_voting_power_value_at, get_voting_power_value_at,
};
use crate::state::{ALLOCATIONS, CONFIG};

// INSTANTIATE

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    validate_unlock_schedule(msg.unlock_schedule, env.block.time.seconds())?;
    CONFIG.save(deps.storage, &msg.check(deps.api)?)?;
    Ok(Response::default())
}

/// Validate the unlock schedule by applying the following criteria; throw error if any one is not
/// satisfied:
/// - Unlock start time must be no earlier than the current time
/// - Unlock cliff must be greater than zero
/// - Unlock duration must be greater than cliff
fn validate_unlock_schedule(
    unlock_schedule: Schedule,
    current_timestamp: u64,
) -> Result<(), ContractError> {
    if unlock_schedule.start_time <= current_timestamp
        || unlock_schedule.cliff == 0
        || unlock_schedule.duration <= unlock_schedule.cliff
    {
        return Err(ContractError::InvalidUnlockTimeSetup {});
    }
    Ok(())
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
        ReceiveMsg::CreateAllocation {
            user_address,
            vest_schedule,
        } => execute_create_allocation(
            deps,
            env,
            info.sender,
            api.addr_validate(&cw20_msg.sender)?,
            api.addr_validate(&user_address)?,
            cw20_msg.amount,
            vest_schedule,
        ),
    }
}

pub fn execute_create_allocation(
    deps: DepsMut,
    env: Env,
    token: Addr,
    creator: Addr,
    user_address: Addr,
    allocated_amount: Uint128,
    vest_schedule: Schedule,
) -> Result<Response, ContractError> {
    let current_block = env.block.height;
    let config = CONFIG.load(deps.storage)?;

    let mut addresses_query = address_provider::helpers::query_addresses(
        &deps.querier,
        config.address_provider_address,
        vec![
            MarsContract::ProtocolAdmin,
            MarsContract::MarsToken,
            MarsContract::Staking,
        ],
    )?;
    let staking_address = addresses_query.pop().unwrap();
    let mars_token_address = addresses_query.pop().unwrap();
    let protocol_admin_address = addresses_query.pop().unwrap();

    // Only Mars token can be used to create allocations
    if token != mars_token_address {
        return Err(ContractError::InvalidTokenDeposit {});
    }

    // Only protocol admin can create allocations
    if creator != protocol_admin_address {
        return Err(MarsError::Unauthorized {}.into());
    }

    // Allocations can only be created when Mars:XMars ratio is 1:1
    let xmars_per_mars = get_xmars_per_mars(&deps.querier, &staking_address)?;
    if xmars_per_mars != Decimal::one() {
        return Err(ContractError::MarsXMarsRatioNotOne { xmars_per_mars });
    }

    // Save the user's allocation
    match ALLOCATIONS.may_load(deps.storage, &user_address)? {
        None => {
            let allocation = Allocation {
                allocated_amount,
                withdrawn_amount: Uint128::zero(),
                vest_schedule,
            };
            ALLOCATIONS.save(deps.storage, &user_address, &allocation)?
        }
        Some(_) => {
            return Err(ContractError::DataAlreadyExists {
                user_address: user_address.to_string(),
            })
        }
    }

    // Save the user's voting power snapshot
    capture_voting_power_snapshot(deps.storage, &user_address, current_block, allocated_amount)?;

    // Update total voting power snapshot
    // If no snapshot for this block exists, save the user's allocated amount
    let prev_total_voting_power = get_total_voting_power_value_at(deps.storage, current_block)?;
    let total_voting_power = prev_total_voting_power.checked_add(allocated_amount)?;
    capture_total_voting_power_snapshot(deps.storage, current_block, total_voting_power)?;

    Ok(Response::new()
        .add_attribute("action", "create_allocation")
        .add_attribute("user", user_address)
        .add_attribute("allocated_amount", allocated_amount))
}

/// Query the amount of xMARS that will be minted when staking one unit of MARS
///
/// NOTE: If no MARS token is staked, i.e. `total_mars_for_stakers` is zero, the query function
/// returns `Ok(None)`. In this case, staking contract mints the same amount of xMARS as the MARS
/// to be staked, i.e. a MARS:xMARS ratio of 1:1.
fn get_xmars_per_mars(querier: &QuerierWrapper, staking_address: &Addr) -> StdResult<Decimal> {
    let xmars_per_mars_option: Option<Decimal> =
        querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: staking_address.into(),
            msg: to_binary(&staking::msg::QueryMsg::XMarsPerMars {})?,
        }))?;
    match xmars_per_mars_option {
        Some(xmars_per_mars) => Ok(xmars_per_mars),
        None => Ok(Decimal::one()),
    }
}

pub fn execute_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let current_block = env.block.height;
    let config = CONFIG.load(deps.storage)?;
    let mut allocation = ALLOCATIONS.load(deps.storage, &info.sender)?;

    let mars_token_address = address_provider::helpers::query_address(
        &deps.querier,
        config.address_provider_address,
        MarsContract::MarsToken,
    )?;

    // Calculate the withdrawable amount
    //
    // NOTE: We don't check whether withdrawable amount is zero, because in case it is zero, CW20
    // transfer will automatically fail
    let withdrawable_amount = compute_withdrawable_amount(
        allocation.allocated_amount,
        allocation.withdrawn_amount,
        allocation.vest_schedule,
        config.unlock_schedule,
        env.block.time.seconds(),
    )?;

    // Update allocation
    // We don't use checked math here, since we don't expect these to be over/underflow in any case
    allocation.withdrawn_amount += withdrawable_amount;
    ALLOCATIONS.save(deps.storage, &info.sender, &allocation)?;

    // Update the user's voting power snapshot
    let prev_voting_power = get_voting_power_value_at(deps.storage, &info.sender, current_block)?;
    let voting_power = prev_voting_power.checked_sub(withdrawable_amount)?;
    capture_voting_power_snapshot(deps.storage, &info.sender, current_block, voting_power)?;

    // Update total voting power snapshots
    let prev_total_voting_power = get_total_voting_power_value_at(deps.storage, current_block)?;
    let total_voting_power = prev_total_voting_power.checked_sub(withdrawable_amount)?;
    capture_total_voting_power_snapshot(deps.storage, current_block, total_voting_power)?;

    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: mars_token_address.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount: withdrawable_amount,
            })?,
            funds: vec![],
        }))
        .add_attribute("action", "withdraw")
        .add_attribute("user", &info.sender)
        .add_attribute("withdrawn_amount", withdrawable_amount))
}

/// Compute the withdrawable based on the current timestamp, the vesting schedule, and the unlock
/// schedule
///
/// The vested amount and unlocked amount are computed separately, and the withdrawable amount is
/// whichever one is smaller, minus the amount already withdrawn.
fn compute_withdrawable_amount(
    allocated_amount: Uint128,
    withdrawn_amount: Uint128,
    vest_schedule: Schedule,
    unlock_schedule: Schedule,
    current_time: u64,
) -> StdResult<Uint128> {
    let f = |schedule: Schedule| {
        // Before the end of cliff period, no token will be vested/unlocked
        if current_time < schedule.start_time + schedule.cliff {
            Uint128::zero()
        // After the end of cliff, tokens vest/unlock linearly between start time and end time
        } else if current_time < schedule.start_time + schedule.duration {
            allocated_amount.multiply_ratio(current_time - schedule.start_time, schedule.duration)
        // After end time, all tokens are fully vested/unlocked
        } else {
            allocated_amount
        }
    };

    let vested_amount = f(vest_schedule);
    let unlocked_amount = f(unlock_schedule);

    cmp::min(vested_amount, unlocked_amount)
        .checked_sub(withdrawn_amount)
        .map_err(|overflow_err| overflow_err.into())
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
        QueryMsg::TotalVotingPowerAt { block } => {
            to_binary(&query_total_voting_power_at(deps, block)?)
        }
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
    get_voting_power_value_at(deps.storage, &deps.api.addr_validate(&user_address)?, block)
}

pub fn query_total_voting_power_at(deps: Deps, block: u64) -> StdResult<Uint128> {
    get_total_voting_power_value_at(deps.storage, block)
}

// TESTS

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{MockApi, MockStorage};
    use cosmwasm_std::{CosmosMsg, OwnedDeps, ReplyOn, SubMsg, Timestamp, WasmMsg};
    use mars_core::testing::{
        mock_dependencies, mock_env, mock_info, MarsMockQuerier, MockEnvParams,
    };
    use serde::de::DeserializeOwned;

    const MOCK_UNLOCK_SCHEDULE: Schedule = Schedule {
        start_time: 1635724800, // 2021-11-01
        cliff: 31536000,        // 1 year (365 days)
        duration: 94608000,     // 3 years (3 * 365 days)
    };
    const MOCK_VEST_SCHEDULE: Schedule = Schedule {
        start_time: 1614556800, // 2021-03-01
        cliff: 15552000,        // 180 days
        duration: 94608000,     // 3 years
    };

    #[test]
    fn proper_instantiation() {
        let deps = th_setup();
        let env = mock_env(MockEnvParams::default());

        let res: Config<String> = query_helper(deps.as_ref(), env, QueryMsg::Config {});
        let expected = Config {
            address_provider_address: "address_provider".to_string(),
            unlock_schedule: Schedule {
                start_time: 1635724800,
                cliff: 31536000,
                duration: 94608000,
            },
        };
        assert_eq!(res, expected)
    }

    #[test]
    fn creating_allocation() {
        let mut deps = th_setup();
        let env = mock_env(MockEnvParams::default());

        // allocation data for alice should have been created
        let query_msg = QueryMsg::Allocation {
            user_address: "alice".to_string(),
        };
        let res: Allocation = query_helper(deps.as_ref(), env.clone(), query_msg);
        let expected = Allocation {
            allocated_amount: Uint128::new(100000000),
            withdrawn_amount: Uint128::zero(),
            vest_schedule: MOCK_VEST_SCHEDULE,
        };
        assert_eq!(res, expected);

        // try create an allocation for alice again; should fail
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            amount: Uint128::new(100000000), // 100 Mars
            sender: "protocol_admin".to_string(),
            msg: to_binary(&ReceiveMsg::CreateAllocation {
                user_address: "alice".to_string(),
                vest_schedule: MOCK_VEST_SCHEDULE,
            })
            .unwrap(),
        });
        let res = execute(deps.as_mut(), env.clone(), mock_info("mars_token"), msg);
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
                vest_schedule: MOCK_VEST_SCHEDULE,
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

        //------------------------------------------------------------------------------------------
        // 2021-12-01
        // height: 10020
        // time: 1638316800
        //
        // before unlock cliff, zero token should be withdrawable
        //
        // NOTE: the transaction should fail in this case because CW20 forbids sending zero amount
        let env = mock_env(MockEnvParams {
            block_height: 10010,
            block_time: Timestamp::from_seconds(1638316800),
        });
        let msg = ExecuteMsg::Withdraw {};
        let res = execute(deps.as_mut(), env.clone(), mock_info("alice"), msg).unwrap();
        let expected = SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "mars_token".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: String::from("alice"),
                    amount: Uint128::zero(),
                })
                .unwrap(),
                funds: vec![],
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never,
        };
        assert_eq!(res.messages.len(), 1);
        assert_eq!(res.messages[0], expected);

        //------------------------------------------------------------------------------------------
        // 2022-12-01
        // height: 10030
        // time: 1669852800
        //
        // vested_amount = 100000000 * (1669852800 - 1614556800) / 94608000 = 58447488
        // unlocked_amount = 100000000 * (1669852800 - 1635724800) / 94608000 = 36073059
        // withdrawable_amount = min(vested_amount, unlocked_amount) = 36073059
        let env = mock_env(MockEnvParams {
            block_height: 10030,
            block_time: Timestamp::from_seconds(1669852800),
        });

        let msg = ExecuteMsg::Withdraw {};
        let res = execute(deps.as_mut(), env.clone(), mock_info("alice"), msg).unwrap();
        let expected = SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "mars_token".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: String::from("alice"),
                    amount: Uint128::new(36073059),
                })
                .unwrap(),
                funds: vec![],
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never,
        };
        assert_eq!(res.messages.len(), 1);
        assert_eq!(res.messages[0], expected);

        let msg = QueryMsg::Allocation {
            user_address: "alice".to_string(),
        };
        let res: Allocation = query_helper(deps.as_ref(), env, msg);
        let expected = Allocation {
            allocated_amount: Uint128::new(100000000),
            withdrawn_amount: Uint128::new(36073059),
            vest_schedule: MOCK_VEST_SCHEDULE,
        };
        assert_eq!(res, expected);

        //------------------------------------------------------------------------------------------
        // 2024-03-01
        // height: 10040
        // time: 1709251200
        //
        // vested_amount = 100000000 (fully vested)
        // unlocked_amount = 100000000 * (1709251200 - 1635724800) / 94608000 = 77716894
        // withdrawable_amount = min(vested_amount, unlocked_amount) - withdrawn_amount
        // = 77716894 - 36073059 = 41643835
        let env = mock_env(MockEnvParams {
            block_height: 10040,
            block_time: Timestamp::from_seconds(1709251200),
        });

        let msg = ExecuteMsg::Withdraw {};
        let res = execute(deps.as_mut(), env.clone(), mock_info("alice"), msg).unwrap();
        let expected = SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "mars_token".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: String::from("alice"),
                    amount: Uint128::new(41643835),
                })
                .unwrap(),
                funds: vec![],
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never,
        };
        assert_eq!(res.messages.len(), 1);
        assert_eq!(res.messages[0], expected);

        let msg = QueryMsg::Allocation {
            user_address: "alice".to_string(),
        };
        let res: Allocation = query_helper(deps.as_ref(), env, msg);
        let expected = Allocation {
            allocated_amount: Uint128::new(100000000),
            withdrawn_amount: Uint128::new(77716894),
            vest_schedule: MOCK_VEST_SCHEDULE,
        };
        assert_eq!(res, expected);

        //------------------------------------------------------------------------------------------
        // 2077-01-01
        // height: 10050
        // time: 3376684800
        //
        // fully vested and unlocked
        // withdrawable_amount = 100000000 - 77716894 = 22283106
        let env = mock_env(MockEnvParams {
            block_height: 10050,
            block_time: Timestamp::from_seconds(3376684800),
        });

        let msg = ExecuteMsg::Withdraw {};
        let res = execute(deps.as_mut(), env.clone(), mock_info("alice"), msg).unwrap();
        let expected = SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "mars_token".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: String::from("alice"),
                    amount: Uint128::new(22283106),
                })
                .unwrap(),
                funds: vec![],
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never,
        };
        assert_eq!(res.messages.len(), 1);
        assert_eq!(res.messages[0], expected);

        let msg = QueryMsg::Allocation {
            user_address: "alice".to_string(),
        };
        let res: Allocation = query_helper(deps.as_ref(), env, msg);
        let expected = Allocation {
            allocated_amount: Uint128::new(100000000),
            withdrawn_amount: Uint128::new(100000000),
            vest_schedule: MOCK_VEST_SCHEDULE,
        };
        assert_eq!(res, expected);
    }

    #[test]
    fn querying_voting_powers() {
        // deploy contract
        let mut deps = th_setup();

        //------------------------------
        // 2023-01-01
        // timestamp: 1672531200
        // block number: 10500
        //
        // vested_amount = 100000000 * (1672531200 - 1614556800) / 94608000 = 61278538
        // unlocked_amount = 100000000 * (1672531200 - 1635724800) / 94608000 = 38904109
        // withdrawable_amount = 38904109
        // available_amount = 100000000 - 38904109 = 61095891
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
        //
        // fully wihtdrawn
        let env = mock_env(MockEnvParams {
            block_height: 11000,
            block_time: Timestamp::from_seconds(3389990400),
        });
        let msg = ExecuteMsg::Withdraw {};
        execute(deps.as_mut(), env, mock_info("alice"), msg).unwrap();

        //------------------------------
        // timestamp: 9999999999
        // block number: 69420
        //
        // bob finally withdraws
        let env = mock_env(MockEnvParams {
            block_height: 69420,
            block_time: Timestamp::from_seconds(9999999999),
        });
        let msg = ExecuteMsg::Withdraw {};
        execute(deps.as_mut(), env, mock_info("bob"), msg).unwrap();

        assert_eq!(
            voting_power_at(deps.as_ref(), "alice", 10000),
            Uint128::zero()
        );
        assert_eq!(
            voting_power_at(deps.as_ref(), "alice", 10010),
            Uint128::new(100000000)
        );
        assert_eq!(
            voting_power_at(deps.as_ref(), "alice", 10020),
            Uint128::new(100000000)
        );
        assert_eq!(
            voting_power_at(deps.as_ref(), "alice", 10499),
            Uint128::new(100000000)
        );
        assert_eq!(
            voting_power_at(deps.as_ref(), "alice", 10500),
            Uint128::new(61095891)
        );
        assert_eq!(
            voting_power_at(deps.as_ref(), "alice", 10750),
            Uint128::new(61095891)
        );
        assert_eq!(
            voting_power_at(deps.as_ref(), "alice", 10999),
            Uint128::new(61095891)
        );
        assert_eq!(
            voting_power_at(deps.as_ref(), "alice", 11000),
            Uint128::zero()
        );
        assert_eq!(
            voting_power_at(deps.as_ref(), "alice", 88888),
            Uint128::zero()
        );

        assert_eq!(
            voting_power_at(deps.as_ref(), "bob", 10000),
            Uint128::zero()
        );
        assert_eq!(
            voting_power_at(deps.as_ref(), "bob", 10010),
            Uint128::new(100000000)
        );
        assert_eq!(
            voting_power_at(deps.as_ref(), "bob", 69419),
            Uint128::new(100000000)
        );
        assert_eq!(
            voting_power_at(deps.as_ref(), "bob", 69420),
            Uint128::zero()
        );
        assert_eq!(
            voting_power_at(deps.as_ref(), "bob", 88888),
            Uint128::zero()
        );

        assert_eq!(total_voting_power_at(deps.as_ref(), 10000), Uint128::zero());
        assert_eq!(
            total_voting_power_at(deps.as_ref(), 10010),
            Uint128::new(200000000)
        );
        assert_eq!(
            total_voting_power_at(deps.as_ref(), 10020),
            Uint128::new(200000000)
        );
        assert_eq!(
            total_voting_power_at(deps.as_ref(), 10499),
            Uint128::new(200000000)
        );
        assert_eq!(
            total_voting_power_at(deps.as_ref(), 10500),
            Uint128::new(161095891)
        );
        assert_eq!(
            total_voting_power_at(deps.as_ref(), 10750),
            Uint128::new(161095891)
        );
        assert_eq!(
            total_voting_power_at(deps.as_ref(), 10999),
            Uint128::new(161095891)
        );
        assert_eq!(
            total_voting_power_at(deps.as_ref(), 11000),
            Uint128::new(100000000)
        );
        assert_eq!(
            total_voting_power_at(deps.as_ref(), 69419),
            Uint128::new(100000000)
        );
        assert_eq!(total_voting_power_at(deps.as_ref(), 69420), Uint128::zero());
        assert_eq!(total_voting_power_at(deps.as_ref(), 88888), Uint128::zero());
    }

    // TEST HELPERS
    fn th_setup() -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier> {
        let mut deps = mock_dependencies(&[]);

        // deploy contract at block 10010
        let env = mock_env(MockEnvParams {
            block_height: 10010,
            block_time: Timestamp::from_seconds(0),
        });

        // instantiate the contract
        let msg = InstantiateMsg {
            address_provider_address: "address_provider".to_string(),
            unlock_schedule: MOCK_UNLOCK_SCHEDULE,
        };
        instantiate(deps.as_mut(), env.clone(), mock_info("deployer"), msg).unwrap();

        let msg = |user: &str| {
            ExecuteMsg::Receive(Cw20ReceiveMsg {
                amount: Uint128::new(100000000), // 100 Mars
                sender: "protocol_admin".to_string(),
                msg: to_binary(&ReceiveMsg::CreateAllocation {
                    user_address: user.to_string(),
                    vest_schedule: MOCK_VEST_SCHEDULE,
                })
                .unwrap(),
            })
        };

        // create an allocation for alice
        execute(
            deps.as_mut(),
            env.clone(),
            mock_info("mars_token"),
            msg("alice"),
        )
        .unwrap();

        // create an allocation for bob
        execute(
            deps.as_mut(),
            env.clone(),
            mock_info("mars_token"),
            msg("bob"),
        )
        .unwrap();

        deps
    }

    fn query_helper<T: DeserializeOwned>(deps: Deps, env: Env, msg: QueryMsg) -> T {
        from_binary(&query(deps, env, msg).unwrap()).unwrap()
    }

    fn voting_power_at(deps: Deps, user: &str, height: u64) -> Uint128 {
        query_helper(
            deps,
            mock_env(MockEnvParams::default()),
            QueryMsg::VotingPowerAt {
                user_address: user.to_string(),
                block: height,
            },
        )
    }

    fn total_voting_power_at(deps: Deps, height: u64) -> Uint128 {
        query_helper(
            deps,
            mock_env(MockEnvParams::default()),
            QueryMsg::TotalVotingPowerAt { block: height },
        )
    }
}
