use cosmwasm_std::{
    attr, entry_point, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order,
    OverflowError, OverflowOperation, QueryRequest, Response, StdError, StdResult, Uint128,
    WasmMsg, WasmQuery,
};

use mars_core::error::MarsError;
use mars_core::helpers::option_string_to_addr;
use mars_core::math::decimal::Decimal;

use mars_core::address_provider;
use mars_core::address_provider::MarsContract;
use mars_core::staking;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{ASSET_INCENTIVES, CONFIG, USER_ASSET_INDICES, USER_UNCLAIMED_REWARDS};
use crate::{AssetIncentive, AssetIncentiveResponse, Config};

// INIT

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let config = Config {
        owner: deps.api.addr_validate(&msg.owner)?,
        address_provider_address: deps.api.addr_validate(&msg.address_provider_address)?,
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
        ExecuteMsg::SetAssetIncentive {
            ma_token_address,
            emission_per_second,
        } => execute_set_asset_incentive(deps, env, info, ma_token_address, emission_per_second),
        ExecuteMsg::BalanceChange {
            user_address,
            user_balance_before,
            total_supply_before,
        } => execute_balance_change(
            deps,
            env,
            info,
            user_address,
            user_balance_before,
            total_supply_before,
        ),
        ExecuteMsg::ClaimRewards {} => execute_claim_rewards(deps, env, info),
        ExecuteMsg::UpdateConfig {
            owner,
            address_provider_address,
        } => Ok(execute_update_config(
            deps,
            env,
            info,
            owner,
            address_provider_address,
        )?),
        ExecuteMsg::ExecuteCosmosMsg(cosmos_msg) => {
            Ok(execute_execute_cosmos_msg(deps, env, info, cosmos_msg)?)
        }
    }
}

pub fn execute_set_asset_incentive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    ma_token_address: String,
    emission_per_second: Uint128,
) -> Result<Response, ContractError> {
    // only owner can call this
    let config = CONFIG.load(deps.storage)?;
    let owner = config.owner;
    if info.sender != owner {
        return Err(MarsError::Unauthorized {}.into());
    }

    let ma_asset_address = deps.api.addr_validate(&ma_token_address)?;

    let new_asset_incentive = match ASSET_INCENTIVES.may_load(deps.storage, &ma_asset_address)? {
        Some(mut asset_incentive) => {
            // Update index up to now
            let total_supply =
                mars_core::helpers::cw20_get_total_supply(&deps.querier, ma_asset_address.clone())?;
            asset_incentive_update_index(
                &mut asset_incentive,
                total_supply,
                env.block.time.seconds(),
            )?;

            // Set new emission
            asset_incentive.emission_per_second = emission_per_second;

            asset_incentive
        }
        None => AssetIncentive {
            emission_per_second,
            index: Decimal::zero(),
            last_updated: env.block.time.seconds(),
        },
    };

    ASSET_INCENTIVES.save(deps.storage, &ma_asset_address, &new_asset_incentive)?;

    let response = Response::new().add_attributes(vec![
        attr("action", "set_asset_incentive"),
        attr("ma_asset", ma_token_address),
        attr("emission_per_second", emission_per_second),
    ]);
    Ok(response)
}

pub fn execute_balance_change(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user_address: Addr,
    user_balance_before: Uint128,
    total_supply_before: Uint128,
) -> Result<Response, ContractError> {
    let ma_token_address = info.sender;
    let mut asset_incentive = match ASSET_INCENTIVES.may_load(deps.storage, &ma_token_address)? {
        // If there are no incentives,
        // an empty successful response is returned as the
        // success of the call is needed for the call that triggered the change to
        // succeed and be persisted to state.
        None => return Ok(Response::default()),

        Some(ai) => ai,
    };

    asset_incentive_update_index(
        &mut asset_incentive,
        total_supply_before,
        env.block.time.seconds(),
    )?;
    ASSET_INCENTIVES.save(deps.storage, &ma_token_address, &asset_incentive)?;

    // Check if user has accumulated uncomputed rewards (which means index is not up to date)
    let user_asset_index_key = USER_ASSET_INDICES.key((&user_address, &ma_token_address));

    let user_asset_index = user_asset_index_key
        .may_load(deps.storage)?
        .unwrap_or_else(Decimal::zero);

    let mut accrued_rewards = Uint128::zero();

    if user_asset_index != asset_incentive.index {
        // Compute user accrued rewards and update state
        accrued_rewards = user_compute_accrued_rewards(
            user_balance_before,
            user_asset_index,
            asset_incentive.index,
        )?;

        // Store user accrued rewards as unclaimed
        if !accrued_rewards.is_zero() {
            USER_UNCLAIMED_REWARDS.update(
                deps.storage,
                &user_address,
                |ur: Option<Uint128>| -> StdResult<Uint128> {
                    match ur {
                        Some(unclaimed_rewards) => Ok(unclaimed_rewards + accrued_rewards),
                        None => Ok(accrued_rewards),
                    }
                },
            )?;
        }

        user_asset_index_key.save(deps.storage, &asset_incentive.index)?;
    }

    let response = Response::new().add_attributes(vec![
        attr("action", "balance_change"),
        attr("ma_asset", ma_token_address),
        attr("user", user_address),
        attr("rewards_accrued", accrued_rewards),
        attr("asset_index", asset_incentive.index.to_string()),
    ]);

    Ok(response)
}

