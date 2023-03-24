use cosmwasm_std::testing::{mock_env, mock_info};
use mars_owner::{OwnerError::NotOwner, OwnerUpdate};
use mars_red_bank::{contract::execute, error::ContractError};
use mars_red_bank_types::red_bank::{ConfigResponse, ExecuteMsg, QueryMsg};

use crate::helpers::{th_query, th_setup};

mod helpers;

#[test]
fn initialized_state() {
    let deps = th_setup(&[]);

    let config: ConfigResponse = th_query(deps.as_ref(), QueryMsg::Config {});
    assert!(config.emergency_owner.is_none());
}

#[test]
fn only_owner_can_set_emergency_owner() {
    let mut deps = th_setup(&[]);

    // only admin can propose new admins
    let bad_guy = "bad_guy";
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(bad_guy, &[]),
        ExecuteMsg::UpdateOwner(OwnerUpdate::SetEmergencyOwner {
            emergency_owner: "new_emergency_owner".to_string(),
        }),
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Owner(NotOwner {}));
}

#[test]
fn set_and_clear_emergency_owner() {
    let mut deps = th_setup(&[]);

    let original_config: ConfigResponse = th_query(deps.as_ref(), QueryMsg::Config {});

    let emergency_owner = "new_emergency_owner";

    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(&original_config.owner.clone().unwrap(), &[]),
        ExecuteMsg::UpdateOwner(OwnerUpdate::SetEmergencyOwner {
            emergency_owner: emergency_owner.to_string(),
        }),
    )
    .unwrap();

    let new_config: ConfigResponse = th_query(deps.as_ref(), QueryMsg::Config {});

    assert_eq!(new_config.owner, original_config.owner);
    assert_eq!(new_config.proposed_new_owner, original_config.proposed_new_owner);
    assert_eq!(new_config.emergency_owner, Some(emergency_owner.to_string()));

    // clear emergency owner

    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(&original_config.owner.clone().unwrap(), &[]),
        ExecuteMsg::UpdateOwner(OwnerUpdate::ClearEmergencyOwner {}),
    )
    .unwrap();

    let new_config: ConfigResponse = th_query(deps.as_ref(), QueryMsg::Config {});

    assert_eq!(new_config.owner, original_config.owner);
    assert_eq!(new_config.proposed_new_owner, original_config.proposed_new_owner);
    assert_eq!(new_config.emergency_owner, None);
}
