use std::{collections::HashMap, ops::Add, str::FromStr};

use cosmwasm_std::{coin, Addr, Coin, Decimal, Uint128};
use mars_params::types::vault::VaultConfig;
use mars_rover::{
    adapters::vault::{
        CoinValue, LockingVaultAmount, UnlockingPositions, Vault, VaultAmount, VaultPosition,
        VaultPositionAmount, VaultPositionValue, VaultUnlockingPosition,
    },
    msg::query::{DebtAmount, LentAmount, Positions},
};
use mars_rover_health_computer::{DenomsData, HealthComputer, VaultsData};
use mars_rover_health_types::AccountKind;

use crate::helpers::{udai_info, ujuno_info, uluna_info, umars_info, ustars_info};

pub mod helpers;

/// Action: User deposits 300 mars (1 price)
/// Health: assets_value: 300
///         debt value 0
///         liquidatable: false
///         above_max_ltv: false
#[test]
fn only_assets_with_no_debts() {
    let umars = umars_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([(umars.denom.clone(), umars.price)]),
        params: HashMap::from([(umars.denom.clone(), umars.params.clone())]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let deposit_amount = Uint128::new(300);
    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![Coin {
                denom: umars.denom.clone(),
                amount: deposit_amount,
            }],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
        },
        denoms_data,
        vaults_data,
    };

    let health = h.compute_health().unwrap();
    let collateral_value = deposit_amount.checked_mul_floor(umars.price).unwrap();
    assert_eq!(health.total_collateral_value, collateral_value);
    assert_eq!(
        health.max_ltv_adjusted_collateral,
        collateral_value.checked_mul_floor(umars.params.max_loan_to_value).unwrap()
    );
    assert_eq!(
        health.liquidation_threshold_adjusted_collateral,
        collateral_value.checked_mul_floor(umars.params.liquidation_threshold).unwrap()
    );
    assert_eq!(health.total_debt_value, Uint128::zero());
    assert_eq!(health.liquidation_health_factor, None);
    assert_eq!(health.max_ltv_health_factor, None);
    assert!(!health.is_liquidatable());
    assert!(!health.is_above_max_ltv());
}

/// Step 1: User deposits 12 luna (100 price) and borrows 2 luna
/// Health: assets_value: 1400
///         debt value 200
///         liquidatable: false
///         above_max_ltv: false
/// Step 2: luna price goes to zero
/// Health: assets_value: 0
///         debt value 0 (still debt shares outstanding)
///         liquidatable: false
///         above_max_ltv: false
#[test]
fn terra_ragnarok() {
    let mut uluna = uluna_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([(uluna.denom.clone(), uluna.price)]),
        params: HashMap::from([(uluna.denom.clone(), uluna.params.clone())]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let deposit_amount = Uint128::new(12);
    let borrow_amount = Uint128::new(3);

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![Coin {
                denom: uluna.denom.clone(),
                amount: deposit_amount,
            }],
            debts: vec![DebtAmount {
                denom: uluna.denom.clone(),
                amount: borrow_amount,
                shares: Uint128::new(100),
            }],
            lends: vec![],
            vaults: vec![],
        },
        denoms_data,
        vaults_data: vaults_data.clone(),
    };

    let health = h.compute_health().unwrap();
    let collateral_value = deposit_amount.checked_mul_floor(uluna.price).unwrap();
    let debts_value = borrow_amount.checked_mul_floor(uluna.price).unwrap();

    assert_eq!(health.total_collateral_value, collateral_value);
    assert_eq!(
        health.max_ltv_adjusted_collateral,
        collateral_value.checked_mul_floor(uluna.params.max_loan_to_value).unwrap()
    );
    assert_eq!(
        health.liquidation_threshold_adjusted_collateral,
        collateral_value.checked_mul_floor(uluna.params.liquidation_threshold).unwrap()
    );
    assert_eq!(health.total_debt_value, borrow_amount.checked_mul_floor(uluna.price).unwrap());
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_ratio(
            collateral_value.checked_mul_floor(uluna.params.liquidation_threshold).unwrap(),
            debts_value
        ))
    );
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_ratio(
            collateral_value.checked_mul_floor(uluna.params.max_loan_to_value).unwrap(),
            debts_value,
        ))
    );
    assert!(!health.is_liquidatable());
    assert!(!health.is_above_max_ltv());

    // Terra implosion
    uluna.price = Decimal::zero();

    let denoms_data = DenomsData {
        prices: HashMap::from([(uluna.denom.clone(), uluna.price)]),
        params: HashMap::from([(uluna.denom.clone(), uluna.params.clone())]),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![Coin {
                denom: uluna.denom.clone(),
                amount: deposit_amount,
            }],
            debts: vec![DebtAmount {
                denom: uluna.denom,
                amount: borrow_amount,
                shares: Uint128::new(100),
            }],
            lends: vec![],
            vaults: vec![],
        },
        denoms_data,
        vaults_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::zero());
    assert_eq!(health.total_debt_value, Uint128::zero());
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::zero());
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::zero());
    assert_eq!(health.liquidation_health_factor, None);
    assert_eq!(health.max_ltv_health_factor, None);
    assert!(!health.is_liquidatable());
    assert!(!health.is_above_max_ltv());
}