pub fn execute_claim_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let user_address = info.sender;
    let (total_unclaimed_rewards, user_asset_incentive_statuses_to_update) =
        compute_user_unclaimed_rewards(deps.as_ref(), &env, &user_address)?;

    // Commit updated asset_incentives and user indexes
    for user_asset_incentive_status in user_asset_incentive_statuses_to_update {
        let asset_incentive_updated = user_asset_incentive_status.asset_incentive_updated;

        ASSET_INCENTIVES.save(
            deps.storage,
            &user_asset_incentive_status.ma_token_address,
            &asset_incentive_updated,
        )?;

        if asset_incentive_updated.index != user_asset_incentive_status.user_index_current {
            USER_ASSET_INDICES.save(
                deps.storage,
                (&user_address, &user_asset_incentive_status.ma_token_address),
                &asset_incentive_updated.index,
            )?
        }
    }

    // clear unclaimed rewards
    USER_UNCLAIMED_REWARDS.save(deps.storage, &user_address, &Uint128::zero())?;

    let mut response = Response::new();
    if total_unclaimed_rewards > Uint128::zero() {
        // Build message to stake mars and send resulting xmars to the user
        let config = CONFIG.load(deps.storage)?;
        let mars_contracts = vec![MarsContract::MarsToken, MarsContract::Staking];
        let mut addresses_query = address_provider::helpers::query_addresses(
            &deps.querier,
            config.address_provider_address,
            mars_contracts,
        )?;
        let staking_address = addresses_query.pop().unwrap();
        let mars_token_address = addresses_query.pop().unwrap();

        response = response.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: mars_token_address.to_string(),
            msg: to_binary(&cw20::Cw20ExecuteMsg::Send {
                contract: staking_address.to_string(),
                amount: total_unclaimed_rewards,
                msg: to_binary(&staking::msg::ReceiveMsg::Stake {
                    recipient: Some(user_address.to_string()),
                })?,
            })?,
            funds: vec![],
        }));
    };

    response = response.add_attributes(vec![
        attr("action", "claim_rewards"),
        attr("user", user_address),
        attr("mars_staked_as_rewards", total_unclaimed_rewards),
    ]);

    Ok(response)
}

pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner: Option<String>,
    address_provider_address: Option<String>,
) -> Result<Response, MarsError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(MarsError::Unauthorized {});
    };

    config.owner = option_string_to_addr(deps.api, owner, config.owner)?;
    config.address_provider_address = option_string_to_addr(
        deps.api,
        address_provider_address,
        config.address_provider_address,
    )?;

    CONFIG.save(deps.storage, &config)?;

    let response = Response::new().add_attribute("action", "update_config");

    Ok(response)
}

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

// HELPERS

/// Updates asset incentive index and last updated timestamp by computing
/// how many rewards were accrued since last time updated given incentive's
/// emission per second.
/// Total supply is the total (liquidity) token supply during the period being computed.
/// Note that this method does not commit updates to state as that should be executed by the
/// caller
fn asset_incentive_update_index(
    asset_incentive: &mut AssetIncentive,
    total_supply: Uint128,
    current_block_time: u64,
) -> StdResult<()> {
    if (current_block_time != asset_incentive.last_updated)
        && !total_supply.is_zero()
        && !asset_incentive.emission_per_second.is_zero()
    {
        asset_incentive.index = asset_incentive_compute_index(
            asset_incentive.index,
            asset_incentive.emission_per_second,
            total_supply,
            asset_incentive.last_updated,
            current_block_time,
        )?
    }
    asset_incentive.last_updated = current_block_time;
    Ok(())
}

fn asset_incentive_compute_index(
    previous_index: Decimal,
    emission_per_second: Uint128,
    total_supply: Uint128,
    time_start: u64,
    time_end: u64,
) -> StdResult<Decimal> {
    if time_start > time_end {
        return Err(StdError::overflow(OverflowError::new(
            OverflowOperation::Sub,
            time_start,
            time_end,
        )));
    }
    let seconds_elapsed = time_end - time_start;
    let new_index = previous_index
        + Decimal::from_ratio(
            emission_per_second.u128() * seconds_elapsed as u128,
            total_supply,
        );
    Ok(new_index)
}

/// Computes user accrued rewards using the difference between asset_incentive index and
/// user current index
/// asset_incentives index should be up to date.
fn user_compute_accrued_rewards(
    user_balance: Uint128,
    user_asset_index: Decimal,
    asset_incentive_index: Decimal,
) -> StdResult<Uint128> {
    Ok((user_balance * asset_incentive_index) - (user_balance * user_asset_index))
}

/// Result of querying and updating the status of the user and a give asset incentives in order to
/// compute unclaimed rewards.
struct UserAssetIncentiveStatus {
    /// Address of the ma token that's the incentives target
    ma_token_address: Addr,
    /// Current user index's value on the contract store (not updated by current asset index)
    user_index_current: Decimal,
    /// Asset incentive with values updated to the current block (not neccesarily commited
    /// to storage)
    asset_incentive_updated: AssetIncentive,
}

fn compute_user_unclaimed_rewards(
    deps: Deps,
    env: &Env,
    user_address: &Addr,
) -> StdResult<(Uint128, Vec<UserAssetIncentiveStatus>)> {
    let mut total_unclaimed_rewards = USER_UNCLAIMED_REWARDS
        .may_load(deps.storage, user_address)?
        .unwrap_or_else(Uint128::zero);

    let result_asset_incentives: StdResult<Vec<_>> = ASSET_INCENTIVES
        .range(deps.storage, None, None, Order::Ascending)
        .collect();

    let mut user_asset_incentive_statuses_to_update: Vec<UserAssetIncentiveStatus> = vec![];

    for (ma_token_address_bytes, mut asset_incentive) in result_asset_incentives? {
        let ma_token_address = deps
            .api
            .addr_validate(&String::from_utf8(ma_token_address_bytes)?)?;

        // Get asset user balances and total supply
        let balance_and_total_supply: mars_core::ma_token::msg::BalanceAndTotalSupplyResponse =
            deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: ma_token_address.to_string(),
                msg: to_binary(&mars_core::ma_token::msg::QueryMsg::BalanceAndTotalSupply {
                    address: user_address.to_string(),
                })?,
            }))?;

        // If user's balance is 0 there should be no rewards to accrue, so we don't care about
        // updating indexes. If the user's balance changes, the indexes will be updated correctly at
        // that point in time.
        if balance_and_total_supply.balance.is_zero() {
            continue;
        }

        asset_incentive_update_index(
            &mut asset_incentive,
            balance_and_total_supply.total_supply,
            env.block.time.seconds(),
        )?;

        let user_asset_index = USER_ASSET_INDICES
            .may_load(deps.storage, (user_address, &ma_token_address))?
            .unwrap_or_else(Decimal::zero);

        if user_asset_index != asset_incentive.index {
            // Compute user accrued rewards and update user index
            let asset_accrued_rewards = user_compute_accrued_rewards(
                balance_and_total_supply.balance,
                user_asset_index,
                asset_incentive.index,
            )?;
            total_unclaimed_rewards += asset_accrued_rewards;
        }

        user_asset_incentive_statuses_to_update.push(UserAssetIncentiveStatus {
            ma_token_address,
            user_index_current: user_asset_index,
            asset_incentive_updated: asset_incentive,
        });
    }

    Ok((
        total_unclaimed_rewards,
        user_asset_incentive_statuses_to_update,
    ))
}

