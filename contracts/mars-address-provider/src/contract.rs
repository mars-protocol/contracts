use std::convert::TryInto;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};

use cw_storage_plus::Bound;
use mars_outpost::address_provider::{
    AddressResponseItem, Config, ExecuteMsg, InstantiateMsg, MarsContract, QueryMsg,
};

use crate::error::ContractError;
use crate::helpers::{assert_owner, assert_valid_addr};
use crate::key::MarsContractKey;
use crate::state::{CONFIG, CONTRACTS};

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

// INIT

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    assert_valid_addr(deps.api, &msg.owner, &msg.prefix)?;

    CONFIG.save(deps.storage, &msg)?;

    Ok(Response::default())
}

// EXECUTE

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SetAddress {
            contract,
            address,
        } => set_address(deps, info.sender, contract, address),
        ExecuteMsg::TransferOwnership {
            new_owner,
        } => transfer_ownership(deps, info.sender, new_owner),
    }
}

pub fn set_address(
    deps: DepsMut,
    sender: Addr,
    contract: MarsContract,
    address: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    assert_owner(&sender, &config.owner)?;
    assert_valid_addr(deps.api, &address, &config.prefix)?;

    CONTRACTS.save(deps.storage, contract.into(), &address)?;

    Ok(Response::new()
        .add_attribute("action", "mars-address-provider/address_set")
        .add_attribute("contract", contract.to_string())
        .add_attribute("address", address))
}

pub fn transfer_ownership(
    deps: DepsMut,
    sender: Addr,
    new_owner: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    assert_owner(&sender, &config.owner)?;

    config.owner = new_owner;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "mars-address-provider/ownership_transferred")
        .add_attribute("previous_owner", sender)
        .add_attribute("new_owner", config.owner))
}

// QUERIES

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Address(contract) => to_binary(&query_address(deps, contract)?),
        QueryMsg::Addresses(contracts) => to_binary(&query_addresses(deps, contracts)?),
        QueryMsg::AllAddresses {
            start_after,
            limit,
        } => to_binary(&query_all_addresses(deps, start_after, limit)?),
    }
}

fn query_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

fn query_address(deps: Deps, contract: MarsContract) -> StdResult<AddressResponseItem> {
    Ok(AddressResponseItem {
        contract,
        address: CONTRACTS.load(deps.storage, contract.into())?,
    })
}

fn query_addresses(
    deps: Deps,
    contracts: Vec<MarsContract>,
) -> StdResult<Vec<AddressResponseItem>> {
    contracts
        .into_iter()
        .map(|contract| query_address(deps, contract))
        .collect::<StdResult<Vec<_>>>()
}

fn query_all_addresses(
    deps: Deps,
    start_after: Option<MarsContract>,
    limit: Option<u32>,
) -> StdResult<Vec<AddressResponseItem>> {
    let start = start_after.map(MarsContractKey::from).map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    CONTRACTS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, v) = item?;
            Ok(AddressResponseItem {
                contract: k.try_into()?,
                address: v,
            })
        })
        .collect()
}

// TESTS

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{
        mock_dependencies_with_balance, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
    };
    use cosmwasm_std::{from_binary, OwnedDeps};

    #[test]
    fn test_proper_initialization() {
        let mut deps = th_setup();

        let msg = InstantiateMsg {
            owner: "owner".to_string(),
            prefix: "osmo".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), mock_info("deployer", &[]), msg).unwrap();

        let config = CONFIG.load(&deps.storage).unwrap();
        assert_eq!(config.owner, "owner".to_string());
    }

    #[test]
    fn test_set_address() {
        let mut deps = th_setup();

        let msg = ExecuteMsg::SetAddress {
            contract: MarsContract::RedBank,
            address: "red_bank".to_string(),
        };

        // non-owner cannot set address
        let err =
            execute(deps.as_mut(), mock_env(), mock_info("jake", &[]), msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized);

        // owner can set address
        execute(deps.as_mut(), mock_env(), mock_info("owner", &[]), msg).unwrap();

        let address = CONTRACTS.load(deps.as_ref().storage, MarsContract::RedBank.into()).unwrap();
        assert_eq!(address, "red_bank".to_string());
    }

    #[test]
    fn test_transfer_ownership() {
        let mut deps = th_setup();

        let msg = ExecuteMsg::TransferOwnership {
            new_owner: "larry".to_string(),
        };

        // non-owner cannot transfer ownership
        let err =
            execute(deps.as_mut(), mock_env(), mock_info("jake", &[]), msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized);

        // owner can transfer ownership
        execute(deps.as_mut(), mock_env(), mock_info("owner", &[]), msg).unwrap();

        let config = CONFIG.load(deps.as_ref().storage).unwrap();
        assert_eq!(config.owner, "larry".to_string());
    }

    #[test]
    fn test_address_queries() {
        let mut deps = th_setup();

        CONTRACTS
            .save(deps.as_mut().storage, MarsContract::Incentives.into(), &"incentives".to_string())
            .unwrap();
        CONTRACTS
            .save(deps.as_mut().storage, MarsContract::Oracle.into(), &"oracle".to_string())
            .unwrap();
        CONTRACTS
            .save(deps.as_mut().storage, MarsContract::RedBank.into(), &"red_bank".to_string())
            .unwrap();

        let res: AddressResponseItem =
            th_query(deps.as_ref(), QueryMsg::Address(MarsContract::Incentives));
        assert_eq!(
            res,
            AddressResponseItem {
                contract: MarsContract::Incentives,
                address: "incentives".to_string()
            }
        );

        let res: Vec<AddressResponseItem> = th_query(
            deps.as_ref(),
            QueryMsg::Addresses(vec![MarsContract::Incentives, MarsContract::RedBank]),
        );
        assert_eq!(
            res,
            vec![
                AddressResponseItem {
                    contract: MarsContract::Incentives,
                    address: "incentives".to_string()
                },
                AddressResponseItem {
                    contract: MarsContract::RedBank,
                    address: "red_bank".to_string()
                }
            ]
        );

        let res: Vec<AddressResponseItem> = th_query(
            deps.as_ref(),
            QueryMsg::AllAddresses {
                start_after: None,
                limit: Some(2),
            },
        );
        assert_eq!(
            res,
            vec![
                AddressResponseItem {
                    contract: MarsContract::Incentives,
                    address: "incentives".to_string()
                },
                AddressResponseItem {
                    contract: MarsContract::Oracle,
                    address: "oracle".to_string()
                }
            ]
        );

        let res: Vec<AddressResponseItem> = th_query(
            deps.as_ref(),
            QueryMsg::AllAddresses {
                start_after: Some(MarsContract::Oracle),
                limit: None,
            },
        );
        assert_eq!(
            res,
            vec![AddressResponseItem {
                contract: MarsContract::RedBank,
                address: "red_bank".to_string()
            }]
        );
    }

    fn th_setup() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies_with_balance(&[]);

        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("deployer", &[]),
            InstantiateMsg {
                owner: "owner".to_string(),
                prefix: "osmo".to_string(),
            },
        )
        .unwrap();

        deps
    }

    fn th_query<T: serde::de::DeserializeOwned>(deps: Deps, msg: QueryMsg) -> T {
        from_binary(&query(deps, mock_env(), msg).unwrap()).unwrap()
    }
}
