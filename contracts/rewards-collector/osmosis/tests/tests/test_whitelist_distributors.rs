use cosmwasm_std::{testing::mock_env, Uint128};
use mars_owner::OwnerError::NotOwner;
use mars_rewards_collector_base::ContractError;
use mars_rewards_collector_osmosis::entry::execute;
use mars_testing::mock_info;
use mars_types::rewards_collector::{
    ConfigResponse, ExecuteMsg, QueryMsg, UpdateConfig, WhitelistAction,
};

use super::helpers;

#[test]
fn owner_can_add_to_whitelist() {
    let mut deps = helpers::setup_test();
    // Owner adds alice
    let info = mock_info("owner");
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: UpdateConfig {
            whitelist_actions: Some(vec![WhitelistAction::AddAddress {
                address: "alice".to_string(),
            }]),
            ..Default::default()
        },
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let cfg: ConfigResponse = helpers::query(deps.as_ref(), QueryMsg::Config {});
    assert!(cfg.whitelisted_distributors.contains(&"alice".to_string()));
}

#[test]
fn non_owner_cannot_add_to_whitelist() {
    let mut deps = helpers::setup_test();
    let info = mock_info("not_owner");
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: UpdateConfig {
            whitelist_actions: Some(vec![WhitelistAction::AddAddress {
                address: "alice".to_string(),
            }]),
            ..Default::default()
        },
    };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(err, ContractError::Owner(NotOwner {}));
}

#[test]
fn owner_can_remove_from_whitelist() {
    let mut deps = helpers::setup_test();
    // Owner adds alice
    let info = mock_info("owner");
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: UpdateConfig {
            whitelist_actions: Some(vec![WhitelistAction::AddAddress {
                address: "alice".to_string(),
            }]),
            ..Default::default()
        },
    };
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    // Owner removes alice
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: UpdateConfig {
            whitelist_actions: Some(vec![WhitelistAction::RemoveAddress {
                address: "alice".to_string(),
            }]),
            ..Default::default()
        },
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let cfg: ConfigResponse = helpers::query(deps.as_ref(), QueryMsg::Config {});
    assert!(!cfg.whitelisted_distributors.contains(&"alice".to_string()));
}

#[test]
fn whitelisted_can_distribute_rewards() {
    let mut deps = helpers::setup_test();
    // Owner adds alice
    let info = mock_info("owner");
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: UpdateConfig {
            whitelist_actions: Some(vec![WhitelistAction::AddAddress {
                address: "alice".to_string(),
            }]),
            ..Default::default()
        },
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    deps.querier.set_contract_balances(&[cosmwasm_std::coin(1000, "umars")]);
    // Alice can distribute
    let info = mock_info("alice");
    let msg = ExecuteMsg::DistributeRewards {
        denom: "umars".to_string(),
    };
    let result = execute(deps.as_mut(), mock_env(), info, msg);
    assert!(result.is_ok());
}

#[test]
fn owner_can_distribute_rewards() {
    let mut deps = helpers::setup_test();
    deps.querier.set_contract_balances(&[cosmwasm_std::coin(1000, "umars")]);
    let info = mock_info("owner");
    let msg = ExecuteMsg::DistributeRewards {
        denom: "umars".to_string(),
    };
    let result = execute(deps.as_mut(), mock_env(), info, msg);
    assert!(result.is_ok());
}

#[test]
fn non_whitelisted_cannot_distribute_rewards() {
    let mut deps = helpers::setup_test();
    deps.querier.set_contract_balances(&[cosmwasm_std::coin(1000, "umars")]);
    let info = mock_info("bob");
    let msg = ExecuteMsg::DistributeRewards {
        denom: "umars".to_string(),
    };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert!(matches!(
        err,
        ContractError::UnauthorizedDistributor {
            sender: _
        }
    ));
    if let ContractError::UnauthorizedDistributor {
        sender,
    } = err
    {
        assert_eq!(sender, "bob");
    }
}

#[test]
fn removed_account_cannot_distribute_rewards() {
    let mut deps = helpers::setup_test();
    // Owner adds alice
    let info = mock_info("owner");
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: UpdateConfig {
            whitelist_actions: Some(vec![WhitelistAction::AddAddress {
                address: "alice".to_string(),
            }]),
            ..Default::default()
        },
    };
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    // Owner removes alice
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: UpdateConfig {
            whitelist_actions: Some(vec![WhitelistAction::RemoveAddress {
                address: "alice".to_string(),
            }]),
            ..Default::default()
        },
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    // Alice can no longer distribute
    let info = mock_info("alice");
    let msg = ExecuteMsg::DistributeRewards {
        denom: "umars".to_string(),
    };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert!(matches!(
        err,
        ContractError::UnauthorizedDistributor {
            sender: _
        }
    ));
}

#[test]
fn whitelisted_can_swap_asset() {
    let mut deps = helpers::setup_test();
    // Owner adds alice
    let info = mock_info("owner");
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: UpdateConfig {
            whitelist_actions: Some(vec![WhitelistAction::AddAddress {
                address: "alice".to_string(),
            }]),
            ..Default::default()
        },
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    deps.querier.set_contract_balances(&[cosmwasm_std::coin(1000, "umars")]);
    // Alice can swap
    let info = mock_info("alice");
    let msg = ExecuteMsg::SwapAsset {
        denom: "umars".to_string(),
        amount: None,
        safety_fund_route: None,
        fee_collector_route: None,
        safety_fund_min_receive: Some(Uint128::from(1000u128)),

        fee_collector_min_receive: None,
    };
    let result = execute(deps.as_mut(), mock_env(), info, msg);
    assert!(result.is_ok());
}

