use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use mars_address_provider::{contract::instantiate, error::ContractError};
use mars_types::address_provider::{ConfigResponse, InstantiateMsg, QueryMsg};

use super::helpers::th_query;

#[test]
fn invalid_chain_prefix() {
    let mut deps = mock_dependencies();

    let err = instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info("deployer", &[]),
        InstantiateMsg {
            owner: "osmo_owner".to_string(),
            prefix: "mars".to_string(),
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::InvalidChainPrefix("mars".to_string()));
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();

    instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info("deployer", &[]),
        InstantiateMsg {
            owner: "osmo1_owner".to_string(),
            prefix: "osmo".to_string(),
        },
    )
    .unwrap();

    let config: ConfigResponse = th_query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(config.owner, Some("osmo1_owner".to_string()));
    assert_eq!(config.proposed_new_owner, None);
    assert_eq!(config.prefix, "osmo".to_string());
}
