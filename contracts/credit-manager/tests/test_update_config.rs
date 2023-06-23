use cosmwasm_std::{Addr, Decimal, Empty, Uint128};
use cw_multi_test::{BasicApp, Executor};
use mars_mock_oracle::msg::{CoinPrice, InstantiateMsg as OracleInstantiateMsg};
use mars_rover::{
    adapters::{
        health::HealthContractUnchecked, oracle::OracleUnchecked, red_bank::RedBankUnchecked,
        swap::SwapperBase, zapper::ZapperBase,
    },
    msg::instantiate::ConfigUpdates,
};

use crate::helpers::{mock_oracle_contract, mock_red_bank_contract, MockEnv};

pub mod helpers;

#[test]
fn only_owner_can_update_config() {
    let mut mock = MockEnv::new().build().unwrap();
    let new_owner = Addr::unchecked("bad_guy");

    let res = mock.update_config(
        &new_owner,
        ConfigUpdates {
            account_nft: None,
            oracle: None,
            red_bank: None,
            max_unlocking_positions: None,
            swapper: None,
            zapper: None,
            health_contract: None,
            rewards_collector: None,
        },
    );

    if res.is_ok() {
        panic!("only owner should be able to update config");
    }
}

#[test]
fn update_config_works_with_full_config() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();

    let new_nft_contract = mock.deploy_new_nft_contract().unwrap();
    let new_oracle = deploy_new_oracle(&mut mock.app);
    let new_red_bank = deploy_new_red_bank(&mut mock.app);
    let new_zapper = ZapperBase::new("new_zapper".to_string());
    let new_unlocking_max = Uint128::new(321);
    let new_swapper = SwapperBase::new("new_swapper".to_string());
    let new_health_contract = HealthContractUnchecked::new("new_health_contract".to_string());
    let new_rewards_collector = "rewards_collector_contract_new".to_string();

    mock.update_config(
        &Addr::unchecked(original_config.ownership.owner.clone().unwrap()),
        ConfigUpdates {
            account_nft: Some(new_nft_contract.clone()),
            oracle: Some(new_oracle.clone()),
            red_bank: Some(new_red_bank.clone()),
            max_unlocking_positions: Some(new_unlocking_max),
            swapper: Some(new_swapper.clone()),
            zapper: Some(new_zapper.clone()),
            health_contract: Some(new_health_contract.clone()),
            rewards_collector: Some(new_rewards_collector.clone()),
        },
    )
    .unwrap();

    let new_config = mock.query_config();

    assert_eq!(new_config.account_nft, Some(new_nft_contract.address().clone()));
    assert_ne!(new_config.account_nft, original_config.account_nft);

    assert_eq!(
        new_config.ownership.owner.unwrap(),
        original_config.ownership.owner.clone().unwrap()
    );

    assert_eq!(&new_config.oracle, new_oracle.address());
    assert_ne!(new_config.oracle, original_config.oracle);

    assert_eq!(&new_config.red_bank, new_red_bank.address());
    assert_ne!(new_config.red_bank, original_config.red_bank);

    assert_eq!(&new_config.zapper, new_zapper.address());
    assert_ne!(new_config.zapper, original_config.zapper);

    assert_eq!(new_config.max_unlocking_positions, new_unlocking_max);
    assert_ne!(new_config.max_unlocking_positions, original_config.max_unlocking_positions);

    assert_eq!(&new_config.swapper, new_swapper.address());
    assert_ne!(new_config.swapper, original_config.swapper);

    assert_eq!(&new_config.health_contract, new_health_contract.address());
    assert_ne!(new_config.health_contract, original_config.health_contract);

    assert_eq!(new_config.rewards_collector.clone().unwrap(), new_rewards_collector);
    assert_ne!(new_config.rewards_collector, original_config.rewards_collector);
}

#[test]
fn update_config_works_with_some_config() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();

    let new_nft_contract = mock.deploy_new_nft_contract().unwrap();
    let new_max_unlocking = Uint128::new(42);

    mock.update_config(
        &Addr::unchecked(original_config.ownership.owner.clone().unwrap()),
        ConfigUpdates {
            account_nft: Some(new_nft_contract.clone()),
            max_unlocking_positions: Some(new_max_unlocking),
            ..Default::default()
        },
    )
    .unwrap();

    let new_config = mock.query_config();

    // Changed configs
    assert_eq!(new_config.account_nft, Some(new_nft_contract.address().clone()));
    assert_ne!(new_config.account_nft, original_config.account_nft);

    assert_eq!(new_config.max_unlocking_positions, new_max_unlocking);
    assert_ne!(new_config.max_unlocking_positions, original_config.max_unlocking_positions);

    // Unchanged configs
    assert_eq!(new_config.ownership.owner, original_config.ownership.owner);
    assert_eq!(new_config.ownership.proposed, original_config.ownership.proposed);
    assert_eq!(new_config.red_bank, original_config.red_bank);
    assert_eq!(new_config.oracle, original_config.oracle);
    assert_eq!(new_config.params, original_config.params);
    assert_eq!(new_config.swapper, original_config.swapper);
    assert_eq!(new_config.zapper, original_config.zapper);
    assert_eq!(new_config.health_contract, original_config.health_contract);
}

#[test]
fn update_config_does_nothing_when_nothing_is_passed() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();

    mock.update_config(
        &Addr::unchecked(original_config.ownership.owner.clone().unwrap()),
        Default::default(),
    )
    .unwrap();

    let new_config = mock.query_config();

    assert_eq!(new_config.account_nft, original_config.account_nft);
    assert_eq!(new_config.ownership, original_config.ownership);
    assert_eq!(new_config.red_bank, original_config.red_bank);
    assert_eq!(new_config.oracle, original_config.oracle);
    assert_eq!(new_config.zapper, original_config.zapper);
    assert_eq!(new_config.params, original_config.params);
    assert_eq!(new_config.swapper, original_config.swapper);
    assert_eq!(new_config.health_contract, original_config.health_contract);
}

fn deploy_new_oracle(app: &mut BasicApp) -> OracleUnchecked {
    let contract_code_id = app.store_code(mock_oracle_contract());
    let addr = app
        .instantiate_contract(
            contract_code_id,
            Addr::unchecked("oracle_contract_owner"),
            &OracleInstantiateMsg {
                prices: vec![
                    CoinPrice {
                        denom: "uusdc".to_string(),
                        price: Decimal::from_atomics(12345u128, 4).unwrap(),
                    },
                    CoinPrice {
                        denom: "vault_xyz".to_string(),
                        price: Decimal::from_atomics(989685877u128, 8).unwrap(),
                    },
                ],
            },
            &[],
            "mock-oracle",
            None,
        )
        .unwrap();
    OracleUnchecked::new(addr.to_string())
}

fn deploy_new_red_bank(app: &mut BasicApp) -> RedBankUnchecked {
    let contract_code_id = app.store_code(mock_red_bank_contract());
    let addr = app
        .instantiate_contract(
            contract_code_id,
            Addr::unchecked("red_bank_contract_owner"),
            &Empty {},
            &[],
            "mock-red-bank",
            None,
        )
        .unwrap();
    RedBankUnchecked::new(addr.to_string())
}
