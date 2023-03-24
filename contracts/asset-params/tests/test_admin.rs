use cosmwasm_std::{Addr, Decimal, from_binary};
use mars_owner::OwnerError::{NotOwner, NotProposedOwner, StateTransitionError};
use mars_owner::OwnerUpdate;
use mars_testing::{mock_dependencies, mock_env, mock_env_at_block_time, mock_info, MockEnvParams};
use mars_testing::integration::mock_env::MockEnv;
use mars_params::contract::{execute, instantiate, query};
use mars_params::error::{ContractError, ValidationError};
use mars_params::error::ContractError::Owner;
use mars_params::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use mars_params::types::ConfigResponse;

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    // *
    // init config with close_factor greater than 1
    // *
    let mut close_factor = Decimal::from_ratio(13u128, 10u128);
    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        emergency_owner: "emergency_owner".to_string(),
        close_factor
    };

    let info = mock_info("owner");
    let error_res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap_err();
    assert_eq!(
        error_res,
        ValidationError::InvalidParam {
            param_name: "close_factor".to_string(),
            invalid_value: "1.3".to_string(),
            predicate: "<= 1".to_string(),
        }
            .into()
    );

    // *
    // init config with valid params
    // *
    close_factor = Decimal::from_ratio(1u128, 2u128);
    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        emergency_owner: "emergency_owner".to_string(),
        close_factor
    };

    let info = mock_info("owner");
    let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    let res = query(deps.as_ref(), QueryMsg::Config {}).unwrap();
    let value: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(value.owner.unwrap(), "owner");
    assert_eq!(value.emergency_owner.unwrap(), "emergency_owner");
}

#[test]
fn update_close_factor() {
    let mut deps = mock_dependencies(&[]);
    // *
    // init config with valid params
    // *
    let mut close_factor = Decimal::from_ratio(1u128, 4u128);
    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        emergency_owner: "emergency_owner".to_string(),
        close_factor
    };

    let info = mock_info("owner");
    let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // *
    // non owner is not authorized
    // *
    let msg = ExecuteMsg::UpdateCloseFactor {
        close_factor,
    };
    let info = mock_info("somebody");
    let error_res = execute(deps.as_mut(), info, msg).unwrap_err();
    assert_eq!(error_res, ContractError::Owner(NotOwner {}));
}

#[test]
fn propose_new_owner() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();

    let new_owner = "new_owner".to_string();

    // only owner can propose new owners
    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.update_owner(
        &bad_guy,
        OwnerUpdate::ProposeNewOwner {
            proposed: bad_guy.to_string(),
        },
    );
    assert_err(res, Owner(NotOwner {}));

    mock.update_owner(
        &Addr::unchecked(original_config.owner.clone().unwrap()),
        OwnerUpdate::ProposeNewOwner {
            proposed: new_owner.clone(),
        },
    )
        .unwrap();

    let new_config = mock.query_config();

    assert_eq!(new_config.owner, original_config.owner);
    assert_ne!(new_config.proposed_new_owner, original_config.proposed_new_owner);
    assert_eq!(new_config.proposed_new_owner, Some(new_owner));
}

#[test]
fn clear_proposed() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();

    let new_owner = "new_owner".to_string();

    mock.update_owner(
        &Addr::unchecked(original_config.owner.clone().unwrap()),
        OwnerUpdate::ProposeNewOwner {
            proposed: new_owner.clone(),
        },
    )
        .unwrap();

    let interim_config = mock.query_config();

    assert_eq!(interim_config.proposed_new_owner, Some(new_owner));

    // only owner can clear
    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.update_owner(&bad_guy, OwnerUpdate::ClearProposed);
    assert_err(res, Owner(NotOwner {}));

    mock.update_owner(
        &Addr::unchecked(original_config.owner.clone().unwrap()),
        OwnerUpdate::ClearProposed,
    )
        .unwrap();

    let latest_config = mock.query_config();

    assert_eq!(latest_config.owner, original_config.owner);
    assert_ne!(latest_config.proposed_new_owner, interim_config.proposed_new_owner);
    assert_eq!(latest_config.proposed_new_owner, None);
}

#[test]
fn accept_owner_role() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();

    let new_owner = "new_owner".to_string();

    mock.update_owner(
        &Addr::unchecked(original_config.owner.clone().unwrap()),
        OwnerUpdate::ProposeNewOwner {
            proposed: new_owner.clone(),
        },
    )
        .unwrap();

    // Only proposed owner can accept
    let res = mock.update_owner(
        &Addr::unchecked(original_config.owner.unwrap()),
        OwnerUpdate::AcceptProposed,
    );
    assert_err(res, Owner(NotProposedOwner {}));

    mock.update_owner(&Addr::unchecked(new_owner.clone()), OwnerUpdate::AcceptProposed).unwrap();

    let new_config = mock.query_config();

    assert_eq!(new_config.owner.unwrap(), new_owner);
    assert_eq!(new_config.proposed_new_owner, None);
}

#[test]
fn abolish_owner_role() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();

    // Only owner can abolish role
    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.update_owner(&bad_guy, OwnerUpdate::AbolishOwnerRole);
    assert_err(res, Owner(NotOwner {}));

    mock.update_owner(
        &Addr::unchecked(original_config.owner.clone().unwrap()),
        OwnerUpdate::AbolishOwnerRole,
    )
        .unwrap();

    let new_config = mock.query_config();

    assert_eq!(new_config.owner, None);
    assert_eq!(new_config.proposed_new_owner, None);

    // No new updates can occur
    let res = mock.update_owner(
        &Addr::unchecked(original_config.owner.clone().unwrap()),
        OwnerUpdate::ProposeNewOwner {
            proposed: original_config.owner.unwrap(),
        },
    );
    assert_err(res, Owner(StateTransitionError {}));
}
