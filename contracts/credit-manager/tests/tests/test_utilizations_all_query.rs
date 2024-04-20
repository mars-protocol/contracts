use std::str::FromStr;

use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use mars_types::{
    credit_manager::{
        Action::{Deposit, EnterVault},
        ActionAmount::Exact,
        ActionCoin, VaultUtilizationResponse,
    },
    params::LiquidationBonus,
};

use super::helpers::{
    locked_vault_info, uatom_info, ujake_info, unlocked_vault_info, uosmo_info, AccountToFund,
    CoinInfo, MockEnv, VaultTestInfo,
};

#[test]
fn empty_if_start_after_not_found() {
    let vault_info_1 = unlocked_vault_info();
    let vault_info_2 = locked_vault_info();

    let mock = MockEnv::new().vault_configs(&[vault_info_1, vault_info_2]).build().unwrap();

    let res = mock.query_all_vault_utilizations(Some("test".to_string()), None).unwrap();

    assert_eq!(res.data, vec![]);
    assert!(!res.metadata.has_more);
}

#[test]
fn utilizations_are_zero() {
    let vault_info_1 = unlocked_vault_info();
    let vault_info_2 = locked_vault_info();

    let mock = MockEnv::new()
        .vault_configs(&[vault_info_1.clone(), vault_info_2.clone()])
        .build()
        .unwrap();

    let res = mock.query_all_vault_utilizations(None, None).unwrap();

    assert_eq!(
        res.data,
        vec![
            VaultUtilizationResponse {
                vault: mock.get_vault(&vault_info_1),
                utilization: Coin {
                    denom: vault_info_1.deposit_cap.denom,
                    amount: Uint128::zero(),
                },
            },
            VaultUtilizationResponse {
                vault: mock.get_vault(&vault_info_2),
                utilization: Coin {
                    denom: vault_info_2.deposit_cap.denom,
                    amount: Uint128::zero(),
                },
            },
        ]
    );
}

#[test]
fn utilizations_if_cap_is_base_denom() {
    let user = Addr::unchecked("user");
    let base_info = CoinInfo {
        denom: "base_denom".to_string(),
        price: Decimal::from_str("1").unwrap(),
        max_ltv: Decimal::from_str("0.6").unwrap(),
        liquidation_threshold: Decimal::from_str("0.7").unwrap(),
        liquidation_bonus: LiquidationBonus {
            starting_lb: Decimal::percent(1u64),
            slope: Decimal::from_atomics(2u128, 0).unwrap(),
            min_lb: Decimal::percent(2u64),
            max_lb: Decimal::percent(10u64),
        },
        protocol_liquidation_fee: Decimal::percent(2u64),
        whitelisted: true,
        hls: None,
    };

    let vault_info_1 = VaultTestInfo {
        vault_token_denom: "uvault_1".to_string(),
        base_token_denom: base_info.denom.clone(),
        lockup: None,
        deposit_cap: base_info.to_coin(1_000),
        max_ltv: Decimal::from_str("0.6").unwrap(),
        liquidation_threshold: Decimal::from_str("0.7").unwrap(),
        whitelisted: true,
        hls: None,
    };

    let vault_info_2 = VaultTestInfo {
        vault_token_denom: "uvault_2".to_string(),
        base_token_denom: base_info.denom.clone(),
        lockup: None,
        deposit_cap: base_info.to_coin(100),
        max_ltv: Decimal::from_str("0.6").unwrap(),
        liquidation_threshold: Decimal::from_str("0.7").unwrap(),
        whitelisted: true,
        hls: None,
    };

    let mut mock = MockEnv::new()
        .set_params(&[base_info.clone()])
        .vault_configs(&[vault_info_1.clone(), vault_info_2.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![base_info.to_coin(50)],
        })
        .build()
        .unwrap();

    let vault_1 = mock.get_vault(&vault_info_1);
    let vault_2 = mock.get_vault(&vault_info_2);

    let account_id = mock.create_credit_account(&user).unwrap();

    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(base_info.to_coin(50)),
            EnterVault {
                vault: vault_1.clone(),
                coin: ActionCoin {
                    denom: base_info.denom.clone(),
                    amount: Exact(Uint128::new(50)),
                },
            },
        ],
        &[base_info.to_coin(50)],
    )
    .unwrap();

    let res = mock.query_all_vault_utilizations(None, None).unwrap();

    let vault_utilization_1 = VaultUtilizationResponse {
        vault: vault_1,
        utilization: Coin {
            denom: vault_info_1.deposit_cap.denom,
            amount: Uint128::new(50),
        },
    };

    let vault_utilization_2 = VaultUtilizationResponse {
        vault: vault_2,
        utilization: Coin {
            denom: vault_info_2.deposit_cap.denom,
            amount: Uint128::zero(),
        },
    };

    assert_eq!(res.data, vec![vault_utilization_1, vault_utilization_2]);
    assert!(!res.metadata.has_more);
}

