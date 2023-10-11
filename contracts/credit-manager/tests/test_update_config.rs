use cosmwasm_std::{Addr, Decimal, Empty, Uint128};
use cw_multi_test::{BasicApp, Executor};
use helpers::assert_err;
use mars_mock_oracle::msg::{CoinPrice, InstantiateMsg as OracleInstantiateMsg};
use mars_red_bank_types::oracle::ActionKind;
use mars_rover::{
    adapters::{
        health::HealthContractUnchecked, incentives::IncentivesUnchecked, oracle::OracleUnchecked,
        red_bank::RedBankUnchecked, rewards_collector::RewardsCollector, swap::SwapperBase,
        zapper::ZapperBase,
    },
    error::ContractError,
    msg::instantiate::ConfigUpdates,
};
use mars_rover_health_types::AccountKind;

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
            incentives: None,
            max_unlocking_positions: None,
            max_slippage: None,
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
fn invalid_max_slippage() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();

    let res = mock.update_config(
        &Addr::unchecked(original_config.ownership.owner.clone().unwrap()),
        ConfigUpdates {
            max_slippage: Some(Decimal::zero()),
            ..Default::default()
        },
    );
    assert_err(
        res,
        ContractError::InvalidConfig {
            reason: "Max slippage must be greater than 0 and less than 1".to_string(),
        },
    );

    let res = mock.update_config(
        &Addr::unchecked(original_config.ownership.owner.unwrap()),
        ConfigUpdates {
            max_slippage: Some(Decimal::one()),
            ..Default::default()
        },
    );
    assert_err(
        res,
        ContractError::InvalidConfig {
            reason: "Max slippage must be greater than 0 and less than 1".to_string(),
        },
    );
}

#[test]
fn update_config_works_with_full_config() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();

    let new_nft_contract = mock.deploy_new_nft_contract().unwrap();
    let new_oracle = deploy_new_oracle(&mut mock.app);
    let new_red_bank = deploy_new_red_bank(&mut mock.app);
    let new_incentives = IncentivesUnchecked::new("new_incentives".to_string());
    let new_zapper = ZapperBase::new("new_zapper".to_string());
    let new_unlocking_max = Uint128::new(321);
    let new_max_slippage = Decimal::percent(12);
    let new_swapper = SwapperBase::new("new_swapper".to_string());
    let new_health_contract = HealthContractUnchecked::new("new_health_contract".to_string());
    let new_rewards_collector = "rewards_collector_contract_new".to_string();

    mock.update_config(
        &Addr::unchecked(original_config.ownership.owner.clone().unwrap()),
        ConfigUpdates {
            account_nft: Some(new_nft_contract.clone()),
            oracle: Some(new_oracle.clone()),
            red_bank: Some(new_red_bank.clone()),
            incentives: Some(new_incentives.clone()),
            max_unlocking_positions: Some(new_unlocking_max),
            max_slippage: Some(new_max_slippage),
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

    assert_eq!(new_config.max_slippage, new_max_slippage);
    assert_ne!(new_config.max_slippage, original_config.max_slippage);

    assert_eq!(&new_config.swapper, new_swapper.address());
    assert_ne!(new_config.swapper, original_config.swapper);

    assert_eq!(&new_config.health_contract, new_health_contract.address());
    assert_ne!(new_config.health_contract, original_config.health_contract);

    let rc_accounts = mock.query_accounts(&new_rewards_collector, None, None);
    let rc_account = rc_accounts.first().unwrap();
    assert_eq!(rc_account.kind, AccountKind::Default);
    assert_eq!(
        new_config.rewards_collector.clone().unwrap(),
        RewardsCollector {
            address: new_rewards_collector,
            account_id: rc_account.id.clone()
        }
    );
    assert_ne!(new_config.rewards_collector, original_config.rewards_collector);

    assert_eq!(&new_config.incentives, new_incentives.address());
    assert_ne!(new_config.incentives, original_config.incentives);
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
                        pricing: ActionKind::Default,
                        denom: "uusdc".to_string(),
                        price: Decimal::from_atomics(12345u128, 4).unwrap(),
                    },
                    CoinPrice {
                        pricing: ActionKind::Default,
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