#[test]
fn non_whitelisted_cannot_swap_asset() {
    let mut deps = helpers::setup_test();
    deps.querier.set_contract_balances(&[cosmwasm_std::coin(1000, "umars")]);
    let info = mock_info("bob");
    let msg = ExecuteMsg::SwapAsset {
        denom: "umars".to_string(),
        amount: None,
        safety_fund_route: None,
        fee_collector_route: None,
        safety_fund_min_receive: None,
        fee_collector_min_receive: None,
    };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert!(matches!(
        err,
        ContractError::UnauthorizedDistributor {
            sender: _
        }
    ));
    if let ContractError::UnauthorizedDistributor {
        sender,
    } = err
    {
        assert_eq!(sender, "bob");
    }
}

#[test]
fn removed_account_cannot_swap_asset() {
    let mut deps = helpers::setup_test();
    // Owner adds alice
    let info = mock_info("owner");
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: UpdateConfig {
            whitelist_actions: Some(vec![WhitelistAction::AddAddress {
                address: "alice".to_string(),
            }]),
            ..Default::default()
        },
    };
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    // Owner removes alice
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: UpdateConfig {
            whitelist_actions: Some(vec![WhitelistAction::RemoveAddress {
                address: "alice".to_string(),
            }]),
            ..Default::default()
        },
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    // Alice can no longer swap
    let info = mock_info("alice");
    let msg = ExecuteMsg::SwapAsset {
        denom: "umars".to_string(),
        amount: None,
        safety_fund_route: None,
        fee_collector_route: None,
        safety_fund_min_receive: Some(Uint128::from(1000u128)),
        fee_collector_min_receive: None,
    };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert!(matches!(
        err,
        ContractError::UnauthorizedDistributor {
            sender: _
        }
    ));
}

#[test]
fn owner_can_swap_asset() {
    let mut deps = helpers::setup_test();
    deps.querier.set_contract_balances(&[cosmwasm_std::coin(1000, "umars")]);
    let info = mock_info("owner");
    let msg = ExecuteMsg::SwapAsset {
        denom: "umars".to_string(),
        amount: None,
        safety_fund_route: None,
        fee_collector_route: None,
        safety_fund_min_receive: Some(Uint128::from(1000u128)),
        fee_collector_min_receive: None,
    };
    let result = execute(deps.as_mut(), mock_env(), info, msg);
    assert!(result.is_ok());
}

#[test]
fn owner_can_add_multiple_to_whitelist() {
    let mut deps = helpers::setup_test();
    // Owner adds alice and bob
    let info = mock_info("owner");
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: UpdateConfig {
            whitelist_actions: Some(vec![
                WhitelistAction::AddAddress {
                    address: "alice".to_string(),
                },
                WhitelistAction::AddAddress {
                    address: "bob".to_string(),
                },
            ]),
            ..Default::default()
        },
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let cfg: ConfigResponse = helpers::query(deps.as_ref(), QueryMsg::Config {});
    assert!(cfg.whitelisted_distributors.contains(&"alice".to_string()));
    assert!(cfg.whitelisted_distributors.contains(&"bob".to_string()));
}

#[test]
fn owner_can_remove_multiple_from_whitelist() {
    let mut deps = helpers::setup_test();
    // Owner adds alice and bob
    let info = mock_info("owner");
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: UpdateConfig {
            whitelist_actions: Some(vec![
                WhitelistAction::AddAddress {
                    address: "alice".to_string(),
                },
                WhitelistAction::AddAddress {
                    address: "bob".to_string(),
                },
            ]),
            ..Default::default()
        },
    };
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // Owner removes alice and bob
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: UpdateConfig {
            whitelist_actions: Some(vec![
                WhitelistAction::RemoveAddress {
                    address: "alice".to_string(),
                },
                WhitelistAction::RemoveAddress {
                    address: "bob".to_string(),
                },
            ]),
            ..Default::default()
        },
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let cfg: ConfigResponse = helpers::query(deps.as_ref(), QueryMsg::Config {});
    assert!(!cfg.whitelisted_distributors.contains(&"alice".to_string()));
    assert!(!cfg.whitelisted_distributors.contains(&"bob".to_string()));
}

#[test]
fn owner_can_add_and_remove_in_same_tx() {
    let mut deps = helpers::setup_test();
    // Owner adds alice
    let info = mock_info("owner");
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: UpdateConfig {
            whitelist_actions: Some(vec![WhitelistAction::AddAddress {
                address: "alice".to_string(),
            }]),
            ..Default::default()
        },
    };
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // Ensure alice is there
    let cfg: ConfigResponse = helpers::query(deps.as_ref(), QueryMsg::Config {});
    assert!(cfg.whitelisted_distributors.contains(&"alice".to_string()));

    // Owner adds bob and removes alice in the same tx
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: UpdateConfig {
            whitelist_actions: Some(vec![
                WhitelistAction::AddAddress {
                    address: "bob".to_string(),
                },
                WhitelistAction::RemoveAddress {
                    address: "alice".to_string(),
                },
            ]),
            ..Default::default()
        },
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let cfg: ConfigResponse = helpers::query(deps.as_ref(), QueryMsg::Config {});
    assert!(cfg.whitelisted_distributors.contains(&"bob".to_string()));
    assert!(!cfg.whitelisted_distributors.contains(&"alice".to_string()));
}