/*
    1 uosmo = 0.25
    1 ujake = 2.3654
    1 uatom = 1

    Vault 1 utilization:
        - deposit 1_000 ujake
        - 1 ujake = 2.3654 0.25 = 9.4616 uosmo
        - 1_000_000 * 9.640 = 9641600 uosmo

    Vault 2 utilization:
        - deposit 1_000 uatom
        - 1 uatom = 1 / 2.3654 = 0.422761477 ujake
        - 1_000_000_000 * 0.4227 = 422761477 uosmo (floored)
*/
#[test]
fn utilization_in_other_denom() {
    let osmo_info = uosmo_info(); // .25
    let jake_info = ujake_info(); // 2.3654
    let atom_info = uatom_info(); // 1

    let vault_info_1 = VaultTestInfo {
        vault_token_denom: "uvault_1".to_string(),
        base_token_denom: jake_info.denom.clone(),
        lockup: None,
        deposit_cap: osmo_info.to_coin(50_000_000),
        max_ltv: Decimal::from_str("0.6").unwrap(),
        liquidation_threshold: Decimal::from_str("0.7").unwrap(),
        whitelisted: true,
        hls: None,
    };

    let vault_info_2 = VaultTestInfo {
        vault_token_denom: "uvault_2".to_string(),
        base_token_denom: atom_info.denom.clone(),
        lockup: None,
        deposit_cap: jake_info.to_coin(1_000_000_000),
        max_ltv: Decimal::from_str("0.6").unwrap(),
        liquidation_threshold: Decimal::from_str("0.7").unwrap(),
        whitelisted: true,
        hls: None,
    };

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[jake_info.clone(), osmo_info, atom_info.clone()])
        .vault_configs(&[vault_info_1.clone(), vault_info_2.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![jake_info.to_coin(1_000_000), atom_info.to_coin(1_000_000_000)],
        })
        .build()
        .unwrap();

    let vault_1 = mock.get_vault(&vault_info_1);
    let vault_2 = mock.get_vault(&vault_info_2);
    let account_id = mock.create_credit_account(&user).unwrap();

    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(jake_info.to_coin(1_000_000)),
            Deposit(atom_info.to_coin(1_000_000_000)),
            EnterVault {
                vault: vault_1.clone(),
                coin: ActionCoin {
                    denom: jake_info.denom.clone(),
                    amount: Exact(Uint128::new(1_000_000)),
                },
            },
            EnterVault {
                vault: vault_2.clone(),
                coin: ActionCoin {
                    denom: atom_info.denom.clone(),
                    amount: Exact(Uint128::new(1_000_000_000)),
                },
            },
        ],
        &[jake_info.to_coin(1_000_000), atom_info.to_coin(1_000_000_000)],
    )
    .unwrap();

    let res = mock.query_all_vault_utilizations(None, None).unwrap();

    let vault_utilization_1 = VaultUtilizationResponse {
        vault: vault_1,
        utilization: Coin {
            denom: vault_info_1.deposit_cap.denom,
            amount: Uint128::new(9461600),
        },
    };

    let vault_utilization_2 = VaultUtilizationResponse {
        vault: vault_2,
        utilization: Coin {
            denom: vault_info_2.deposit_cap.denom,
            amount: Uint128::new(422761477),
        },
    };

    assert_eq!(res.data, vec![vault_utilization_1, vault_utilization_2]);
    assert!(!res.metadata.has_more);
}

#[test]
fn has_more_true_when_limit_not_reached() {
    let vault_info_1 = unlocked_vault_info();
    let vault_info_2 = locked_vault_info();

    let mock = MockEnv::new().vault_configs(&[vault_info_1, vault_info_2]).build().unwrap();

    let res = mock.query_all_vault_utilizations(None, Some(1)).unwrap();

    assert!(res.metadata.has_more);
}
