use cosmwasm_std::testing::{mock_env, mock_info};
use mars_oracle::msg::{ConfigResponse, ExecuteMsg, QueryMsg};
use mars_oracle_base::ContractError;
use mars_oracle_osmosis::contract::entry::execute;
use mars_owner::{OwnerError::NotOwner, OwnerUpdate};

use crate::helpers::{query, setup_test_with_pools};

mod helpers;

#[test]
fn initialized_state() {
    let deps = setup_test_with_pools();

    let config: ConfigResponse = query(deps.as_ref(), QueryMsg::Config {});
    assert!(config.owner.is_some());
    assert!(config.proposed_new_owner.is_none());
}

#[test]
fn update_owner() {
    let mut deps = setup_test_with_pools();

    let original_config: ConfigResponse = query(deps.as_ref(), QueryMsg::Config {});

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
    let new_config: ConfigResponse = query(deps.as_ref(), QueryMsg::Config {});
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
    let new_config: ConfigResponse = query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(new_config.owner.clone().unwrap(), new_owner.to_string());
    assert_ne!(new_config.owner, original_config.owner);
    assert_eq!(new_config.proposed_new_owner, None);
}
