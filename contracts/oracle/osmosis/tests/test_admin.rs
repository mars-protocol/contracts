use cosmwasm_std::testing::mock_env;

use mars_outpost::error::MarsError;
use mars_outpost::oracle::{Config, QueryMsg};
use mars_testing::mock_info;

use mars_oracle_osmosis::contract::entry::execute;
use mars_oracle_osmosis::msg::ExecuteMsg;

mod helpers;

#[test]
fn test_instantiating() {
    let deps = helpers::setup_test();

    let cfg: Config<String> = helpers::query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(cfg.owner, "owner".to_string());
    assert_eq!(cfg.base_denom, "uosmo".to_string());
}

#[test]
fn test_updating_config() {
    let mut deps = helpers::setup_test();

    let msg = ExecuteMsg::UpdateConfig {
        owner: Some("new_owner".to_string()),
    };

    // non-owner cannot update
    let err = execute(deps.as_mut(), mock_env(), mock_info("jake"), msg.clone()).unwrap_err();
    assert_eq!(err, MarsError::Unauthorized {}.into());

    // owner can update
    let res = execute(deps.as_mut(), mock_env(), mock_info("owner"), msg).unwrap();
    assert_eq!(res.messages.len(), 0);

    let cfg: Config<String> = helpers::query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(cfg.owner, "new_owner".to_string());
}