/// Actions: User deposits 300 stars
///          and borrows 49 juno
/// Health: assets_value: 1569456334491.12991516325
///         debt value 350615100.25
///         liquidatable: false
///         above_max_ltv: false
#[test]
fn ltv_and_lqdt_adjusted_values() {
    let ustars = ustars_info();
    let ujuno = ujuno_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (ustars.denom.clone(), ustars.price),
            (ujuno.denom.clone(), ujuno.price),
        ]),
        params: HashMap::from([
            (ustars.denom.clone(), ustars.params.clone()),
            (ujuno.denom.clone(), ujuno.params.clone()),
        ]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let deposit_amount = Uint128::new(300);
    let borrow_amount = Uint128::new(49);

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![
                Coin {
                    denom: ustars.denom.clone(),
                    amount: deposit_amount,
                },
                Coin {
                    denom: ujuno.denom.clone(),
                    amount: borrow_amount,
                },
            ],
            debts: vec![DebtAmount {
                denom: ujuno.denom.clone(),
                shares: Uint128::new(12345),
                amount: borrow_amount.add(Uint128::one()), // simulated interest
            }],
            lends: vec![],
            vaults: vec![],
        },
        denoms_data,
        vaults_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(
        health.total_collateral_value,
        deposit_amount
            .checked_mul_floor(ustars.price)
            .unwrap()
            .add(borrow_amount.checked_mul_floor(ujuno.price).unwrap())
    );
    assert_eq!(
        health.total_debt_value,
        Uint128::new(350_615_101) // with simulated interest
    );
    let lqdt_adjusted_assets_value = deposit_amount
        .checked_mul_floor(ustars.price)
        .unwrap()
        .checked_mul_floor(ustars.params.liquidation_threshold)
        .unwrap()
        .add(
            borrow_amount
                .checked_mul_floor(ujuno.price)
                .unwrap()
                .checked_mul_floor(ujuno.params.liquidation_threshold)
                .unwrap(),
        );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_ratio(
            lqdt_adjusted_assets_value,
            (borrow_amount + Uint128::one()).checked_mul_ceil(ujuno.price).unwrap()
        ))
    );
    let ltv_adjusted_assets_value = deposit_amount
        .checked_mul_floor(ustars.price)
        .unwrap()
        .checked_mul_floor(ustars.params.max_loan_to_value)
        .unwrap()
        .add(
            borrow_amount
                .checked_mul_floor(ujuno.price)
                .unwrap()
                .checked_mul_floor(ujuno.params.max_loan_to_value)
                .unwrap(),
        );
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_ratio(
            ltv_adjusted_assets_value,
            (borrow_amount + Uint128::one()).checked_mul_ceil(ujuno.price).unwrap()
        ))
    );
    assert!(!health.is_liquidatable());
    assert!(!health.is_above_max_ltv());
}

