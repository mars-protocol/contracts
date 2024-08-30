use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use mars_burn_contract::contract::instantiate;
use mars_types::burn::InstantiateMsg;

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();
    let res = instantiate(deps.as_mut(), mock_env(), mock_info("deployer", &[]), InstantiateMsg {})
        .unwrap();
    assert_eq!(0, res.messages.len());
}
