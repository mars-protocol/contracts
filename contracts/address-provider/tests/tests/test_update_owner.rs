use cosmwasm_std::testing::{mock_env, mock_info};
use mars_address_provider::{contract::execute, error::ContractError};
use mars_owner::{OwnerError::NotOwner, OwnerUpdate};
use mars_types::address_provider::{ConfigResponse, ExecuteMsg, QueryMsg};

use super::helpers::{th_query, th_setup};

#[test]
fn initialized_state() {
    let deps = th_setup();

    let config: ConfigResponse = th_query(deps.as_ref(), QueryMsg::Config {});
    assert!(config.owner.is_some());
    assert!(config.proposed_new_owner.is_none());
}

#[test]
fn update_owner() {
    let mut deps = th_setup();

    let original_config: ConfigResponse = th_query(deps.as_ref(), QueryMsg::Config {});

    let new_owner = "new_admin";

    // only owner can propose new owners
    let bad_guy = "bad_guy";
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(bad_guy, &[]),
        ExecuteMsg::UpdateOwner(OwnerUpdate::ProposeNewOwner {
            proposed: bad_guy.to_string(),
        }),
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Owner(NotOwner {}));

    // propose new owner
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(&original_config.owner.clone().unwrap(), &[]),
        ExecuteMsg::UpdateOwner(OwnerUpdate::ProposeNewOwner {
            proposed: new_owner.to_string(),
        }),
    )
    .unwrap();
    let new_config: ConfigResponse = th_query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(new_config.owner.unwrap(), original_config.owner.clone().unwrap());
    assert_ne!(new_config.proposed_new_owner, original_config.proposed_new_owner);
    assert_eq!(new_config.proposed_new_owner.unwrap(), new_owner.to_string());

    // accept ownership
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(new_owner, &[]),
        ExecuteMsg::UpdateOwner(OwnerUpdate::AcceptProposed),
    )
    .unwrap();
    let new_config: ConfigResponse = th_query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(new_config.owner.clone().unwrap(), new_owner.to_string());
    assert_ne!(new_config.owner, original_config.owner);
    assert_eq!(new_config.proposed_new_owner, None);
}