/// Borrows 30 stars
/// Borrows 49 juno
/// Deposits 298 stars
/// Test validates debt calculation results
#[test]
fn debt_value() {
    let ustars = ustars_info();
    let ujuno = ujuno_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (ustars.denom.clone(), ustars.price),
            (ujuno.denom.clone(), ujuno.price),
        ]),
        params: HashMap::from([
            (ustars.denom.clone(), ustars.params.clone()),
            (ujuno.denom.clone(), ujuno.params.clone()),
        ]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let deposit_amount_stars = Uint128::new(298);
    let borrowed_amount_juno = Uint128::new(49);
    let borrowed_amount_stars = Uint128::new(30);

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![
                Coin {
                    denom: ustars.denom.clone(),
                    amount: deposit_amount_stars,
                },
                Coin {
                    denom: ujuno.denom.clone(),
                    amount: borrowed_amount_juno,
                },
                Coin {
                    denom: ustars.denom.clone(),
                    amount: borrowed_amount_stars,
                },
            ],
            debts: vec![
                DebtAmount {
                    denom: ujuno.denom.clone(),
                    shares: Uint128::new(12345),
                    amount: borrowed_amount_juno.add(Uint128::one()), // simulated interest
                },
                DebtAmount {
                    denom: ustars.denom.clone(),
                    shares: Uint128::new(12345),
                    amount: borrowed_amount_stars.add(Uint128::one()), // simulated interest
                },
            ],
            lends: vec![],
            vaults: vec![],
        },
        denoms_data,
        vaults_data,
    };

    let health = h.compute_health().unwrap();

    assert!(!health.is_above_max_ltv());
    assert!(!health.is_liquidatable());

    let juno_debt_value =
        borrowed_amount_juno.add(Uint128::one()).checked_mul_ceil(ujuno.price).unwrap();

    let stars_debt_value =
        borrowed_amount_stars.add(Uint128::one()).checked_mul_ceil(ustars.price).unwrap();

    let total_debt_value = juno_debt_value.add(stars_debt_value);
    assert_eq!(health.total_debt_value, total_debt_value);

    let lqdt_adjusted_assets_value = deposit_amount_stars
        .checked_mul_floor(ustars.price)
        .unwrap()
        .checked_mul_floor(ustars.params.liquidation_threshold)
        .unwrap()
        .add(
            borrowed_amount_stars
                .checked_mul_floor(ustars.price)
                .unwrap()
                .checked_mul_floor(ustars.params.liquidation_threshold)
                .unwrap(),
        )
        .add(
            borrowed_amount_juno
                .checked_mul_floor(ujuno.price)
                .unwrap()
                .checked_mul_floor(ujuno.params.liquidation_threshold)
                .unwrap(),
        );

    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_ratio(lqdt_adjusted_assets_value, total_debt_value))
    );

    let max_ltv_adjusted_assets_value = deposit_amount_stars
        .checked_mul_floor(ustars.price)
        .unwrap()
        .checked_mul_floor(ustars.params.max_loan_to_value)
        .unwrap()
        .add(
            borrowed_amount_stars
                .checked_mul_floor(ustars.price)
                .unwrap()
                .checked_mul_floor(ustars.params.max_loan_to_value)
                .unwrap(),
        )
        .add(
            borrowed_amount_juno
                .checked_mul_floor(ujuno.price)
                .unwrap()
                .checked_mul_floor(ujuno.params.max_loan_to_value)
                .unwrap(),
        );
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_ratio(max_ltv_adjusted_assets_value, total_debt_value))
    );
}

#[test]
fn above_max_ltv_below_liq_threshold() {
    let umars = umars_info();
    let udai = udai_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (umars.denom.clone(), umars.price),
            (udai.denom.clone(), udai.price),
        ]),
        params: HashMap::from([
            (umars.denom.clone(), umars.params.clone()),
            (udai.denom.clone(), udai.params.clone()),
        ]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.denom), coin(33, &udai.denom)],
            debts: vec![DebtAmount {
                denom: udai.denom,
                shares: Default::default(),
                amount: Uint128::new(3100),
            }],
            lends: vec![],
            vaults: vec![],
        },
        denoms_data,
        vaults_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(1210));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(968));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(1017));
    assert_eq!(health.total_debt_value, Uint128::new(972));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("0.99588477366255144").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("1.046296296296296296").unwrap())
    );
    assert!(health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

#[test]
fn liquidatable() {
    let umars = umars_info();
    let udai = udai_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (umars.denom.clone(), umars.price),
            (udai.denom.clone(), udai.price),
        ]),
        params: HashMap::from([
            (umars.denom.clone(), umars.params.clone()),
            (udai.denom.clone(), udai.params.clone()),
        ]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.denom), coin(33, &udai.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.denom,
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: umars.denom,
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![],
        },
        denoms_data,
        vaults_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(1210));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(968));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(1017));
    assert_eq!(health.total_debt_value, Uint128::new(1172));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("0.825938566552901023").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("0.867747440273037542").unwrap())
    );
    assert!(health.is_above_max_ltv());
    assert!(health.is_liquidatable());
}

