use cosmwasm_std::{coin, Addr};
use cw_multi_test::{BankSudo, Executor, SudoMsg};
use mars_testing::multitest::helpers;
use mars_types::rewards_collector::{
    ExecuteMsg as RcExecuteMsg, UpdateConfig as RcUpdateConfig, WhitelistAction,
};

#[test]
fn rewards_collector_whitelist_enforced() {
    let mut mock = helpers::MockEnv::new().build().unwrap();
    let config = mock.query_config();
    let rewards_collector_info = config.rewards_collector.expect("rewards collector configured");
    let rewards_collector_addr = Addr::unchecked(rewards_collector_info.address.clone());

    // fund the rewards collector with uusdc so distribution can execute
    mock.app
        .sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: rewards_collector_addr.to_string(),
            amount: vec![coin(1_000, "uusdc")],
        }))
        .unwrap();

    // non-whitelisted address cannot distribute rewards
    assert!(mock
        .app
        .execute_contract(
            Addr::unchecked("not_whitelisted"),
            rewards_collector_addr.clone(),
            &RcExecuteMsg::DistributeRewards {
                denom: "uusdc".to_string(),
            },
            &[],
        )
        .is_err());

    // whitelist alice
    mock.app
        .execute_contract(
            Addr::unchecked("owner"),
            rewards_collector_addr.clone(),
            &RcExecuteMsg::UpdateConfig {
                new_cfg: RcUpdateConfig {
                    whitelist_actions: Some(vec![WhitelistAction::AddAddress {
                        address: "alice".to_string(),
                    }]),
                    ..Default::default()
                },
            },
            &[],
        )
        .unwrap();

    // whitelisted address succeeds
    mock.app
        .execute_contract(
            Addr::unchecked("alice"),
            rewards_collector_addr,
            &RcExecuteMsg::DistributeRewards {
                denom: "uusdc".to_string(),
            },
            &[],
        )
        .unwrap();
}
