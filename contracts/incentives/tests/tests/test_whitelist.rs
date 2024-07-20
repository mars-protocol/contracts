use cosmwasm_std::{
    coin,
    testing::{mock_env, mock_info},
    Addr, Coin, Timestamp, Uint128,
};
use mars_incentives::{
    contract::execute,
    mars_incentives::execute_balance_change,
    state::{EMISSIONS, WHITELIST_COUNT},
    ContractError,
};
use mars_owner::OwnerError::NotOwner;
use mars_testing::MockEnvParams;
use mars_types::{
    incentives::{ConfigResponse, ExecuteMsg, QueryMsg, WhitelistEntry},
    red_bank::{Market, UserCollateralResponse},
};
use mars_utils::error::ValidationError;

use super::helpers::{
    th_query, th_query_with_env, th_setup, th_setup_with_env, ths_setup_with_epoch_duration,
};

#[test]
fn initialized_state() {
    let deps = th_setup();

    let whitelist: Vec<(String, Uint128)> = th_query(deps.as_ref(), QueryMsg::Whitelist {});
    assert!(whitelist.is_empty());
}

#[test]
fn update_whitelist_only_callable_by_admin() {
    let mut deps = th_setup();

    // only owner can update whitelist
    let bad_guy = "bad_guy";
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(bad_guy, &[]),
        ExecuteMsg::UpdateWhitelist {
            add_denoms: vec![("umars".to_string(), Uint128::new(3)).into()],
            remove_denoms: vec![],
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Owner(NotOwner {}));
}

