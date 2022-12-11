use mars_outpost::oracle::{ConfigResponse, QueryMsg};

mod helpers;

#[test]
fn test_instantiating() {
    let deps = helpers::setup_test();

    let cfg: ConfigResponse = helpers::query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(cfg.owner.unwrap(), "owner".to_string());
    assert_eq!(cfg.proposed_new_owner, None);
    assert_eq!(cfg.base_denom, "uosmo".to_string());
}