// QUERIES

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::AssetIncentive { ma_token_address } => {
            to_binary(&query_asset_incentive(deps, ma_token_address)?)
        }
        QueryMsg::UserUnclaimedRewards { user_address } => {
            to_binary(&query_user_unclaimed_rewards(deps, env, user_address)?)
        }
    }
}

fn query_config(deps: Deps) -> StdResult<Config> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}

fn query_asset_incentive(
    deps: Deps,
    ma_token_address_unchecked: String,
) -> StdResult<AssetIncentiveResponse> {
    let ma_token_address = deps.api.addr_validate(&ma_token_address_unchecked)?;
    let option_asset_incentive = ASSET_INCENTIVES.may_load(deps.storage, &ma_token_address)?;
    Ok(AssetIncentiveResponse {
        asset_incentive: option_asset_incentive,
    })
}

fn query_user_unclaimed_rewards(
    deps: Deps,
    env: Env,
    user_address_unchecked: String,
) -> StdResult<Uint128> {
    let user_address = deps.api.addr_validate(&user_address_unchecked)?;
    let (unclaimed_rewards, _) = compute_user_unclaimed_rewards(deps, &env, &user_address)?;

    Ok(unclaimed_rewards)
}

// TESTS

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{
        testing::{mock_env, mock_info, MockApi, MockStorage},
        Addr, BankMsg, Coin, OwnedDeps, SubMsg, Timestamp, Uint128,
    };
    use mars_core::testing::{mock_dependencies, MarsMockQuerier, MockEnvParams};

    // init
    #[test]
    fn test_proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let info = mock_info("sender", &[]);
        let msg = InstantiateMsg {
            owner: String::from("owner"),
            address_provider_address: String::from("address_provider"),
        };

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        let empty_vec: Vec<SubMsg> = vec![];
        assert_eq!(empty_vec, res.messages);

        let config = CONFIG.load(deps.as_ref().storage).unwrap();
        assert_eq!(config.owner, Addr::unchecked("owner"));
        assert_eq!(
            config.address_provider_address,
            Addr::unchecked("address_provider")
        );
    }

    // SetAssetIncentive

    #[test]
    fn test_only_owner_can_set_asset_incentive() {
        let mut deps = th_setup(&[]);

        let info = mock_info("sender", &[]);
        let msg = ExecuteMsg::SetAssetIncentive {
            ma_token_address: String::from("ma_asset"),
            emission_per_second: Uint128::new(100),
        };

        let res_error = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(res_error, ContractError::Mars(MarsError::Unauthorized {}));
    }

    #[test]
    fn test_set_new_asset_incentive() {
        let mut deps = th_setup(&[]);
        let ma_asset_address = Addr::unchecked("ma_asset");

        let info = mock_info("owner", &[]);
        let env = mars_core::testing::mock_env(MockEnvParams {
            block_time: Timestamp::from_seconds(1_000_000),
            ..Default::default()
        });
        let msg = ExecuteMsg::SetAssetIncentive {
            ma_token_address: ma_asset_address.to_string(),
            emission_per_second: Uint128::new(100),
        };

        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "set_asset_incentive"),
                attr("ma_asset", "ma_asset"),
                attr("emission_per_second", "100"),
            ]
        );

        let asset_incentive = ASSET_INCENTIVES
            .load(deps.as_ref().storage, &ma_asset_address)
            .unwrap();

        assert_eq!(asset_incentive.emission_per_second, Uint128::new(100));
        assert_eq!(asset_incentive.index, Decimal::zero());
        assert_eq!(asset_incentive.last_updated, 1_000_000);
    }

    #[test]
    fn test_set_existing_asset_incentive() {
        // setup
        let mut deps = th_setup(&[]);
        let ma_asset_address = Addr::unchecked("ma_asset");
        let ma_asset_total_supply = Uint128::new(2_000_000);
        deps.querier
            .set_cw20_total_supply(ma_asset_address.clone(), ma_asset_total_supply);

        ASSET_INCENTIVES
            .save(
                deps.as_mut().storage,
                &ma_asset_address,
                &AssetIncentive {
                    emission_per_second: Uint128::new(100),
                    index: Decimal::from_ratio(1_u128, 2_u128),
                    last_updated: 500_000,
                },
            )
            .unwrap();

        // execute msg
        let info = mock_info("owner", &[]);
        let env = mars_core::testing::mock_env(MockEnvParams {
            block_time: Timestamp::from_seconds(1_000_000),
            ..Default::default()
        });
        let msg = ExecuteMsg::SetAssetIncentive {
            ma_token_address: ma_asset_address.to_string(),
            emission_per_second: Uint128::new(200),
        };

        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        // tests
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "set_asset_incentive"),
                attr("ma_asset", "ma_asset"),
                attr("emission_per_second", "200"),
            ]
        );

        let asset_incentive = ASSET_INCENTIVES
            .load(deps.as_ref().storage, &ma_asset_address)
            .unwrap();

        let expected_index = asset_incentive_compute_index(
            Decimal::from_ratio(1_u128, 2_u128),
            Uint128::new(100),
            ma_asset_total_supply,
            500_000,
            1_000_000,
        )
        .unwrap();

        assert_eq!(asset_incentive.emission_per_second, Uint128::new(200));
        assert_eq!(asset_incentive.index, expected_index);
        assert_eq!(asset_incentive.last_updated, 1_000_000);
    }

    // BalanceChange

    #[test]
    fn test_execute_balance_change_noops() {
        let mut deps = th_setup(&[]);

        // non existing incentive returns a no op
        let info = mock_info("ma_asset", &[]);
        let msg = ExecuteMsg::BalanceChange {
            user_address: Addr::unchecked("user"),
            user_balance_before: Uint128::new(100000),
            total_supply_before: Uint128::new(100000),
        };

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res, Response::default())
    }

    #[test]
    fn test_balance_change_zero_emission() {
        let mut deps = th_setup(&[]);
        let ma_asset_address = Addr::unchecked("ma_asset");
        let user_address = Addr::unchecked("user");
        let asset_incentive_index = Decimal::from_ratio(1_u128, 2_u128);

        ASSET_INCENTIVES
            .save(
                deps.as_mut().storage,
                &ma_asset_address,
                &AssetIncentive {
                    emission_per_second: Uint128::zero(),
                    index: asset_incentive_index,
                    last_updated: 500_000,
                },
            )
            .unwrap();

        let info = mock_info("ma_asset", &[]);
        let env = mars_core::testing::mock_env(MockEnvParams {
            block_time: Timestamp::from_seconds(600_000),
            ..Default::default()
        });
        let msg = ExecuteMsg::BalanceChange {
            user_address: Addr::unchecked("user"),
            user_balance_before: Uint128::new(100_000),
            total_supply_before: Uint128::new(100_000),
        };

        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        let expected_accrued_rewards = user_compute_accrued_rewards(
            Uint128::new(100_000),
            Decimal::zero(),
            asset_incentive_index,
        )
        .unwrap();

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "balance_change"),
                attr("ma_asset", "ma_asset"),
                attr("user", "user"),
                attr("rewards_accrued", expected_accrued_rewards),
                attr("asset_index", asset_incentive_index.to_string()),
            ]
        );

        // asset incentive index stays the same
        let asset_incentive = ASSET_INCENTIVES
            .load(deps.as_ref().storage, &ma_asset_address)
            .unwrap();
        assert_eq!(asset_incentive.index, asset_incentive_index);
        assert_eq!(asset_incentive.last_updated, 600_000);

        // user index is set to asset's index
        let user_asset_index = USER_ASSET_INDICES
            .load(deps.as_ref().storage, (&user_address, &ma_asset_address))
            .unwrap();
        assert_eq!(user_asset_index, asset_incentive_index);

        // rewards get updated
        let user_unclaimed_rewards = USER_UNCLAIMED_REWARDS
            .load(deps.as_ref().storage, &user_address)
            .unwrap();
        assert_eq!(user_unclaimed_rewards, expected_accrued_rewards)
    }

    #[test]
    fn test_balance_change_user_with_zero_balance() {
        let mut deps = th_setup(&[]);
        let ma_asset_address = Addr::unchecked("ma_asset");
        let user_address = Addr::unchecked("user");

        let start_index = Decimal::from_ratio(1_u128, 2_u128);
        let emission_per_second = Uint128::new(100);
        let total_supply = Uint128::new(100_000);
        let time_last_updated = 500_000_u64;
        let time_contract_call = 600_000_u64;

        ASSET_INCENTIVES
            .save(
                deps.as_mut().storage,
                &ma_asset_address,
                &AssetIncentive {
                    emission_per_second,
                    index: start_index,
                    last_updated: time_last_updated,
                },
            )
            .unwrap();

        let info = mock_info("ma_asset", &[]);
        let env = mars_core::testing::mock_env(MockEnvParams {
            block_time: Timestamp::from_seconds(time_contract_call),
            ..Default::default()
        });
        let msg = ExecuteMsg::BalanceChange {
            user_address: user_address.clone(),
            user_balance_before: Uint128::zero(),
            total_supply_before: total_supply,
        };

        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        let expected_index = asset_incentive_compute_index(
            start_index,
            emission_per_second,
            total_supply,
            time_last_updated,
            time_contract_call,
        )
        .unwrap();

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "balance_change"),
                attr("ma_asset", "ma_asset"),
                attr("user", "user"),
                attr("rewards_accrued", "0"),
                attr("asset_index", expected_index.to_string()),
            ]
        );

        // asset incentive gets updated
        let asset_incentive = ASSET_INCENTIVES
            .load(deps.as_ref().storage, &ma_asset_address)
            .unwrap();
        assert_eq!(asset_incentive.index, expected_index);
        assert_eq!(asset_incentive.last_updated, time_contract_call);

        // user index is set to asset's index
        let user_asset_index = USER_ASSET_INDICES
            .load(deps.as_ref().storage, (&user_address, &ma_asset_address))
            .unwrap();
        assert_eq!(user_asset_index, expected_index);

        // no new rewards
        let user_unclaimed_rewards = USER_UNCLAIMED_REWARDS
            .may_load(deps.as_ref().storage, &user_address)
            .unwrap();
        assert_eq!(user_unclaimed_rewards, None)
    }

    #[test]
    fn test_with_zero_previous_balance_and_asset_with_zero_index_accumulates_rewards() {
        let mut deps = th_setup(&[]);
        let ma_asset_address = Addr::unchecked("ma_asset");
        let user_address = Addr::unchecked("user");

        let start_index = Decimal::zero();
        let emission_per_second = Uint128::new(100);
        let time_last_updated = 500_000_u64;
        let time_contract_call = 600_000_u64;

        ASSET_INCENTIVES
            .save(
                deps.as_mut().storage,
                &ma_asset_address,
                &AssetIncentive {
                    emission_per_second,
                    index: start_index,
                    last_updated: time_last_updated,
                },
            )
            .unwrap();

        {
            let info = mock_info("ma_asset", &[]);
            let env = mars_core::testing::mock_env(MockEnvParams {
                block_time: Timestamp::from_seconds(time_contract_call),
                ..Default::default()
            });
            let msg = ExecuteMsg::BalanceChange {
                user_address: user_address.clone(),
                user_balance_before: Uint128::zero(),
                total_supply_before: Uint128::zero(),
            };
            // Execute balance changed, this is the first mint of the asset, so previous total
            // supply and user balance is 0
            execute(deps.as_mut(), env, info, msg).unwrap();
        }

        {
            // Some time passes and we query the user rewards, expected value should not be 0
            let user_balance = Uint128::new(100_000);
            let total_supply = Uint128::new(100_000);
            deps.querier
                .set_cw20_total_supply(ma_asset_address.clone(), total_supply);
            deps.querier.set_cw20_balances(
                ma_asset_address.clone(),
                &[(user_address.clone(), user_balance)],
            );
            let env = mars_core::testing::mock_env(MockEnvParams {
                block_time: Timestamp::from_seconds(time_contract_call + 1000),
                ..Default::default()
            });
            let rewards_query =
                query_user_unclaimed_rewards(deps.as_ref(), env, String::from("user")).unwrap();
            assert_eq!(
                Uint128::new(1000).checked_mul(emission_per_second).unwrap(),
                rewards_query
            );
        }
    }

    #[test]
    fn test_set_new_asset_incentive_user_non_zero_balance() {
        let mut deps = th_setup(&[]);
        let user_address = Addr::unchecked("user");

        // set cw20 balance for user
        let ma_asset_address = Addr::unchecked("ma_asset");
        let total_supply = Uint128::new(100_000);
        let user_balance = Uint128::new(10_000);

        deps.querier
            .set_cw20_total_supply(ma_asset_address.clone(), total_supply);
        deps.querier.set_cw20_balances(
            ma_asset_address.clone(),
            &[(user_address.clone(), user_balance)],
        );

        // set asset incentive
        {
            let time_last_updated = 500_000_u64;
            let emission_per_second = Uint128::new(100);
            let asset_incentive_index = Decimal::zero();

            ASSET_INCENTIVES
                .save(
                    deps.as_mut().storage,
                    &ma_asset_address,
                    &AssetIncentive {
                        emission_per_second,
                        index: asset_incentive_index,
                        last_updated: time_last_updated,
                    },
                )
                .unwrap();
        }

        // first query
        {
            let time_contract_call = 600_000_u64;

            let env = mars_core::testing::mock_env(MockEnvParams {
                block_time: Timestamp::from_seconds(time_contract_call),
                ..Default::default()
            });

            let unclaimed_rewards =
                query_user_unclaimed_rewards(deps.as_ref(), env, "user".to_string()).unwrap();
            // 100_000 s * 100 MARS/s * 1/10th cw20 supply
            let expected_unclaimed_rewards = Uint128::new(1_000_000);
            assert_eq!(unclaimed_rewards, expected_unclaimed_rewards);
        }

        // increase user ma_asset balance
        {
            let time_contract_call = 700_000_u64;
            let user_balance = Uint128::new(25_000);

            deps.querier.set_cw20_balances(
                ma_asset_address.clone(),
                &[(user_address.clone(), user_balance)],
            );

            let env = mars_core::testing::mock_env(MockEnvParams {
                block_time: Timestamp::from_seconds(time_contract_call),
                ..Default::default()
            });

            let info = mock_info(&ma_asset_address.to_string(), &[]);

            execute_balance_change(
                deps.as_mut(),
                env,
                info,
                user_address.clone(),
                Uint128::new(10_000),
                total_supply,
            )
            .unwrap();
        }

        // second query
        {
            let time_contract_call = 800_000_u64;

            let env = mars_core::testing::mock_env(MockEnvParams {
                block_time: Timestamp::from_seconds(time_contract_call),
                ..Default::default()
            });

            let unclaimed_rewards =
                query_user_unclaimed_rewards(deps.as_ref(), env, "user".to_string()).unwrap();
            let expected_unclaimed_rewards = Uint128::new(
                // 200_000 s * 100 MARS/s * 1/10th cw20 supply +
                2_000_000 +
                // 100_000 s * 100 MARS/s * 1/4 cw20 supply
                2_500_000,
            );
            assert_eq!(unclaimed_rewards, expected_unclaimed_rewards);
        }
    }

    #[test]
    fn test_balance_change_user_non_zero_balance() {
        let mut deps = th_setup(&[]);
        let ma_asset_address = Addr::unchecked("ma_asset");
        let user_address = Addr::unchecked("user");

        let emission_per_second = Uint128::new(100);
        let total_supply = Uint128::new(100_000);

        let mut expected_asset_incentive_index = Decimal::from_ratio(1_u128, 2_u128);
        let mut expected_time_last_updated = 500_000_u64;
        let mut expected_accumulated_rewards = Uint128::zero();

        ASSET_INCENTIVES
            .save(
                deps.as_mut().storage,
                &ma_asset_address,
                &AssetIncentive {
                    emission_per_second,
                    index: expected_asset_incentive_index,
                    last_updated: expected_time_last_updated,
                },
            )
            .unwrap();

        let info = mock_info("ma_asset", &[]);

        // first call no previous rewards
        {
            let time_contract_call = 600_000_u64;
            let user_balance = Uint128::new(10_000);

            let env = mars_core::testing::mock_env(MockEnvParams {
                block_time: Timestamp::from_seconds(time_contract_call),
                ..Default::default()
            });
            let msg = ExecuteMsg::BalanceChange {
                user_address: user_address.clone(),
                user_balance_before: user_balance,
                total_supply_before: total_supply,
            };
            let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();

            expected_asset_incentive_index = asset_incentive_compute_index(
                expected_asset_incentive_index,
                emission_per_second,
                total_supply,
                expected_time_last_updated,
                time_contract_call,
            )
            .unwrap();

            let expected_accrued_rewards = user_compute_accrued_rewards(
                user_balance,
                Decimal::zero(),
                expected_asset_incentive_index,
            )
            .unwrap();
            assert_eq!(
                res.attributes,
                vec![
                    attr("action", "balance_change"),
                    attr("ma_asset", "ma_asset"),
                    attr("user", "user"),
                    attr("rewards_accrued", expected_accrued_rewards),
                    attr("asset_index", expected_asset_incentive_index.to_string()),
                ]
            );

            // asset incentive gets updated
            expected_time_last_updated = time_contract_call;

            let asset_incentive = ASSET_INCENTIVES
                .load(deps.as_ref().storage, &ma_asset_address)
                .unwrap();
            assert_eq!(asset_incentive.index, expected_asset_incentive_index);
            assert_eq!(asset_incentive.last_updated, expected_time_last_updated);

            // user index is set to asset's index
            let user_asset_index = USER_ASSET_INDICES
                .load(deps.as_ref().storage, (&user_address, &ma_asset_address))
                .unwrap();
            assert_eq!(user_asset_index, expected_asset_incentive_index);

            // user gets new rewards
            let user_unclaimed_rewards = USER_UNCLAIMED_REWARDS
                .load(deps.as_ref().storage, &user_address)
                .unwrap();
            expected_accumulated_rewards += expected_accrued_rewards;
            assert_eq!(user_unclaimed_rewards, expected_accumulated_rewards)
        }

        // Second call accumulates new rewards
        {
            let time_contract_call = 700_000_u64;
            let user_balance = Uint128::new(20_000);

            let env = mars_core::testing::mock_env(MockEnvParams {
                block_time: Timestamp::from_seconds(time_contract_call),
                ..Default::default()
            });
            let msg = ExecuteMsg::BalanceChange {
                user_address: user_address.clone(),
                user_balance_before: user_balance,
                total_supply_before: total_supply,
            };
            let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();

            let previous_user_index = expected_asset_incentive_index;
            expected_asset_incentive_index = asset_incentive_compute_index(
                expected_asset_incentive_index,
                emission_per_second,
                total_supply,
                expected_time_last_updated,
                time_contract_call,
            )
            .unwrap();

            let expected_accrued_rewards = user_compute_accrued_rewards(
                user_balance,
                previous_user_index,
                expected_asset_incentive_index,
            )
            .unwrap();
            assert_eq!(
                res.attributes,
                vec![
                    attr("action", "balance_change"),
                    attr("ma_asset", "ma_asset"),
                    attr("user", "user"),
                    attr("rewards_accrued", expected_accrued_rewards),
                    attr("asset_index", expected_asset_incentive_index.to_string()),
                ]
            );

            // asset incentive gets updated
            expected_time_last_updated = time_contract_call;

            let asset_incentive = ASSET_INCENTIVES
                .load(deps.as_ref().storage, &ma_asset_address)
                .unwrap();
            assert_eq!(asset_incentive.index, expected_asset_incentive_index);
            assert_eq!(asset_incentive.last_updated, expected_time_last_updated);

            // user index is set to asset's index
            let user_asset_index = USER_ASSET_INDICES
                .load(deps.as_ref().storage, (&user_address, &ma_asset_address))
                .unwrap();
            assert_eq!(user_asset_index, expected_asset_incentive_index);

            // user gets new rewards
            let user_unclaimed_rewards = USER_UNCLAIMED_REWARDS
                .load(deps.as_ref().storage, &user_address)
                .unwrap();
            expected_accumulated_rewards += expected_accrued_rewards;
            assert_eq!(user_unclaimed_rewards, expected_accumulated_rewards)
        }

        // Third call same block does not change anything
        {
            let time_contract_call = 700_000_u64;
            let user_balance = Uint128::new(20_000);

            let env = mars_core::testing::mock_env(MockEnvParams {
                block_time: Timestamp::from_seconds(time_contract_call),
                ..Default::default()
            });
            let msg = ExecuteMsg::BalanceChange {
                user_address: user_address.clone(),
                user_balance_before: user_balance,
                total_supply_before: total_supply,
            };
            let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();

            assert_eq!(
                res.attributes,
                vec![
                    attr("action", "balance_change"),
                    attr("ma_asset", "ma_asset"),
                    attr("user", "user"),
                    attr("rewards_accrued", "0"),
                    attr("asset_index", expected_asset_incentive_index.to_string()),
                ]
            );

            // asset incentive is still the same
            let asset_incentive = ASSET_INCENTIVES
                .load(deps.as_ref().storage, &ma_asset_address)
                .unwrap();
            assert_eq!(asset_incentive.index, expected_asset_incentive_index);
            assert_eq!(asset_incentive.last_updated, expected_time_last_updated);

            // user index is still the same
            let user_asset_index = USER_ASSET_INDICES
                .load(deps.as_ref().storage, (&user_address, &ma_asset_address))
                .unwrap();
            assert_eq!(user_asset_index, expected_asset_incentive_index);

            // user gets no new rewards
            let user_unclaimed_rewards = USER_UNCLAIMED_REWARDS
                .load(deps.as_ref().storage, &user_address)
                .unwrap();
            assert_eq!(user_unclaimed_rewards, expected_accumulated_rewards)
        }
    }

    #[test]
    fn test_execute_claim_rewards() {
        // SETUP
        let mut deps = th_setup(&[]);
        let user_address = Addr::unchecked("user");

        let previous_unclaimed_rewards = Uint128::new(50_000);
        let ma_asset_total_supply = Uint128::new(100_000);
        let ma_asset_user_balance = Uint128::new(10_000);
        let ma_zero_total_supply = Uint128::new(200_000);
        let ma_zero_user_balance = Uint128::new(10_000);
        let ma_no_user_total_supply = Uint128::new(100_000);
        let ma_no_user_balance = Uint128::zero();
        let time_start = 500_000_u64;
        let time_contract_call = 600_000_u64;

        // addresses
        // ma_asset with ongoing rewards
        let ma_asset_address = Addr::unchecked("ma_asset");
        // ma_asset with no pending rewards but with user index (so it had active incentives
        // at some point)
        let ma_zero_address = Addr::unchecked("ma_zero");
        // ma_asset where the user never had a balance during an active
        // incentive -> hence no associated index
        let ma_no_user_address = Addr::unchecked("ma_no_user");

        deps.querier
            .set_cw20_total_supply(ma_asset_address.clone(), ma_asset_total_supply);
        deps.querier
            .set_cw20_total_supply(ma_zero_address.clone(), ma_zero_total_supply);
        deps.querier
            .set_cw20_total_supply(ma_no_user_address.clone(), ma_no_user_total_supply);
        deps.querier.set_cw20_balances(
            ma_asset_address.clone(),
            &[(user_address.clone(), ma_asset_user_balance)],
        );
        deps.querier.set_cw20_balances(
            ma_zero_address.clone(),
            &[(user_address.clone(), ma_zero_user_balance)],
        );
        deps.querier.set_cw20_balances(
            ma_no_user_address.clone(),
            &[(user_address.clone(), ma_no_user_balance)],
        );

        // incentives
        ASSET_INCENTIVES
            .save(
                deps.as_mut().storage,
                &ma_asset_address,
                &AssetIncentive {
                    emission_per_second: Uint128::new(100),
                    index: Decimal::one(),
                    last_updated: time_start,
                },
            )
            .unwrap();
        ASSET_INCENTIVES
            .save(
                deps.as_mut().storage,
                &ma_zero_address,
                &AssetIncentive {
                    emission_per_second: Uint128::zero(),
                    index: Decimal::one(),
                    last_updated: time_start,
                },
            )
            .unwrap();
        ASSET_INCENTIVES
            .save(
                deps.as_mut().storage,
                &ma_no_user_address,
                &AssetIncentive {
                    emission_per_second: Uint128::new(200),
                    index: Decimal::one(),
                    last_updated: time_start,
                },
            )
            .unwrap();

        // user indices
        USER_ASSET_INDICES
            .save(
                deps.as_mut().storage,
                (&user_address, &ma_asset_address),
                &Decimal::one(),
            )
            .unwrap();

        USER_ASSET_INDICES
            .save(
                deps.as_mut().storage,
                (&user_address, &ma_zero_address),
                &Decimal::from_ratio(1_u128, 2_u128),
            )
            .unwrap();

        // unclaimed_rewards
        USER_UNCLAIMED_REWARDS
            .save(
                deps.as_mut().storage,
                &user_address,
                &previous_unclaimed_rewards,
            )
            .unwrap();

        let expected_ma_asset_incentive_index = asset_incentive_compute_index(
            Decimal::one(),
            Uint128::new(100),
            ma_asset_total_supply,
            time_start,
            time_contract_call,
        )
        .unwrap();

        let expected_ma_asset_accrued_rewards = user_compute_accrued_rewards(
            ma_asset_user_balance,
            Decimal::one(),
            expected_ma_asset_incentive_index,
        )
        .unwrap();

        let expected_ma_zero_accrued_rewards = user_compute_accrued_rewards(
            ma_zero_user_balance,
            Decimal::from_ratio(1_u128, 2_u128),
            Decimal::one(),
        )
        .unwrap();

        let expected_accrued_rewards = previous_unclaimed_rewards
            + expected_ma_asset_accrued_rewards
            + expected_ma_zero_accrued_rewards;

        // MSG
        let info = mock_info("user", &[]);
        let env = mars_core::testing::mock_env(MockEnvParams {
            block_time: Timestamp::from_seconds(time_contract_call),
            ..Default::default()
        });
        let msg = ExecuteMsg::ClaimRewards {};

        // query a bit before gives less rewards
        let env_before = mars_core::testing::mock_env(MockEnvParams {
            block_time: Timestamp::from_seconds(time_contract_call - 10_000),
            ..Default::default()
        });
        let rewards_query_before =
            query_user_unclaimed_rewards(deps.as_ref(), env_before, String::from("user")).unwrap();
        assert!(rewards_query_before < expected_accrued_rewards);

        // query before execution gives expected rewards
        let rewards_query =
            query_user_unclaimed_rewards(deps.as_ref(), env.clone(), String::from("user")).unwrap();
        assert_eq!(rewards_query, expected_accrued_rewards);

        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        // query after execution gives 0 rewards
        let rewards_query_after =
            query_user_unclaimed_rewards(deps.as_ref(), env, String::from("user")).unwrap();
        assert_eq!(rewards_query_after, Uint128::zero());

        // ASSERT

        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("mars_token"),
                msg: to_binary(&cw20::Cw20ExecuteMsg::Send {
                    contract: String::from("staking"),
                    amount: expected_accrued_rewards,
                    msg: to_binary(&staking::msg::ReceiveMsg::Stake {
                        recipient: Some(user_address.to_string()),
                    })
                    .unwrap()
                })
                .unwrap(),
                funds: vec![],
            }))]
        );

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "claim_rewards"),
                attr("user", "user"),
                attr("mars_staked_as_rewards", expected_accrued_rewards),
            ]
        );

        // ma_asset and ma_zero incentives get updated, ma_no_user does not
        let ma_asset_incentive = ASSET_INCENTIVES
            .load(deps.as_ref().storage, &ma_asset_address)
            .unwrap();
        assert_eq!(ma_asset_incentive.index, expected_ma_asset_incentive_index);
        assert_eq!(ma_asset_incentive.last_updated, time_contract_call);

        let ma_zero_incentive = ASSET_INCENTIVES
            .load(deps.as_ref().storage, &ma_zero_address)
            .unwrap();
        assert_eq!(ma_zero_incentive.index, Decimal::one());
        assert_eq!(ma_zero_incentive.last_updated, time_contract_call);

        let ma_no_user_incentive = ASSET_INCENTIVES
            .load(deps.as_ref().storage, &ma_no_user_address)
            .unwrap();
        assert_eq!(ma_no_user_incentive.index, Decimal::one());
        assert_eq!(ma_no_user_incentive.last_updated, time_start);

        // user's ma_asset and ma_zero indices are updated
        let user_ma_asset_index = USER_ASSET_INDICES
            .load(deps.as_ref().storage, (&user_address, &ma_asset_address))
            .unwrap();
        assert_eq!(user_ma_asset_index, expected_ma_asset_incentive_index);

        let user_ma_zero_index = USER_ASSET_INDICES
            .load(deps.as_ref().storage, (&user_address, &ma_zero_address))
            .unwrap();
        assert_eq!(user_ma_zero_index, Decimal::one());

        // user's ma_no_user does not get updated
        let user_ma_no_user_index = USER_ASSET_INDICES
            .may_load(deps.as_ref().storage, (&user_address, &ma_no_user_address))
            .unwrap();
        assert_eq!(user_ma_no_user_index, None);

        // user rewards are cleared
        let user_unclaimed_rewards = USER_UNCLAIMED_REWARDS
            .load(deps.as_ref().storage, &user_address)
            .unwrap();
        assert_eq!(user_unclaimed_rewards, Uint128::zero())
    }

    #[test]
    fn test_claim_zero_rewards() {
        // SETUP
        let mut deps = th_setup(&[]);

        let info = mock_info("user", &[]);
        let msg = ExecuteMsg::ClaimRewards {};

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "claim_rewards"),
                attr("user", "user"),
                attr("mars_staked_as_rewards", "0"),
            ]
        );
    }

    #[test]
    fn test_update_config() {
        let mut deps = th_setup(&[]);

        // *
        // non owner is not authorized
        // *
        let msg = ExecuteMsg::UpdateConfig {
            owner: None,
            address_provider_address: None,
        };
        let info = mock_info("somebody", &[]);
        let error_res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(error_res, ContractError::Mars(MarsError::Unauthorized {}));

        // *
        // update config with new params
        // *
        let msg = ExecuteMsg::UpdateConfig {
            owner: Some(String::from("new_owner")),
            address_provider_address: None,
        };
        let info = mock_info("owner", &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Read config from state
        let new_config = CONFIG.load(deps.as_ref().storage).unwrap();
        assert_eq!(new_config.owner, Addr::unchecked("new_owner"));
        assert_eq!(
            new_config.address_provider_address,
            Addr::unchecked("address_provider") // should not change
        );
    }

    #[test]
    fn test_execute_cosmos_msg() {
        let mut deps = th_setup(&[]);

        let bank = BankMsg::Send {
            to_address: "destination".to_string(),
            amount: vec![Coin {
                denom: "uluna".to_string(),
                amount: Uint128::new(123456u128),
            }],
        };
        let cosmos_msg = CosmosMsg::Bank(bank);
        let msg = ExecuteMsg::ExecuteCosmosMsg(cosmos_msg.clone());

        // *
        // non owner is not authorized
        // *
        let info = mock_info("somebody", &[]);
        let error_res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
        assert_eq!(error_res, ContractError::Mars(MarsError::Unauthorized {}));

        // *
        // can execute Cosmos msg
        // *
        let info = mock_info("owner", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages, vec![SubMsg::new(cosmos_msg)]);
        assert_eq!(res.attributes, vec![attr("action", "execute_cosmos_msg")]);
    }

    #[test]
    fn test_asset_incentive_compute_index() {
        assert_eq!(
            asset_incentive_compute_index(
                Decimal::zero(),
                Uint128::new(100),
                Uint128::new(200_000),
                1000,
                10
            ),
            Err(StdError::overflow(OverflowError::new(
                OverflowOperation::Sub,
                1000,
                10
            )))
        );

        assert_eq!(
            asset_incentive_compute_index(
                Decimal::zero(),
                Uint128::new(100),
                Uint128::new(200_000),
                0,
                1000
            )
            .unwrap(),
            Decimal::from_ratio(1_u128, 2_u128)
        );
        assert_eq!(
            asset_incentive_compute_index(
                Decimal::from_ratio(1_u128, 2_u128),
                Uint128::new(2000),
                Uint128::new(5_000_000),
                20_000,
                30_000
            )
            .unwrap(),
            Decimal::from_ratio(9_u128, 2_u128)
        );
    }

    #[test]
    fn test_user_compute_accrued_rewards() {
        assert_eq!(
            user_compute_accrued_rewards(
                Uint128::zero(),
                Decimal::one(),
                Decimal::from_ratio(2_u128, 1_u128)
            )
            .unwrap(),
            Uint128::zero()
        );

        assert_eq!(
            user_compute_accrued_rewards(
                Uint128::new(100),
                Decimal::zero(),
                Decimal::from_ratio(2_u128, 1_u128)
            )
            .unwrap(),
            Uint128::new(200)
        );
        assert_eq!(
            user_compute_accrued_rewards(
                Uint128::new(100),
                Decimal::one(),
                Decimal::from_ratio(2_u128, 1_u128)
            )
            .unwrap(),
            Uint128::new(100)
        );
    }

    // TEST HELPERS
    fn th_setup(contract_balances: &[Coin]) -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier> {
        let mut deps = mock_dependencies(contract_balances);

        let msg = InstantiateMsg {
            owner: String::from("owner"),
            address_provider_address: String::from("address_provider"),
        };
        let info = mock_info("owner", &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        deps
    }
}