#[test]
fn rover_whitelist_influences_max_ltv() {
    let umars = umars_info();
    let mut udai = udai_info();

    udai.params.credit_manager.whitelisted = false;

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (umars.denom.clone(), umars.price),
            (udai.denom.clone(), udai.price),
        ]),
        params: HashMap::from([
            (umars.denom.clone(), umars.params.clone()),
            (udai.denom.clone(), udai.params.clone()),
        ]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.denom), coin(33, &udai.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.denom,
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: umars.denom,
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![],
        },
        denoms_data,
        vaults_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(1210));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(960));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(1017));
    assert_eq!(health.total_debt_value, Uint128::new(1172));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("0.819112627986348122").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("0.867747440273037542").unwrap())
    );
    assert!(health.is_above_max_ltv());
    assert!(health.is_liquidatable());
}

#[test]
fn unlocked_vault() {
    let umars = umars_info();
    let udai = udai_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (umars.denom.clone(), umars.price),
            (udai.denom.clone(), udai.price),
        ]),
        params: HashMap::from([
            (umars.denom.clone(), umars.params.clone()),
            (udai.denom.clone(), udai.params.clone()),
        ]),
    };

    let vault = Vault::new(Addr::unchecked("vault_addr_123".to_string()));

    let vaults_data = VaultsData {
        vault_values: HashMap::from([(
            vault.address.clone(),
            VaultPositionValue {
                vault_coin: CoinValue {
                    denom: "leverage_vault_123".to_string(),
                    amount: Default::default(),
                    value: Uint128::new(5264),
                },
                base_coin: CoinValue {
                    denom: udai.denom.clone(),
                    amount: Default::default(),
                    value: Default::default(),
                },
            },
        )]),
        vault_configs: HashMap::from([(
            vault.address.clone(),
            VaultConfig {
                addr: vault.address.clone(),
                deposit_cap: Default::default(),
                max_loan_to_value: Decimal::from_str("0.4").unwrap(),
                liquidation_threshold: Decimal::from_str("0.5").unwrap(),
                whitelisted: true,
                hls: None,
            },
        )]),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.denom), coin(33, &udai.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.denom,
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: umars.denom,
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault,
                amount: VaultPositionAmount::Unlocked(VaultAmount::new(Uint128::new(5264))),
            }],
        },
        denoms_data,
        vaults_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(6474));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(3073));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(3649));
    assert_eq!(health.total_debt_value, Uint128::new(1172));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("2.622013651877133105").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("3.113481228668941979").unwrap())
    );
    assert!(!health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

#[test]
fn locked_vault() {
    let umars = umars_info();
    let udai = udai_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (umars.denom.clone(), umars.price),
            (udai.denom.clone(), udai.price),
        ]),
        params: HashMap::from([
            (umars.denom.clone(), umars.params.clone()),
            (udai.denom.clone(), udai.params.clone()),
        ]),
    };

    let vault = Vault::new(Addr::unchecked("vault_addr_123".to_string()));

    let vaults_data = VaultsData {
        vault_values: HashMap::from([(
            vault.address.clone(),
            VaultPositionValue {
                vault_coin: CoinValue {
                    denom: "leverage_vault_123".to_string(),
                    amount: Default::default(),
                    value: Uint128::new(5264),
                },
                base_coin: CoinValue {
                    denom: udai.denom.clone(),
                    amount: Default::default(),
                    value: Default::default(),
                },
            },
        )]),
        vault_configs: HashMap::from([(
            vault.address.clone(),
            VaultConfig {
                addr: vault.address.clone(),
                deposit_cap: Default::default(),
                max_loan_to_value: Decimal::from_str("0.4").unwrap(),
                liquidation_threshold: Decimal::from_str("0.5").unwrap(),
                whitelisted: true,
                hls: None,
            },
        )]),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.denom), coin(33, &udai.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.denom,
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: umars.denom,
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault,
                amount: VaultPositionAmount::Locking(LockingVaultAmount {
                    locked: VaultAmount::new(Uint128::new(42451613)),
                    unlocking: UnlockingPositions::new(vec![]),
                }),
            }],
        },
        denoms_data,
        vaults_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(6474));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(3073));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(3649));
    assert_eq!(health.total_debt_value, Uint128::new(1172));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("2.622013651877133105").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("3.113481228668941979").unwrap())
    );
    assert!(!health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

#[test]
fn locked_vault_with_unlocking_positions() {
    let umars = umars_info();
    let udai = udai_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (umars.denom.clone(), umars.price),
            (udai.denom.clone(), udai.price),
        ]),
        params: HashMap::from([
            (umars.denom.clone(), umars.params.clone()),
            (udai.denom.clone(), udai.params.clone()),
        ]),
    };

    let vault = Vault::new(Addr::unchecked("vault_addr_123".to_string()));

    let vaults_data = VaultsData {
        vault_values: HashMap::from([(
            vault.address.clone(),
            VaultPositionValue {
                vault_coin: CoinValue {
                    denom: "leverage_vault_123".to_string(),
                    amount: Default::default(),
                    value: Uint128::new(5000),
                },
                base_coin: CoinValue {
                    denom: udai.denom.clone(),
                    amount: Default::default(),
                    value: Uint128::new(264),
                },
            },
        )]),
        vault_configs: HashMap::from([(
            vault.address.clone(),
            VaultConfig {
                addr: vault.address.clone(),
                deposit_cap: Default::default(),
                max_loan_to_value: Decimal::from_str("0.4").unwrap(),
                liquidation_threshold: Decimal::from_str("0.5").unwrap(),
                whitelisted: true,
                hls: None,
            },
        )]),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.denom), coin(33, &udai.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.denom.clone(),
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: umars.denom,
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault,
                amount: VaultPositionAmount::Locking(LockingVaultAmount {
                    locked: VaultAmount::new(Uint128::new(40330000)),
                    unlocking: UnlockingPositions::new(vec![
                        VaultUnlockingPosition {
                            id: 0,
                            coin: coin(840, udai.denom.clone()),
                        },
                        VaultUnlockingPosition {
                            id: 1,
                            coin: coin(3, udai.denom),
                        },
                    ]),
                }),
            }],
        },
        denoms_data,
        vaults_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(6474));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(3192));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(3754));
    assert_eq!(health.total_debt_value, Uint128::new(1172));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("2.723549488054607508").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("3.203071672354948805").unwrap())
    );
    assert!(!health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

#[test]
fn vault_is_not_whitelisted() {
    let umars = umars_info();
    let udai = udai_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (umars.denom.clone(), umars.price),
            (udai.denom.clone(), udai.price),
        ]),
        params: HashMap::from([
            (umars.denom.clone(), umars.params.clone()),
            (udai.denom.clone(), udai.params.clone()),
        ]),
    };

    let vault = Vault::new(Addr::unchecked("vault_addr_123".to_string()));

    let vaults_data = VaultsData {
        vault_values: HashMap::from([(
            vault.address.clone(),
            VaultPositionValue {
                vault_coin: CoinValue {
                    denom: "leverage_vault_123".to_string(),
                    amount: Default::default(),
                    value: Uint128::new(5264),
                },
                base_coin: CoinValue {
                    denom: udai.denom.clone(),
                    amount: Default::default(),
                    value: Default::default(),
                },
            },
        )]),
        vault_configs: HashMap::from([(
            vault.address.clone(),
            VaultConfig {
                addr: vault.address.clone(),
                deposit_cap: Default::default(),
                max_loan_to_value: Decimal::from_str("0.4").unwrap(),
                liquidation_threshold: Decimal::from_str("0.5").unwrap(),
                whitelisted: false,
                hls: None,
            },
        )]),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.denom), coin(33, &udai.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.denom,
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: umars.denom,
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault,
                amount: VaultPositionAmount::Unlocked(VaultAmount::new(Uint128::new(5264))),
            }],
        },
        denoms_data,
        vaults_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(6474));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(968));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(3649));
    assert_eq!(health.total_debt_value, Uint128::new(1172));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("0.825938566552901023").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("3.113481228668941979").unwrap())
    );
    assert!(health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

/// Delisting base token will make even vault token maxLTV to drop
#[test]
fn vault_base_token_is_not_whitelisted() {
    let umars = umars_info();
    let udai = udai_info();
    let mut ujuno = ujuno_info();

    ujuno.params.credit_manager.whitelisted = false;

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (umars.denom.clone(), umars.price),
            (udai.denom.clone(), udai.price),
            (ujuno.denom.clone(), ujuno.price),
        ]),
        params: HashMap::from([
            (umars.denom.clone(), umars.params.clone()),
            (udai.denom.clone(), udai.params.clone()),
            (ujuno.denom.clone(), ujuno.params.clone()),
        ]),
    };

    let vault = Vault::new(Addr::unchecked("vault_addr_123".to_string()));

    let vaults_data = VaultsData {
        vault_values: HashMap::from([(
            vault.address.clone(),
            VaultPositionValue {
                vault_coin: CoinValue {
                    denom: "leverage_vault_123".to_string(),
                    amount: Uint128::new(40330000),
                    value: Uint128::new(5000),
                },
                base_coin: CoinValue {
                    denom: ujuno.denom.clone(),
                    amount: Uint128::new(71),
                    value: Uint128::new(497873442),
                },
            },
        )]),
        vault_configs: HashMap::from([(
            vault.address.clone(),
            VaultConfig {
                addr: vault.address.clone(),
                deposit_cap: Default::default(),
                max_loan_to_value: Decimal::from_str("0.4").unwrap(),
                liquidation_threshold: Decimal::from_str("0.5").unwrap(),
                whitelisted: true,
                hls: None,
            },
        )]),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.denom), coin(33, &udai.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.denom,
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: umars.denom,
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault,
                amount: VaultPositionAmount::Locking(LockingVaultAmount {
                    locked: VaultAmount::new(Uint128::new(40330000)),
                    unlocking: UnlockingPositions::new(vec![
                        VaultUnlockingPosition {
                            id: 0,
                            coin: coin(60, ujuno.denom.clone()),
                        },
                        VaultUnlockingPosition {
                            id: 1,
                            coin: coin(11, ujuno.denom),
                        },
                    ]),
                }),
            }],
        },
        denoms_data,
        vaults_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(497879652));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(968)); // Lower due to vault blacklisted
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(448089614));
    assert_eq!(health.total_debt_value, Uint128::new(1172));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("0.825938566552901023").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("382329.022184300341296928").unwrap())
    );
    assert!(health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

#[test]
fn lent_coins_used_as_collateral() {
    let umars = umars_info();
    let udai = udai_info();
    let uluna = uluna_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (umars.denom.clone(), umars.price),
            (udai.denom.clone(), udai.price),
            (uluna.denom.clone(), uluna.price),
        ]),
        params: HashMap::from([
            (umars.denom.clone(), umars.params.clone()),
            (udai.denom.clone(), udai.params.clone()),
            (uluna.denom.clone(), uluna.params.clone()),
        ]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.denom), coin(23, &udai.denom)],
            debts: vec![DebtAmount {
                denom: udai.denom.clone(),
                shares: Default::default(),
                amount: Uint128::new(3100),
            }],
            lends: vec![
                LentAmount {
                    denom: udai.denom,
                    shares: Default::default(),
                    amount: Uint128::new(10),
                },
                LentAmount {
                    denom: uluna.denom,
                    shares: Default::default(),
                    amount: Uint128::new(2),
                },
            ],
            vaults: vec![],
        },
        denoms_data,
        vaults_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(1230));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(981));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(1031));
    assert_eq!(health.total_debt_value, Uint128::new(972));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("1.009259259259259259").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("1.060699588477366255").unwrap())
    );
    assert!(!health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

#[test]
fn allowed_lent_coins_influence_max_ltv() {
    let umars = umars_info();
    let udai = udai_info();
    let mut uluna = uluna_info();

    uluna.params.credit_manager.whitelisted = false;

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (umars.denom.clone(), umars.price),
            (udai.denom.clone(), udai.price),
            (uluna.denom.clone(), uluna.price),
        ]),
        params: HashMap::from([
            (umars.denom.clone(), umars.params.clone()),
            (udai.denom.clone(), udai.params.clone()),
            (uluna.denom.clone(), uluna.params.clone()),
        ]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.denom), coin(23, &udai.denom)],
            debts: vec![DebtAmount {
                denom: udai.denom.clone(),
                shares: Default::default(),
                amount: Uint128::new(3100),
            }],
            lends: vec![
                LentAmount {
                    denom: udai.denom,
                    shares: Default::default(),
                    amount: Uint128::new(10),
                },
                LentAmount {
                    denom: uluna.denom,
                    shares: Default::default(),
                    amount: Uint128::new(2),
                },
            ],
            vaults: vec![],
        },
        denoms_data,
        vaults_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(1230));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(967));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(1031));
    assert_eq!(health.total_debt_value, Uint128::new(972));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("0.9948559670781893").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("1.060699588477366255").unwrap())
    );
    assert!(health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}