#[test]
fn update_whitelist_add_denom_works() {
    let mut deps = th_setup();

    // only owner can update whitelist
    let owner = "owner";
    let msg = ExecuteMsg::UpdateWhitelist {
        add_denoms: vec![("umars".to_string(), Uint128::new(3)).into()],
        remove_denoms: vec![],
    };
    let info = mock_info(owner, &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let whitelist: Vec<WhitelistEntry> = th_query(deps.as_ref(), QueryMsg::Whitelist {});
    assert_eq!(whitelist, vec![("umars".to_string(), Uint128::new(3)).into()]);
}

#[test]
fn update_whitelist_remove_denom_works() {
    let mut deps = th_setup();

    // only owner can update whitelist
    let owner = "owner";
    let msg = ExecuteMsg::UpdateWhitelist {
        add_denoms: vec![("umars".to_string(), Uint128::new(3)).into()],
        remove_denoms: vec![],
    };
    let info = mock_info(owner, &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let whitelist: Vec<WhitelistEntry> = th_query(deps.as_ref(), QueryMsg::Whitelist {});
    assert_eq!(whitelist, vec![("umars".to_string(), Uint128::new(3)).into()]);

    // remove denom
    let msg = ExecuteMsg::UpdateWhitelist {
        add_denoms: vec![],
        remove_denoms: vec!["umars".to_string()],
    };
    let info = mock_info(owner, &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let whitelist: Vec<WhitelistEntry> = th_query(deps.as_ref(), QueryMsg::Whitelist {});
    assert!(whitelist.is_empty());
}

#[test]
fn cannot_add_invalid_denom_to_whitelist() {
    let mut deps = th_setup();

    // only owner can update whitelist
    let owner = "owner";
    let msg = ExecuteMsg::UpdateWhitelist {
        add_denoms: vec![("//invalid-denom//".to_string(), Uint128::new(3)).into()],
        remove_denoms: vec![],
    };
    let info = mock_info(owner, &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert!(matches!(
        err,
        ContractError::Validation(ValidationError::InvalidDenom {
            reason: _
        })
    ));
}

#[test]
fn incentive_can_only_be_added_if_denom_whitelisted() {
    let env = mock_env();
    let mut deps = th_setup_with_env(env.clone());

    // Set Red Bank Market
    deps.querier.set_redbank_market(Market {
        denom: "uosmo".to_string(),
        collateral_total_scaled: Uint128::zero(),
        ..Default::default()
    });

    let owner = "owner";
    let set_incentive_msg = ExecuteMsg::SetAssetIncentive {
        collateral_denom: "uosmo".to_string(),
        incentive_denom: "umars".to_string(),
        emission_per_second: Uint128::new(100),
        start_time: env.block.time.seconds(),
        duration: 604800,
    };
    let info = mock_info(owner, &[coin(100 * 604800, "uosmo")]);
    let err = execute(deps.as_mut(), mock_env(), info, set_incentive_msg.clone()).unwrap_err();
    assert!(matches!(
        err,
        ContractError::NotWhitelisted {
            denom: _
        }
    ));

    // add denom to whitelist
    let add_whitelist_msg: ExecuteMsg = ExecuteMsg::UpdateWhitelist {
        add_denoms: vec![("umars".to_string(), Uint128::new(3)).into()],
        remove_denoms: vec![],
    };
    execute(deps.as_mut(), mock_env(), mock_info(owner, &[]), add_whitelist_msg).unwrap();

    // add incentive
    let info = mock_info(owner, &[coin(100 * 604800, "umars")]);
    execute(deps.as_mut(), mock_env(), info, set_incentive_msg).unwrap();
}

#[test]
fn incentives_updated_and_removed_when_removing_from_whitelist() {
    let env = mock_env();
    let mut deps = ths_setup_with_epoch_duration(env.clone(), 604800);
    let owner = "owner";

    let collateral = Uint128::from(1000000u128);
    // Set Red Bank Market
    deps.querier.set_redbank_market(Market {
        denom: "uosmo".to_string(),
        collateral_total_scaled: collateral,
        ..Default::default()
    });

    // add denom to whitelist
    let add_whitelist_msg = ExecuteMsg::UpdateWhitelist {
        add_denoms: vec![("umars".to_string(), Uint128::new(3)).into()],
        remove_denoms: vec![],
    };
    execute(deps.as_mut(), mock_env(), mock_info(owner, &[]), add_whitelist_msg).unwrap();

    // add incentive
    let start_time = env.block.time.seconds();
    let set_incentive_msg = ExecuteMsg::SetAssetIncentive {
        collateral_denom: "uosmo".to_string(),
        incentive_denom: "umars".to_string(),
        emission_per_second: Uint128::new(100),
        start_time,
        duration: 604800,
    };
    let info = mock_info(owner, &[coin(100 * 604800, "umars")]);
    execute(deps.as_mut(), mock_env(), info, set_incentive_msg).unwrap();

    // Query incentive schedule
    let emission_per_second =
        EMISSIONS.load(&deps.storage, ("uosmo", "umars", start_time)).unwrap();
    assert_eq!(emission_per_second, Uint128::new(100));

    // Deposit collateral
    let user_addr = Addr::unchecked("user");
    deps.querier.set_red_bank_user_collateral(
        user_addr.clone(),
        UserCollateralResponse {
            denom: "uosmo".to_string(),
            amount: Uint128::zero(), // doesn't matter for this test
            amount_scaled: collateral,
            enabled: true,
        },
    );
    // Execute Balance Change
    execute_balance_change(
        deps.as_mut(),
        env.clone(),
        mock_info("red_bank", &[]),
        user_addr.clone(),
        None,
        "uosmo".to_string(),
        Uint128::zero(),
        Uint128::zero(),
    )
    .unwrap();

    // Fast forward time
    let new_time = env.block.time.seconds() + 100;
    let env = mars_testing::mock_env(MockEnvParams {
        block_time: Timestamp::from_seconds(new_time),
        ..Default::default()
    });

    // Remove denom from whitelist
    let remove_whitelist_msg = ExecuteMsg::UpdateWhitelist {
        add_denoms: vec![],
        remove_denoms: vec!["umars".to_string()],
    };
    execute(deps.as_mut(), env.clone(), mock_info(owner, &[]), remove_whitelist_msg).unwrap();

    // Query users rewards. They should have gotten rewards for the entire time
    let user_rewards: Vec<Coin> = th_query_with_env(
        deps.as_ref(),
        env.clone(),
        QueryMsg::UserUnclaimedRewards {
            user: user_addr.to_string(),
            account_id: None,
            start_after_collateral_denom: None,
            start_after_incentive_denom: None,
            limit: None,
        },
    );
    assert_eq!(user_rewards, vec![coin(100 * 100, "umars")]);

    // Fast forward time 100 more seconds and query rewards again.
    // They should be the same.
    let new_time = env.block.time.seconds() + 100;
    let env = mars_testing::mock_env(MockEnvParams {
        block_time: Timestamp::from_seconds(new_time),
        ..Default::default()
    });
    let user_rewards: Vec<Coin> = th_query_with_env(
        deps.as_ref(),
        env,
        QueryMsg::UserUnclaimedRewards {
            user: user_addr.to_string(),
            account_id: None,
            start_after_collateral_denom: None,
            start_after_incentive_denom: None,
            limit: None,
        },
    );
    assert_eq!(user_rewards, vec![coin(100 * 100, "umars")]);

    // Read active emissions. There should be none
    EMISSIONS.load(&deps.storage, ("uosmo", "umars", start_time)).unwrap_err();
}

#[test]
fn whitelisting_already_whitelisted_denom_updates_min_emission() {
    let env = mock_env();
    let mut deps = th_setup_with_env(env);

    let owner = "owner";

    // add denom to whitelist
    let add_whitelist_msg: ExecuteMsg = ExecuteMsg::UpdateWhitelist {
        add_denoms: vec![("umars".to_string(), Uint128::new(3)).into()],
        remove_denoms: vec![],
    };
    execute(deps.as_mut(), mock_env(), mock_info(owner, &[]), add_whitelist_msg).unwrap();

    // Query whitelist
    let whitelist: Vec<WhitelistEntry> = th_query(deps.as_ref(), QueryMsg::Whitelist {});
    assert_eq!(whitelist, vec![("umars".to_string(), Uint128::new(3)).into()]);

    // Query whitelist count
    let whitelist_count = WHITELIST_COUNT.load(&deps.storage).unwrap();
    assert_eq!(whitelist_count, 1);

    // add denom to whitelist again, with a higher min emission
    let add_whitelist_msg: ExecuteMsg = ExecuteMsg::UpdateWhitelist {
        add_denoms: vec![("umars".to_string(), Uint128::new(5)).into()],
        remove_denoms: vec![],
    };
    execute(deps.as_mut(), mock_env(), mock_info(owner, &[]), add_whitelist_msg).unwrap();

    // Query whitelist
    let whitelist: Vec<WhitelistEntry> = th_query(deps.as_ref(), QueryMsg::Whitelist {});
    assert_eq!(whitelist, vec![("umars".to_string(), Uint128::new(5)).into()]);

    // Query whitelist count, should not have changed.
    let whitelist_count = WHITELIST_COUNT.load(&deps.storage).unwrap();
    assert_eq!(whitelist_count, 1);
}

#[test]
fn cannot_whitelist_more_than_max_limit() {
    let env = mock_env();
    let mut deps = th_setup_with_env(env);

    let owner = "owner";

    // add 10 denoms to whitelist
    let add_whitelist_msg: ExecuteMsg = ExecuteMsg::UpdateWhitelist {
        add_denoms: vec![
            ("umars".to_string(), Uint128::new(3)).into(),
            ("denom1".to_string(), Uint128::new(3)).into(),
            ("denom2".to_string(), Uint128::new(3)).into(),
            ("denom3".to_string(), Uint128::new(3)).into(),
            ("denom4".to_string(), Uint128::new(3)).into(),
            ("denom5".to_string(), Uint128::new(3)).into(),
            ("denom6".to_string(), Uint128::new(3)).into(),
            ("denom7".to_string(), Uint128::new(3)).into(),
            ("denom8".to_string(), Uint128::new(3)).into(),
            ("denom9".to_string(), Uint128::new(3)).into(),
        ],
        remove_denoms: vec![],
    };
    execute(deps.as_mut(), mock_env(), mock_info(owner, &[]), add_whitelist_msg).unwrap();

    // Check whitelist count
    let whitelist_count = WHITELIST_COUNT.load(&deps.storage).unwrap();
    assert_eq!(whitelist_count, 10);

    // add denom to whitelist again, should fail
    let add_whitelist_msg: ExecuteMsg = ExecuteMsg::UpdateWhitelist {
        add_denoms: vec![("denom10".to_string(), Uint128::new(5)).into()],
        remove_denoms: vec![],
    };
    let res =
        execute(deps.as_mut(), mock_env(), mock_info(owner, &[]), add_whitelist_msg).unwrap_err();
    assert_eq!(
        res,
        ContractError::MaxWhitelistLimitReached {
            max_whitelist_limit: 10
        }
    );

    // Remove one denom from whitelist, and add a new one, should work
    let add_whitelist_msg: ExecuteMsg = ExecuteMsg::UpdateWhitelist {
        add_denoms: vec![("denom10".to_string(), Uint128::new(5)).into()],
        remove_denoms: vec![("umars".to_string())],
    };
    execute(deps.as_mut(), mock_env(), mock_info(owner, &[]), add_whitelist_msg).unwrap();

    // Check whitelist count. Should still be 10.
    let config: ConfigResponse = th_query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(config.max_whitelisted_denoms, 10);
}

#[test]
fn update_whitelist_args_cannot_contain_duplicate_denoms() {
    let env = mock_env();
    let mut deps = th_setup_with_env(env);

    let owner = "owner";

    // add 1 denoms to whitelist twice
    let whitelist_msg: ExecuteMsg = ExecuteMsg::UpdateWhitelist {
        add_denoms: vec![
            ("umars".to_string(), Uint128::new(3)).into(),
            ("umars".to_string(), Uint128::new(5)).into(),
        ],
        remove_denoms: vec![],
    };
    let err = execute(deps.as_mut(), mock_env(), mock_info(owner, &[]), whitelist_msg).unwrap_err();
    assert_eq!(
        err,
        ContractError::DuplicateDenom {
            denom: "umars".to_string()
        }
    );

    // Try to remove the same denom twice
    let whitelist_msg: ExecuteMsg = ExecuteMsg::UpdateWhitelist {
        add_denoms: vec![],
        remove_denoms: vec!["umars".to_string(), "umars".to_string()],
    };
    let err = execute(deps.as_mut(), mock_env(), mock_info(owner, &[]), whitelist_msg).unwrap_err();
    assert_eq!(
        err,
        ContractError::DuplicateDenom {
            denom: "umars".to_string()
        }
    );

    // Try to add and remove the same denom
    let whitelist_msg: ExecuteMsg = ExecuteMsg::UpdateWhitelist {
        add_denoms: vec![("umars".to_string(), Uint128::new(3)).into()],
        remove_denoms: vec![("umars".to_string())],
    };
    let err = execute(deps.as_mut(), mock_env(), mock_info(owner, &[]), whitelist_msg).unwrap_err();
    assert_eq!(
        err,
        ContractError::DuplicateDenom {
            denom: "umars".to_string()
        }
    );
}

#[test]
fn cannot_remove_denom_from_whitelist_that_is_not_there() {
    let env = mock_env();
    let mut deps = th_setup_with_env(env);

    let owner = "owner";

    // add 1 denoms to whitelist
    let add_whitelist_msg: ExecuteMsg = ExecuteMsg::UpdateWhitelist {
        add_denoms: vec![("umars".to_string(), Uint128::new(3)).into()],
        remove_denoms: vec![],
    };
    execute(deps.as_mut(), mock_env(), mock_info(owner, &[]), add_whitelist_msg).unwrap();

    // Query whitelist
    let whitelist: Vec<WhitelistEntry> = th_query(deps.as_ref(), QueryMsg::Whitelist {});
    assert_eq!(whitelist, vec![("umars".to_string(), Uint128::new(3)).into()]);

    // Check whitelist count
    let whitelist_count = WHITELIST_COUNT.load(&deps.storage).unwrap();
    assert_eq!(whitelist_count, 1);

    // Remove non-whitelisted denom, should error
    let add_whitelist_msg: ExecuteMsg = ExecuteMsg::UpdateWhitelist {
        add_denoms: vec![],
        remove_denoms: vec![("denom1".to_string())],
    };
    let res =
        execute(deps.as_mut(), mock_env(), mock_info(owner, &[]), add_whitelist_msg).unwrap_err();
    assert_eq!(
        res,
        ContractError::NotWhitelisted {
            denom: "denom1".to_string()
        }
    );

    // Query whitelist, should still be the same
    let whitelist: Vec<WhitelistEntry> = th_query(deps.as_ref(), QueryMsg::Whitelist {});
    assert_eq!(whitelist, vec![("umars".to_string(), Uint128::new(3)).into()]);

    // Check whitelist count. Should still be 1.
    let whitelist_count = WHITELIST_COUNT.load(&deps.storage).unwrap();
    assert_eq!(whitelist_count, 1);
}
