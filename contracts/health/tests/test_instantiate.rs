use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use mars_rover_health::{
    contract::instantiate,
    state::{CREDIT_MANAGER, OWNER},
};
use mars_rover_health_types::InstantiateMsg;

pub mod helpers;

#[test]
fn instantiate_without_credit_manager() {
    let mut deps = mock_dependencies();

    instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info("deployer", &[]),
        InstantiateMsg {
            owner: "owner".to_string(),
            credit_manager: None,
        },
    )
    .unwrap();

    let o = OWNER.query(deps.as_ref().storage).unwrap();
    assert_eq!(o.owner.unwrap(), "owner".to_string());
    assert!(o.proposed.is_none());
    assert!(o.initialized);
    assert!(!o.abolished);
    assert!(o.emergency_owner.is_none());

    let cm = CREDIT_MANAGER.may_load(deps.as_ref().storage).unwrap();
    assert_eq!(cm, None);
}

#[test]
fn instantiate_with_credit_manager() {
    let mut deps = mock_dependencies();

    instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info("deployer", &[]),
        InstantiateMsg {
            owner: "owner".to_string(),
            credit_manager: Some("credit_manager_1234".to_string()),
        },
    )
    .unwrap();

    let o = OWNER.query(deps.as_ref().storage).unwrap();
    assert_eq!(o.owner.unwrap(), "owner".to_string());
    assert!(o.proposed.is_none());
    assert!(o.initialized);
    assert!(!o.abolished);
    assert!(o.emergency_owner.is_none());

    let cm = CREDIT_MANAGER.may_load(deps.as_ref().storage).unwrap();
    assert_eq!(cm.unwrap(), "credit_manager_1234".to_string());
}
