use cosmwasm_std::testing::{mock_env, mock_info};
use mars_address_provider::contract::execute;
use mars_address_provider::error::ContractError;
use mars_address_provider::state::CONFIG;

use mars_outpost::address_provider::{Config, ExecuteMsg, QueryMsg};

use crate::helpers::{th_query, th_setup};

mod helpers;

#[test]
fn test_proper_initialization() {
    let deps = th_setup();

    let config: Config = th_query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(config.owner, "owner".to_string());
}

#[test]
fn test_transferring_ownership() {
    let mut deps = th_setup();

    let msg = ExecuteMsg::TransferOwnership {
        new_owner: "larry".to_string(),
    };

    // non-owner cannot transfer ownership
    let err = execute(deps.as_mut(), mock_env(), mock_info("jake", &[]), msg.clone()).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized);

    // owner can transfer ownership
    execute(deps.as_mut(), mock_env(), mock_info("owner", &[]), msg).unwrap();

    let config = CONFIG.load(deps.as_ref().storage).unwrap();
    assert_eq!(config.owner, "larry".to_string());
}
